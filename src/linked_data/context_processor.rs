//! The context processor: the bundled core context, read once, and the active contexts built over it (NGSI-LD
//! clauses 4.4 and 5.5.7).

use crate::{
    linked_data::{
        ActiveContext,
        BundledContextError,
        ProcessingError,
        RemoteContextLoader,
        RequiresLoader,
        core_context::{BundledContexts, OfflineLoader},
    },
    model::Context,
};
use std::sync::Arc;

/// Builds [`ActiveContext`]s. It reads the bundled core context once, when it is made, and every active context it
/// builds shares that copy, so a caller that processes many contexts keeps one processor and parses the core
/// context once. The processor is the caller's: the crate holds no processor of its own.
///
/// ```
/// use ngsi_ld::{
///     linked_data::ContextProcessor,
///     model::{Context, ContextEntry, EntityTypeName, InlineContext},
/// };
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let processor = ContextProcessor::try_new()?;
/// let user = Context::Single(ContextEntry::Inline(
///     r#"{"Vehicle": "https://example.org/Vehicle"}"#.parse::<InlineContext>()?,
/// ));
/// let active = processor.process_offline(&user).await?;
/// let vehicle: EntityTypeName = "Vehicle".parse()?;
/// assert_eq!(
///     active.expand(&vehicle)?.as_str(),
///     "https://example.org/Vehicle"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Clone, derive_more::Debug)]
pub struct ContextProcessor {
    /// The bundled documents, shared with every active context the processor builds.
    #[debug(skip)]
    bundled: Arc<BundledContexts>,
}

impl ContextProcessor {
    /// Reads the bundled core context and the contexts it imports.
    ///
    /// # Errors
    ///
    /// Returns [`BundledContextError`] if a bundled document cannot be read, a defect of the build.
    pub fn try_new() -> Result<Self, BundledContextError> {
        Ok(Self {
            bundled: Arc::new(BundledContexts::read()?),
        })
    }

    /// The active context of a document with no user `@context`: the core context alone (clause 5.5.5).
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::Processing`] if the bundled core context does not process, a defect of the
    /// build.
    pub async fn core(&self) -> Result<ActiveContext, ProcessingError<RequiresLoader>> {
        ActiveContext::build(None, &OfflineLoader, Arc::clone(&self.bundled)).await
    }

    /// Processes `context` without loading anything: every remote context it references must be the core
    /// context.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::Loading`] carrying [`RequiresLoader`] for a remote context other than the core
    /// context, and the errors of [`ContextProcessor::process`].
    pub async fn process_offline(&self, context: &Context) -> Result<ActiveContext, ProcessingError<RequiresLoader>> {
        ActiveContext::build(Some(context), &OfflineLoader, Arc::clone(&self.bundled)).await
    }

    /// Processes `context`, loading the remote contexts it references with `loader`; the core context is always
    /// answered from the bundled copy.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::Syntax`] for an `@context` that is not a JSON-LD context,
    /// [`ProcessingError::ScopedContext`] for a user term definition that carries a scoped context,
    /// [`ProcessingError::Loading`] for a remote context `loader` cannot load, and [`ProcessingError::Processing`]
    /// when JSON-LD context processing fails.
    pub async fn process<L: RemoteContextLoader>(&self, context: &Context, loader: &L) -> Result<ActiveContext, ProcessingError<L::Error>> {
        ActiveContext::build(Some(context), loader, Arc::clone(&self.bundled)).await
    }
}

#[cfg(test)]
mod tests;
