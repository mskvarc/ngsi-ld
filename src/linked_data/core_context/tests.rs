use crate::{
    linked_data::{
        core_context::BundledContexts,
        fixtures::{DEFAULT_CONTEXT, core},
    },
    model::{AttributeName, CORE_CONTEXT},
};
use jstrict::Value as Json;
use pretty_assertions::assert_eq;

/// Clause 4.4: the bundled core context parses, holds an `@context`, and `CORE_CONTEXT` names it.
#[tokio::test]
async fn the_bundled_core_context_parses_and_expands_an_undefined_term_under_its_vocab() {
    let bundled = BundledContexts::read().unwrap();
    assert!(bundled.core_context().iter().next().is_some());
    assert_eq!(CORE_CONTEXT.as_str(), "https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld");
    let core = core().await;
    let speed: AttributeName = "speed".parse().unwrap();
    assert_eq!(core.expand(&speed).unwrap().as_str(), format!("{DEFAULT_CONTEXT}speed"));
}

/// The core context's `ngsildproof` imports the Data Integrity context, which is bundled as well.
#[test]
fn the_bundled_data_integrity_context_is_a_json_object_with_a_context() {
    let bundled = BundledContexts::read().unwrap();
    let data_integrity = bundled.document(iri_rs::iri!("https://w3id.org/security/data-integrity/v2")).unwrap();
    assert!(matches!(data_integrity, Json::Object(object) if object.contains_key("@context")));
    assert!(bundled.document(CORE_CONTEXT).is_some());
    assert!(bundled.document(iri_rs::iri!("https://example.org/a.jsonld")).is_none());
}
