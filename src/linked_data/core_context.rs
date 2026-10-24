//! The bundled NGSI-LD core context (NGSI-LD clause 4.4), and the loader that serves it.

use crate::{
    linked_data::{JsonLdError, RemoteContextLoader},
    model::{CORE_CONTEXT, InvalidJson, RawJson, keyword::CONTEXT},
};
use iri_rs::{Iri, IriBuf, iri};
use jsonld::{
    LoadError,
    Loader,
    RemoteDocument,
    syntax::{Context as SyntaxContext, TryFromJson},
};
use jstrict::{Value as Json, parse::parse_str_value};
use std::future;

/// The NGSI-LD core context v1.9, as ETSI publishes it at [`CORE_CONTEXT`].
const CORE_CONTEXT_DOCUMENT: &str = include_str!("core_context/ngsi-ld-core-context-v1.9.jsonld");

/// The URL the core context's `ngsildproof` term imports.
const DATA_INTEGRITY_CONTEXT: Iri<&'static str> = iri!("https://w3id.org/security/data-integrity/v2");

/// The W3C Data Integrity v2 context, as published at [`DATA_INTEGRITY_CONTEXT`].
const DATA_INTEGRITY_DOCUMENT: &str = include_str!("core_context/data-integrity-v2.jsonld");

/// Why a bundled context document could not be read.
///
/// The documents are fixed at build time and a test reads them, so this is a defect of the build, never of the
/// caller's input.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BundledContextError {
    /// A bundled document is not JSON.
    #[error("a bundled context document is not JSON")]
    Json(#[source] InvalidJson),
    /// The core context document holds no `@context` member.
    #[error("the bundled core context document holds no @context")]
    MissingContext,
    /// The core context's `@context` member is not a JSON-LD context.
    #[error("the bundled core context is not a JSON-LD context")]
    Context(#[source] JsonLdError),
}

/// A remote context was referenced where nothing may be loaded: an active context processed offline names a
/// remote context other than the core context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("the remote context needs a loader")]
pub struct RequiresLoader;

/// The loader of offline processing, which loads nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OfflineLoader;

impl RemoteContextLoader for OfflineLoader {
    type Error = RequiresLoader;

    fn load(&self, _: Iri<&str>) -> impl Future<Output = Result<RawJson, Self::Error>> + Send {
        future::ready(Err(RequiresLoader))
    }
}

/// The bundled documents, read once by a context processor and shared by every active context it builds.
#[derive(Debug)]
pub(crate) struct BundledContexts {
    /// The core context document.
    core: Json,
    /// The Data Integrity v2 context document.
    data_integrity: Json,
    /// The core context's `@context`, as the context syntax the JSON-LD processor processes.
    core_context: SyntaxContext,
}

impl BundledContexts {
    /// Reads the bundled documents.
    ///
    /// # Errors
    ///
    /// Returns [`BundledContextError::Json`] if a bundled document is not JSON, and the other variants if the core
    /// context document holds no valid `@context`.
    pub(crate) fn read() -> Result<Self, BundledContextError> {
        let core = parse_str_value(CORE_CONTEXT_DOCUMENT).map_err(|error| BundledContextError::Json(InvalidJson::of(&error)))?;
        let data_integrity = parse_str_value(DATA_INTEGRITY_DOCUMENT).map_err(|error| BundledContextError::Json(InvalidJson::of(&error)))?;
        let context = core
            .as_object()
            .and_then(|document| document.get_unique(CONTEXT).ok().flatten())
            .ok_or(BundledContextError::MissingContext)?;
        let core_context = SyntaxContext::try_from_json(context).map_err(|error| BundledContextError::Context(JsonLdError::syntax(error)))?;
        Ok(Self {
            core,
            data_integrity,
            core_context,
        })
    }

    /// The core context, as the context syntax the JSON-LD processor processes.
    pub(crate) const fn core_context(&self) -> &SyntaxContext {
        &self.core_context
    }

    /// The bundled document behind `url`, if one is.
    fn document(&self, url: Iri<&str>) -> Option<&Json> {
        if url == CORE_CONTEXT {
            Some(&self.core)
        } else if url == DATA_INTEGRITY_CONTEXT {
            Some(&self.data_integrity)
        } else {
            None
        }
    }
}

/// A loader that answers the bundled documents from memory and hands every other URL to `L`: the only bridge between
/// a [`RemoteContextLoader`] and the JSON-LD processor's loader.
#[derive(Debug)]
pub(crate) struct CoreContextLoader<'l, L> {
    /// The bundled documents.
    bundled: &'l BundledContexts,
    /// The loader every other URL goes to.
    inner: &'l L,
}

impl<'l, L> CoreContextLoader<'l, L> {
    /// A loader that answers `bundled` and hands every other URL to `inner`.
    pub(crate) const fn new(bundled: &'l BundledContexts, inner: &'l L) -> Self {
        Self { bundled, inner }
    }
}

impl<L: RemoteContextLoader> Loader for CoreContextLoader<'_, L> {
    type Error = L::Error;

    async fn load(&self, url: Iri<&str>) -> Result<RemoteDocument<IriBuf>, LoadError<Self::Error>> {
        let document = match self.bundled.document(url) {
            // Clone: a loaded document is handed over by value, once per context that references it.
            Some(document) => document.clone(),
            None => RemoteContextLoader::load(self.inner, url)
                .await
                .map_err(|error| LoadError::new(url.into(), error))?
                .into_json(),
        };
        Ok(RemoteDocument::new(Some(url.into()), None, document))
    }
}

#[cfg(test)]
mod tests;
