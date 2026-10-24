//! The errors the JSON-LD processor reports (JSON-LD 1.1 API section 9.4.2), behind a type of the crate's own.

use crate::linked_data::RequiresLoader;
use jsonld::{
    ExtractContextError,
    compaction::Error as CompactionError,
    expansion::Error as ExpansionError,
    syntax::{ErrorCode, context::InvalidContext},
};

/// The JSON-LD processor refused a context or a document, with the error code of JSON-LD 1.1 API section 9.4.2.
///
/// The processor's error types are not part of the crate's API: a [`JsonLdError`] carries the JSON-LD error code
/// in its message and the processor's own error, when there is one, as its source.
///
/// ```
/// use ngsi_ld::{linked_data::{ProcessingError, ContextProcessor}, model::{Context, ContextEntry, InlineContext}};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let processor = ContextProcessor::try_new()?;
/// let alias = Context::Single(ContextEntry::Inline(r#"{"x":{"@id":"@context"}}"#.parse::<InlineContext>()?));
/// let refused = processor.process_offline(&alias).await;
/// assert!(matches!(
///     refused,
///     Err(ProcessingError::Processing(error)) if error.to_string() == "the JSON-LD processor detected `invalid keyword alias`"
/// ));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
#[error("the JSON-LD processor detected `{code}`")]
pub struct JsonLdError {
    /// The error code.
    code: ErrorCode,
    /// The processor's own error, when it has one beyond the code.
    #[source]
    detail: Option<Detail>,
}

/// The processor's own errors, shown through [`JsonLdError`]'s source.
#[derive(Debug, thiserror::Error)]
enum Detail {
    /// An `@context` is not a JSON-LD context.
    #[error(transparent)]
    Syntax(InvalidContext),
    /// A loaded document holds no usable `@context`.
    #[error(transparent)]
    Extraction(ExtractContextError),
    /// The expansion algorithm failed.
    #[error(transparent)]
    Expansion(ExpansionError<RequiresLoader>),
    /// The compaction algorithm failed.
    #[error(transparent)]
    Compaction(CompactionError<RequiresLoader>),
}

impl JsonLdError {
    /// An `@context` that is not a JSON-LD context.
    pub(crate) fn syntax(error: InvalidContext) -> Self {
        Self {
            code: error.code(),
            detail: Some(Detail::Syntax(error)),
        }
    }

    /// A loaded document that holds no usable `@context`.
    pub(crate) const fn extraction(error: ExtractContextError) -> Self {
        Self {
            code: ErrorCode::LoadingRemoteContextFailed,
            detail: Some(Detail::Extraction(error)),
        }
    }

    /// A failure of the context processing algorithm that the code describes in full.
    pub(crate) const fn processing(code: ErrorCode) -> Self {
        Self { code, detail: None }
    }

    /// A failure of the expansion algorithm.
    pub(crate) fn expansion(error: ExpansionError<RequiresLoader>) -> Self {
        Self {
            code: error.code(),
            detail: Some(Detail::Expansion(error)),
        }
    }

    /// A failure of the compaction algorithm.
    pub(crate) fn compaction(error: CompactionError<RequiresLoader>) -> Self {
        Self {
            code: error.code(),
            detail: Some(Detail::Compaction(error)),
        }
    }
}
