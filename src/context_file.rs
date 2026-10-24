//! `@context`s kept in local files: a JSON-LD document `{"@context": ...}` read from the file system, made
//! self-contained so a broker can use it.
//!
//! Reading is synchronous, with `std::fs`, so the crate depends on no async runtime and never blocks behind an
//! `async` signature. A context file is small and is usually read once, when a client is set up; a caller on an
//! executor that must not block, reading from a slow file system, runs [`load`] on its blocking pool, for instance
//! with `tokio::task::spawn_blocking`.
//!
//! A broker cannot dereference a path on the client's disk, so every reference in the file that resolves to a
//! local file (a relative reference, resolved against the file it is written in as JSON-LD 1.1 section 4.1.7
//! requires, or a `file:` IRI) is loaded in turn and its `@context` entries take the reference's place. A reference
//! that resolves to another scheme, such as `https:`, names a document the broker loads itself and is kept. The
//! result is an [`Context`] of absolute references and inline definitions (NGSI-LD clause 5.2.3, Table 5.2.3-1).
//!
//! References nested inside an inline definition (the `@context` of a scoped term definition) are kept as written.
//!
//! # Loading
//!
//! [`load`], [`ContextFileError`].

use crate::model::{Context, ContextEntry, DocumentError, LocalContext, nesting::grow};
use iri_rs::{InvalidIri, IriRefBuf};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use url::{ParseError, Url};

/// Why a context file could not be read into a self-contained [`Context`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContextFileError {
    /// The file could not be found or read.
    #[error("the context file {} could not be read", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// Why.
        #[source]
        source: io::Error,
    },
    /// The file is not a JSON-LD document holding an `@context` (Table 6.29.3.1-1).
    #[error("the context file {} is not a document holding an @context", path.display())]
    Document {
        /// The file.
        path: PathBuf,
        /// Why, boxed to keep the error small.
        #[source]
        source: Box<DocumentError>,
    },
    /// A reference in the file cannot be resolved against the file's location.
    #[error("the reference {reference} in the context file {} cannot be resolved", path.display())]
    Reference {
        /// The file holding the reference.
        path: PathBuf,
        /// The reference, as written.
        reference: IriRefBuf,
        /// Why.
        #[source]
        source: ParseError,
    },
    /// A reference resolves to a `file:` URL that names no path on this platform.
    #[error("the reference {reference} in the context file {} names no local path", path.display())]
    NotALocalPath {
        /// The file holding the reference.
        path: PathBuf,
        /// The reference, as written.
        reference: IriRefBuf,
    },
    /// A resolved reference is not an IRI.
    #[error("a reference in the context file {} does not resolve to an IRI", path.display())]
    Resolved {
        /// The file holding the reference.
        path: PathBuf,
        /// Why.
        #[source]
        source: InvalidIri<String>,
    },
    /// A path has no `file:` URL to resolve the references of the file against.
    #[error("the context file {} has no file URL", path.display())]
    NotAFileUrl {
        /// The file.
        path: PathBuf,
    },
    /// A file references itself, directly or through other files.
    #[error("the context file {} references itself", path.display())]
    Cycle {
        /// The file referenced again.
        path: PathBuf,
    },
}

/// Reads the JSON-LD document at `path`, `{"@context": ...}`, and returns its `@context` with every reference to a
/// local file replaced by the entries of that file, recursively.
///
/// ```no_run
/// use ngsi_ld::context_file;
///
/// let fleet = context_file::load("contexts/fleet.jsonld")?;
/// assert!(fleet.references().all(|reference| reference.is_absolute()));
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`ContextFileError`] when a file cannot be read, is not a document holding an `@context`, holds a
/// reference that cannot be resolved, or references itself.
pub fn load(path: impl AsRef<Path>) -> Result<Context, ContextFileError> {
    let entries = load_entries(path.as_ref().to_path_buf(), Vec::new())?;
    // A file whose context is one entry keeps that form, so a single reference still travels in a `Link` header.
    Ok(match <[ContextEntry; 1]>::try_from(entries) {
        Ok([entry]) => Context::Single(entry),
        Err(entries) => Context::Multiple(entries),
    })
}

/// The entries of the file at `path`, self-contained; `visiting` holds the files whose loading led here.
fn load_entries(path: PathBuf, mut visiting: Vec<PathBuf>) -> Result<Vec<ContextEntry>, ContextFileError> {
    let file = fs::canonicalize(&path).map_err(|source| ContextFileError::Read { path, source })?;
    if visiting.contains(&file) {
        return Err(ContextFileError::Cycle { path: file });
    }
    let text = fs::read_to_string(&file).map_err(|source| ContextFileError::Read {
        // The error owns the path, and the loading keeps it to resolve references against.
        path: file.clone(),
        source,
    })?;
    let document = LocalContext::parse(&text).map_err(|source| ContextFileError::Document {
        // As above.
        path: file.clone(),
        source: Box::new(source),
    })?;
    let Ok(base) = Url::from_file_path(&file) else {
        return Err(ContextFileError::NotAFileUrl { path: file });
    };
    // The files referenced from here must not lead back to this one; the loading keeps its own path to name the
    // file in an error.
    visiting.push(file.clone());
    let mut entries = Vec::new();
    for entry in document.context.entries() {
        match entry {
            // Each referenced file is loaded with its own list of the files that led to it.
            ContextEntry::Reference(reference) => entries.extend(resolve(&base, &file, reference, visiting.clone())?),
            // The context owns its entries; the document is dropped once they are taken.
            ContextEntry::Inline(inline) => entries.push(ContextEntry::Inline(inline.clone())),
        }
    }
    Ok(entries)
}

/// The entries `reference`, written in `file` (whose URL is `base`), stands for: those of the file it names, or the
/// reference itself, made absolute, when it names a document elsewhere. `visiting` holds the files whose loading led
/// here.
fn resolve(base: &Url, file: &Path, reference: &IriRefBuf, visiting: Vec<PathBuf>) -> Result<Vec<ContextEntry>, ContextFileError> {
    let resolved = base.join(reference.as_str()).map_err(|source| ContextFileError::Reference {
        path: file.to_path_buf(),
        // The error owns the reference as written.
        reference: reference.clone(),
        source,
    })?;
    if resolved.scheme() == "file" {
        let referenced = resolved.to_file_path().map_err(|()| ContextFileError::NotALocalPath {
            path: file.to_path_buf(),
            // As above.
            reference: reference.clone(),
        })?;
        // A chain of files nests the loading one level per file, however long the chain.
        return grow(|| load_entries(referenced, visiting));
    }
    let absolute = IriRefBuf::new(String::from(resolved)).map_err(|source| ContextFileError::Resolved {
        path: file.to_path_buf(),
        source,
    })?;
    Ok(vec![ContextEntry::Reference(absolute)])
}

#[cfg(test)]
mod tests;
