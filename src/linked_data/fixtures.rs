//! The contexts the tests of the linked data module share.

use crate::{
    linked_data::{ActiveContext, ContextProcessor},
    model::{Context, ContextEntry, InlineContext},
};

/// Context `A`: `Vehicle` and `speed` under `https://example.org/`, and the `ex` prefix.
pub(crate) const A: &str = r#"{"Vehicle":"https://example.org/Vehicle","speed":"https://example.org/speed","ex":"https://example.org/"}"#;

/// Context `B`: the same Fully Qualified Names as `A`, spelled `Car` and `velocity`.
pub(crate) const B: &str = r#"{"Car":"https://example.org/Vehicle","velocity":"https://example.org/speed"}"#;

/// The `@vocab` of the core context, under which an undefined term expands.
pub(crate) const DEFAULT_CONTEXT: &str = "https://uri.etsi.org/ngsi-ld/default-context/";

/// The inline `@context` `json`.
pub(crate) fn inline(json: &str) -> Context {
    Context::Single(ContextEntry::Inline(json.parse::<InlineContext>().unwrap()))
}

/// A context processor.
pub(crate) fn processor() -> ContextProcessor {
    ContextProcessor::try_new().unwrap()
}

/// The active context of the core context alone.
pub(crate) async fn core() -> ActiveContext {
    processor().core().await.unwrap()
}

/// The active context of the inline `@context` `json`, processed offline.
pub(crate) async fn active(json: &str) -> ActiveContext {
    processor().process_offline(&inline(json)).await.unwrap()
}
