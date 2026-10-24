use crate::{
    linked_data::{
        ActiveContext,
        ProcessingError,
        RemoteContextLoader,
        RequiresLoader,
        fixtures::{A, DEFAULT_CONTEXT, active, core, inline, processor},
    },
    model::{AttributeName, CORE_CONTEXT, Context, ContextEntry, EntityTypeName, InlineContext, RawJson},
};
use iri_rs::{Iri, IriRefBuf};
use pretty_assertions::assert_eq;
use std::future;

/// The Fully Qualified Name `context` expands the attribute `name` to.
fn attribute(context: &ActiveContext, name: &str) -> String {
    context.expand(&name.parse::<AttributeName>().unwrap()).unwrap().to_string()
}

/// The Fully Qualified Name `context` expands the entity type `name` to.
fn entity_type(context: &ActiveContext, name: &str) -> String {
    context.expand(&name.parse::<EntityTypeName>().unwrap()).unwrap().to_string()
}

/// The URL of a remote copy of `A`.
const REMOTE_A: &str = "https://example.org/a.jsonld";

/// A URL [`ServesA`] does not serve.
const REMOTE_B: &str = "https://example.org/b.jsonld";

/// A loader serving `A` at [`REMOTE_A`], and nothing else.
struct ServesA;

/// The loader serves no other document.
#[derive(Debug, thiserror::Error)]
#[error("not served")]
struct NotServed;

impl RemoteContextLoader for ServesA {
    type Error = NotServed;

    fn load(&self, url: Iri<&str>) -> impl Future<Output = Result<RawJson, Self::Error>> + Send {
        future::ready(if url.as_str() == REMOTE_A {
            Ok(format!(r#"{{"@context":{A}}}"#).parse::<RawJson>().unwrap())
        } else {
            Err(NotServed)
        })
    }
}

/// A context naming the remote document at `url`.
fn remote(url: &str) -> Context {
    Context::from(IriRefBuf::new(url).unwrap())
}

/// Clauses 4.4 and 5.5.5: with no user context, the core context alone applies.
#[tokio::test]
async fn the_core_context_alone_expands_through_its_vocab_and_its_own_terms() {
    let core = core().await;
    assert_eq!(entity_type(&core, "Vehicle"), format!("{DEFAULT_CONTEXT}Vehicle"));
    assert_eq!(attribute(&core, "speed"), format!("{DEFAULT_CONTEXT}speed"));
    assert_eq!(attribute(&core, "location"), "https://uri.etsi.org/ngsi-ld/location");
    assert_eq!(core.source(), None);
}

/// Clause 5.5.7: a user context's terms, prefixes and absolute IRIs.
#[tokio::test]
async fn a_user_context_expands_its_terms_its_prefixes_and_keeps_an_iri() {
    let a = active(A).await;
    assert_eq!(entity_type(&a, "Vehicle"), "https://example.org/Vehicle");
    assert_eq!(attribute(&a, "ex:color"), "https://example.org/color");
    assert_eq!(attribute(&a, "https://example.org/x"), "https://example.org/x");
    assert_eq!(a.source(), Some(&inline(A)));
}

/// Clauses 4.4 and C.9: the core context wins over a user definition of a core term, and the prefixed
/// form is how a user context names its own.
#[tokio::test]
async fn a_core_term_cannot_be_redefined_and_a_prefixed_term_is_the_users() {
    let user = active(r#"{"location":"https://schema.org/location","schema":"https://schema.org/"}"#).await;
    assert_eq!(attribute(&user, "location"), "https://uri.etsi.org/ngsi-ld/location");
    assert_eq!(attribute(&user, "schema:location"), "https://schema.org/location");
}

/// Clause 5.5.7: a user context with a scoped context is refused.
#[tokio::test]
async fn a_user_term_with_a_scoped_context_is_refused() {
    let scoped = inline(r#"{"Vehicle":{"@id":"https://example.org/Vehicle","@context":{"speed":"https://example.org/v"}}}"#);
    let error = processor().process_offline(&scoped).await.unwrap_err();
    assert!(matches!(error, ProcessingError::ScopedContext { term } if term.as_str() == "Vehicle"));
}

/// Clause 5.5.6: a remote context is loaded with the loader given; one the loader cannot load is reported
/// with its URL and the loader's error.
#[tokio::test]
async fn a_remote_context_is_loaded_with_the_loader_given() {
    let loaded = processor().process(&remote(REMOTE_A), &ServesA).await.unwrap();
    assert_eq!(entity_type(&loaded, "Vehicle"), "https://example.org/Vehicle");
    assert_eq!(attribute(&loaded, "ex:color"), "https://example.org/color");
    let error = processor().process(&remote(REMOTE_B), &ServesA).await.unwrap_err();
    assert!(matches!(error, ProcessingError::Loading { url, source: NotServed } if url.as_str() == REMOTE_B));
}

/// Clause 4.4: the core context is never loaded, even when named; any other remote context needs a loader.
#[tokio::test]
async fn the_named_core_context_is_bundled_and_another_remote_one_needs_a_loader() {
    let named = Context::Multiple(vec![
        ContextEntry::Reference(IriRefBuf::new(CORE_CONTEXT.as_str()).unwrap()),
        ContextEntry::Inline(A.parse::<InlineContext>().unwrap()),
    ]);
    let processed = processor().process(&named, &ServesA).await.unwrap();
    assert_eq!(entity_type(&processed, "Vehicle"), "https://example.org/Vehicle");
    assert_eq!(attribute(&processed, "location"), "https://uri.etsi.org/ngsi-ld/location");
    let error = processor().process_offline(&remote(REMOTE_A)).await.unwrap_err();
    assert!(matches!(error, ProcessingError::Loading { source: RequiresLoader, .. }));
}

/// A value that is not a JSON-LD context is refused before processing.
#[tokio::test]
async fn an_inline_context_that_is_not_a_context_is_refused_as_syntax() {
    let error = processor().process_offline(&inline(r#"{"speed":3}"#)).await.unwrap_err();
    assert!(matches!(error, ProcessingError::Syntax(_)));
}

/// JSON-LD 1.1 API section 4.1.2: an error of the context processing algorithm is reported with its JSON-LD
/// error code.
#[tokio::test]
async fn a_processing_error_names_its_json_ld_error_code() {
    let error = processor().process_offline(&inline(r#"{"x":{"@id":"@context"}}"#)).await.unwrap_err();
    let ProcessingError::Processing(detected) = error else {
        panic!("expected a processing error, found {error:?}");
    };
    assert_eq!(detected.to_string(), "the JSON-LD processor detected `invalid keyword alias`");
}
