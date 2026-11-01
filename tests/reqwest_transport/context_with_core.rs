//! Serve @context with `core`, over the client (ETSI TS 104 175 clause 13.4.4, ETSI TS 104 176 clause 10.3.3.1).

use crate::fixtures::client;
use http::header;
use iri_rs::IriRefBuf;
use ngsi_ld::model::{ConformanceVersion, Context, ContextEntry, ContextId, ErrorType};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{header, method, path, query_param, query_param_is_missing},
};

#[tokio::test]
async fn serve_with_core_sends_core_alone_and_reads_the_references() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1"))
        .and(query_param("core", "1.9"))
        .and(query_param_is_missing("details"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@context": [
                "https://example.org/context.jsonld",
                "https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let id: ContextId = "urn:ngsi-ld:context:c1".parse().unwrap();
    let context = client(&server.uri())
        .contexts()
        .serve_with_core(&id, ConformanceVersion::new(1, 9))
        .send()
        .await
        .unwrap();
    let reference = |text: &str| ContextEntry::Reference(IriRefBuf::new(text.to_owned()).unwrap());
    assert_eq!(
        context,
        Context::Multiple(vec![
            reference("https://example.org/context.jsonld"),
            reference("https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"),
        ])
    );
}

#[tokio::test]
async fn serve_with_core_of_an_unknown_context_comes_back_as_resource_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:unknown"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Not found"})))
        .mount(&server)
        .await;
    let id: ContextId = "urn:ngsi-ld:context:unknown".parse().unwrap();
    let error = client(&server.uri())
        .contexts()
        .serve_with_core(&id, ConformanceVersion::new(1, 9))
        .send()
        .await
        .unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound));
}
