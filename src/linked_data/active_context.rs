//! Active contexts: a document's `@context`, processed, with the NGSI-LD core context applied last (NGSI-LD
//! clauses 4.4 and 5.5.7).

use crate::{
    linked_data::{
        JsonLdError,
        RemoteContextLoader,
        core_context::{BundledContexts, CoreContextLoader},
    },
    model::Context,
};
use iri_rs::IriBuf;
use jsonld::{
    BlankIdBuf,
    LoadError,
    Process,
    context::BindingRef,
    context_processing::{Error as ContextProcessingError, Options},
    rdfx::vocabulary::no_vocabulary_mut,
    syntax::{Context as SyntaxContext, ContextEntry, TryFromJson, context::definition::Key},
    warning::WarningBuf,
};
use std::sync::Arc;

/// A term of a JSON-LD context.
///
/// It is any non-empty string a term definition names (JSON-LD 1.1 section 3.1), so not one of the crate's name types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct ContextTerm(String);

impl ContextTerm {
    /// Returns the term as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why an `@context` could not be processed.
///
/// `E` is the error of the [`RemoteContextLoader`] that loads its remote contexts.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProcessingError<E> {
    /// The `@context` is not a JSON-LD context.
    #[error("the @context is not a JSON-LD context")]
    Syntax(#[source] JsonLdError),
    /// A term definition of the user `@context` carries a scoped context, which NGSI-LD forbids.
    #[error("the term `{term}` of a user @context carries a scoped @context")]
    ScopedContext {
        /// The term whose definition carries the scoped context.
        term: ContextTerm,
    },
    /// A remote context could not be loaded.
    #[error("the remote context `{url}` could not be loaded")]
    Loading {
        /// The URL of the remote context.
        url: IriBuf,
        /// Why the loader could not load it.
        #[source]
        source: E,
    },
    /// JSON-LD context processing failed.
    #[error("the @context could not be processed")]
    Processing(#[source] JsonLdError),
}

/// Maps an error of the context processing algorithm to the crate's, naming every variant so a new one is a
/// decision.
fn processing_error<E>(error: ContextProcessingError<E>) -> ProcessingError<E> {
    match error {
        ContextProcessingError::ContextLoadingFailed(LoadError { target, source }) => ProcessingError::Loading { url: target, source },
        ContextProcessingError::ContextExtractionFailed(extraction) => ProcessingError::Processing(JsonLdError::extraction(extraction)),
        error @ (ContextProcessingError::InvalidContextNullification
        | ContextProcessingError::LoadingDocumentFailed
        | ContextProcessingError::ProcessingModeConflict
        | ContextProcessingError::InvalidContextEntry
        | ContextProcessingError::InvalidImportValue
        | ContextProcessingError::InvalidRemoteContext
        | ContextProcessingError::InvalidBaseIri
        | ContextProcessingError::InvalidVocabMapping
        | ContextProcessingError::CyclicIriMapping
        | ContextProcessingError::InvalidTermDefinition
        | ContextProcessingError::KeywordRedefinition
        | ContextProcessingError::InvalidProtectedValue
        | ContextProcessingError::InvalidTypeMapping
        | ContextProcessingError::InvalidReverseProperty
        | ContextProcessingError::InvalidIriMapping
        | ContextProcessingError::InvalidKeywordAlias
        | ContextProcessingError::InvalidContainerMapping
        | ContextProcessingError::InvalidScopedContext
        | ContextProcessingError::ProtectedTermRedefinition
        | ContextProcessingError::RecursiveContextInclusion
        | ContextProcessingError::ContextOverflow
        | ContextProcessingError::ForbiddenVocab) => ProcessingError::Processing(JsonLdError::processing(error.code())),
    }
}

/// A processed `@context`.
///
/// It is the user context a document carries, with the NGSI-LD core context processed on top.
///
/// NGSI-LD treats the core context as the last entry of every `@context`, protected and immutable (clause 4.4):
/// a user context cannot redefine a core term such as `location` and must prefix its own instead (clause C.9).
/// Building an active context therefore processes the user context first and the core context over it, so the
/// core definitions win wherever the user context sits. A user context must not carry scoped contexts or a
/// nested `@context` (clause 5.5.7); one that does is refused.
///
/// A [`ContextProcessor`] builds active contexts. The core context is bundled, so its [`core`] and
/// [`process_offline`] never reach the network; [`process`] hands every other remote context to a
/// [`RemoteContextLoader`].
///
/// ```
/// use ngsi_ld::{
///     linked_data::ContextProcessor,
///     model::{AttributeName, Uri},
/// };
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let core = ContextProcessor::try_new()?.core().await?;
/// let speed: AttributeName = "speed".parse()?;
/// let expanded: Uri = core.expand(&speed)?;
/// assert_eq!(
///     expanded.as_str(),
///     "https://uri.etsi.org/ngsi-ld/default-context/speed"
/// );
/// # Ok(())
/// # }
/// ```
///
/// [`ContextProcessor`]: crate::linked_data::ContextProcessor
/// [`core`]: crate::linked_data::ContextProcessor::core
/// [`process_offline`]: crate::linked_data::ContextProcessor::process_offline
/// [`process`]: crate::linked_data::ContextProcessor::process
#[derive(Clone, derive_more::Debug)]
pub struct ActiveContext {
    /// The user `@context` the active context was processed from; `None` for the core context alone.
    source: Option<Context>,
    /// The processed context, whose term definitions the source already shows.
    #[debug(skip)]
    processed: jsonld::Context<IriBuf, BlankIdBuf>,
    /// The bundled documents, which expansion and compaction load again for the core context's scoped
    /// contexts; shared with the processor that built the active context and every clone of it.
    #[debug(skip)]
    bundled: Arc<BundledContexts>,
}

impl ActiveContext {
    /// Returns the user `@context` this active context was processed from, or `None` for the core context
    /// alone.
    #[must_use]
    pub const fn source(&self) -> Option<&Context> {
        self.source.as_ref()
    }

    /// Returns the processed JSON-LD context.
    pub(crate) const fn processed(&self) -> &jsonld::Context<IriBuf, BlankIdBuf> {
        &self.processed
    }

    /// Returns the bundled documents.
    pub(crate) fn bundled(&self) -> &BundledContexts {
        &self.bundled
    }

    /// Processes `user`, when there is one, then the core context over it, loading remote contexts with `loader`
    /// and answering the bundled ones from `bundled`.
    pub(crate) async fn build<L: RemoteContextLoader>(
        user: Option<&Context>,
        loader: &L,
        bundled: Arc<BundledContexts>,
    ) -> Result<Self, ProcessingError<L::Error>> {
        let core = bundled.core_context();
        let loader = CoreContextLoader::new(&bundled, loader);
        // Warnings report terms JSON-LD drops; the active context is what remains, so they are not surfaced.
        let mut warnings = WarningBuf::new();
        let mut active = jsonld::Context::new(None);
        if let Some(user) = user {
            let syntax = SyntaxContext::try_from_json(&user.to_json()).map_err(|error| ProcessingError::Syntax(JsonLdError::syntax(error)))?;
            active = syntax
                .process_full(no_vocabulary_mut(), &active, &loader, None, Options::default(), &mut warnings)
                .await
                .map_err(processing_error)?
                .into_processed();
            refuse_scoped_contexts(&active, core)?;
        }
        // The core context is processed last and may replace a user definition, even a protected one: its
        // definitions take precedence wherever it sits (clauses 4.4 and C.9).
        let processed = core
            .process_full(no_vocabulary_mut(), &active, &loader, None, Options::default().with_override(), &mut warnings)
            .await
            .map_err(processing_error)?
            .into_processed();
        Ok(Self {
            // Clone: the active context keeps the user context it shows, while the caller keeps theirs.
            source: user.cloned(),
            processed,
            bundled,
        })
    }
}

/// Refuses a term definition of the user context that carries a scoped context (clause 5.5.7).
///
/// A user context may name the core context itself, whose `ngsildproof` term carries one; a term the core
/// context defines is replaced by the core definition anyway, so only the others are checked.
fn refuse_scoped_contexts<E>(active: &jsonld::Context<IriBuf, BlankIdBuf>, core: &SyntaxContext) -> Result<(), ProcessingError<E>> {
    let defined_by_core = |term: &Key| {
        core.iter().any(|entry| match entry {
            ContextEntry::Definition(definition) => definition.get_binding(term.as_str()).is_some(),
            ContextEntry::Null | ContextEntry::IriRef(_) => false,
        })
    };
    active
        .definitions()
        .into_iter()
        .find_map(|binding| match binding {
            BindingRef::Normal(term, definition) if definition.context.is_some() && !defined_by_core(term) => Some(term),
            BindingRef::Normal(..) | BindingRef::Type(_) => None,
        })
        .map_or(Ok(()), |term| {
            Err(ProcessingError::ScopedContext {
                term: ContextTerm(term.as_str().to_owned()),
            })
        })
}

#[cfg(test)]
mod tests;
