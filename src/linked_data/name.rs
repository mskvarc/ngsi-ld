//! Names under an active context: expansion to a Fully Qualified Name, compaction back, and translation from one
//! context to another (NGSI-LD clause 5.5.7).

use crate::{
    linked_data::ActiveContext,
    model::{AttributeName, EntityTypeName, NameError, Term, Uri, VocabTerm},
};
use iri_rs::{InvalidIri, IriBuf};
use jsonld::{
    Id,
    Term as LdTerm,
    ValidId,
    compaction::{IriConfusedWithPrefix, Options as CompactionOptions, compact_iri},
    context_processing::algorithm::{Action, expand_property_key, expand_type_value},
    rdfx::vocabulary::{no_vocabulary, no_vocabulary_mut},
};
use std::sync::Arc;

/// Why a name did not expand to a Fully Qualified Name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ExpansionError<N> {
    /// The active context neither defines the name nor maps it through a vocabulary, a prefix or an IRI.
    #[error("`{term}` expands to no IRI under the active context")]
    Unresolved {
        /// The name as given.
        term: N,
    },
    /// The name expands to a blank node identifier, which is no Fully Qualified Name.
    #[error("`{term}` expands to a blank node identifier")]
    BlankNode {
        /// The name as given.
        term: N,
    },
    /// The name is an alias of a JSON-LD keyword.
    #[error("`{term}` is an alias of a JSON-LD keyword")]
    Keyword {
        /// The name as given.
        term: N,
    },
}

/// Why a Fully Qualified Name did not compact to a name.
// `InvalidIri` carries no `Clone` or equality, so neither can the error that chains it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompactionError {
    /// The name kind cannot hold what the IRI compacts to: a query-language [`Term`] holds neither a prefix nor
    /// an IRI (clause 4.9), so an IRI with no unprefixed term in the active context cannot be written as one.
    #[error("`{iri}` has no unprefixed term under the active context")]
    NotExpressible {
        /// The Fully Qualified Name.
        iri: Uri,
    },
    /// What the IRI compacts to is not a valid name of the kind asked for.
    #[error("the compacted form of `{iri}` is not a valid name")]
    Name {
        /// The Fully Qualified Name.
        iri: Uri,
        /// Why the compacted form is not a name.
        #[source]
        source: NameError,
    },
    /// Written out in full, the IRI would be read back as a compact IRI (JSON-LD 1.1, IRI compaction step 9).
    #[error("`{iri}` would be read back as a compact IRI")]
    ConfusedWithPrefix {
        /// The Fully Qualified Name.
        iri: Uri,
    },
    /// The URI is not an IRI the JSON-LD processor accepts.
    #[error("`{iri}` is not an IRI")]
    NotAnIri {
        /// The Fully Qualified Name.
        iri: Uri,
        /// Why the processor refused it.
        #[source]
        source: InvalidIri<String>,
    },
}

/// Why a name did not translate from one active context to another.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TranslationError<N> {
    /// The name does not expand under the context it is written in.
    #[error(transparent)]
    Expansion(#[from] ExpansionError<N>),
    /// Its Fully Qualified Name does not compact under the target context.
    #[error(transparent)]
    Compaction(#[from] CompactionError),
}

/// Names a JSON-LD processor reads through the `@context`.
mod sealed {
    use crate::model::NameError;

    /// How JSON-LD reads a name: as a key of a node object, or as a value coerced through `@vocab`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NamePosition {
        /// An attribute name, read as a property key.
        Key,
        /// An entity type or a vocabulary term, read as an `@type` value or an `@vocab`-coerced value.
        Value,
    }

    /// Whether a name kind may hold a compact IRI or an IRI.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Prefixing {
        /// The name may be a term, a compact IRI or an IRI.
        Allowed,
        /// The name is a bare term (clause 4.9).
        Forbidden,
    }

    /// The hooks every [`LinkedName`](super::LinkedName) provides.
    pub trait Sealed: Sized {
        /// How JSON-LD reads the name.
        const POSITION: NamePosition;

        /// Whether the name may hold a compact IRI or an IRI.
        const PREFIXING: Prefixing;

        /// The name as written.
        fn spelling(&self) -> &str;

        /// Reads a name from its compacted form.
        fn from_spelling(spelling: &str) -> Result<Self, NameError>;
    }
}

use sealed::{NamePosition, Prefixing};

/// A name an [`ActiveContext`] expands, compacts and translates: [`EntityTypeName`], [`AttributeName`],
/// [`VocabTerm`] (the `vocab` of a `VocabProperty`, clause 4.5.20) and [`Term`] (the bare names of the query
/// languages, clause 4.9).
///
/// The trait is sealed: the set of names is the one NGSI-LD defines.
///
/// ```
/// use ngsi_ld::{
///     linked_data::ContextProcessor,
///     model::{Context, ContextEntry, EntityTypeName, InlineContext},
/// };
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let a = Context::Single(ContextEntry::Inline(
///     "{\"Vehicle\": \"https://example.org/Vehicle\"}".parse::<InlineContext>()?,
/// ));
/// let b = Context::Single(ContextEntry::Inline(
///     "{\"Car\": \"https://example.org/Vehicle\"}".parse::<InlineContext>()?,
/// ));
/// let processor = ContextProcessor::new()?;
/// let a = processor.process_offline(&a).await?;
/// let b = processor.process_offline(&b).await?;
/// let vehicle: EntityTypeName = "Vehicle".parse()?;
/// assert_eq!(a.translate(&vehicle, &b)?.to_string(), "Car");
/// # Ok(())
/// # }
/// ```
pub trait LinkedName: sealed::Sealed {}

impl<N: sealed::Sealed> LinkedName for N {}

/// Implements [`sealed::Sealed`] for a name newtype.
macro_rules! linked_name {
    ($name:ty, $position:expr, $prefixing:expr) => {
        impl sealed::Sealed for $name {
            const POSITION: NamePosition = $position;
            const PREFIXING: Prefixing = $prefixing;

            fn spelling(&self) -> &str {
                self.as_ref()
            }

            fn from_spelling(spelling: &str) -> Result<Self, NameError> {
                spelling.parse()
            }
        }
    };
}

linked_name!(EntityTypeName, NamePosition::Value, Prefixing::Allowed);
linked_name!(AttributeName, NamePosition::Key, Prefixing::Allowed);
linked_name!(VocabTerm, NamePosition::Value, Prefixing::Allowed);
linked_name!(Term, NamePosition::Key, Prefixing::Forbidden);

impl ActiveContext {
    /// Expands `name` to its Fully Qualified Name, as JSON-LD expansion reads it under this context: an
    /// attribute name as a property key, an entity type as an `@type` value, a vocabulary term as an
    /// `@vocab`-coerced value (clause 5.5.7).
    ///
    /// # Errors
    ///
    /// Returns [`ExpansionError`] when the name expands to no IRI, to a blank node identifier or to a keyword.
    pub fn expand<N: LinkedName + Clone>(&self, name: &N) -> Result<Uri, ExpansionError<N>> {
        let spelling = name.spelling();
        let expand = match N::POSITION {
            NamePosition::Key => expand_property_key,
            NamePosition::Value => expand_type_value,
        };
        // `Action::Keep` accepts what `@vocab` expands to, so `RejectVocab` is reached only by a policy this call
        // does not use; it would leave the name unresolved.
        let term = expand(no_vocabulary_mut(), self.processed(), spelling, Action::Keep).ok().flatten();
        let error: fn(N) -> ExpansionError<N> = match term.as_deref() {
            Some(LdTerm::Id(Id::Valid(ValidId::Iri(iri)))) => return Ok(Uri::from(iri.as_ref())),
            Some(LdTerm::Id(Id::Valid(ValidId::Blank(_)))) => |term| ExpansionError::BlankNode { term },
            Some(LdTerm::Keyword(_)) => |term| ExpansionError::Keyword { term },
            Some(LdTerm::Id(Id::Invalid(_)) | LdTerm::Null) | None => |term| ExpansionError::Unresolved { term },
        };
        // Clone: the caller keeps the name it expands, and the error owns one.
        Err(error(name.clone()))
    }

    /// Compacts the Fully Qualified Name `iri` to the name of kind `N` this context writes it as, falling back to
    /// the Fully Qualified Name when no term matches (clause 5.5.7).
    ///
    /// # Errors
    ///
    /// Returns [`CompactionError::NotExpressible`] when `N` is a [`Term`] and the context has no unprefixed term
    /// for `iri`, [`CompactionError::Name`] when the compacted form is no valid name of kind `N`, and
    /// [`CompactionError::ConfusedWithPrefix`] when the IRI would be read back as a compact IRI.
    pub fn compact<N: LinkedName>(&self, iri: &Uri) -> Result<N, CompactionError> {
        // Clone, in each error below: the caller keeps the IRI it compacts, and the error owns one.
        let id = IriBuf::new(iri.to_string()).map_err(|source| CompactionError::NotAnIri { iri: iri.clone(), source })?;
        let term = LdTerm::Id(Id::iri(id));
        let compacted: Option<Arc<str>> = compact_iri(no_vocabulary(), self.processed(), &term, true, false, CompactionOptions::default())
            .map_err(|IriConfusedWithPrefix| CompactionError::ConfusedWithPrefix { iri: iri.clone() })?;
        let compacted = compacted.ok_or_else(|| CompactionError::NotExpressible { iri: iri.clone() })?;
        N::from_spelling(&compacted).map_err(|source| match N::PREFIXING {
            Prefixing::Forbidden => CompactionError::NotExpressible { iri: iri.clone() },
            Prefixing::Allowed => CompactionError::Name { iri: iri.clone(), source },
        })
    }

    /// Translates `name`, written under this context, into the name `to` writes the same Fully Qualified Name
    /// as.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError::Expansion`] when `name` does not expand under this context, and
    /// [`TranslationError::Compaction`] when its Fully Qualified Name does not compact under `to`.
    pub fn translate<N: LinkedName + Clone>(&self, name: &N, to: &Self) -> Result<N, TranslationError<N>> {
        let iri = self.expand(name)?;
        Ok(to.compact(&iri)?)
    }
}

#[cfg(test)]
mod tests;
