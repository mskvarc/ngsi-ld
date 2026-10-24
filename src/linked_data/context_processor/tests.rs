use crate::{
    linked_data::fixtures::{A, DEFAULT_CONTEXT, inline, processor},
    model::{AttributeName, EntityTypeName},
};
use pretty_assertions::assert_eq;
use std::ptr;

/// Clauses 4.4 and 5.5.7: one processor builds the core context alone and a user context over it.
#[tokio::test]
async fn one_processor_builds_the_core_context_and_a_user_context() {
    let processor = processor();
    let speed: AttributeName = "speed".parse().unwrap();
    let vehicle: EntityTypeName = "Vehicle".parse().unwrap();
    let core = processor.core().await.unwrap();
    assert_eq!(core.expand(&speed).unwrap().as_str(), format!("{DEFAULT_CONTEXT}speed"));
    let a = processor.process_offline(&inline(A)).await.unwrap();
    assert_eq!(a.expand(&vehicle).unwrap().as_str(), "https://example.org/Vehicle");
    assert_eq!(a.expand(&speed).unwrap().as_str(), "https://example.org/speed");
}

/// The active contexts of one processor share the bundled documents it read.
#[tokio::test]
async fn the_active_contexts_of_one_processor_share_its_bundled_documents() {
    let processor = processor();
    let core = processor.core().await.unwrap();
    let a = processor.process_offline(&inline(A)).await.unwrap();
    assert!(ptr::eq(core.bundled(), a.bundled()));
    let other = self::processor().core().await.unwrap();
    assert!(!ptr::eq(core.bundled(), other.bundled()));
}
