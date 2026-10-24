//! The JSON-LD context loader over the client (JSON-LD 1.1 API section 9.4 and clause 6.30.3.1).

use crate::fixtures::client;
use http::header;
use iri_rs::{IriBuf, IriRefBuf, iri};
use ngsi_ld::{
    client::{ClientError, ContextLoadError, ContextSource, MediaTypeCheck, ReqwestTransport, Transport},
    linked_data::{ContextProcessor, ProcessingError, RemoteContextLoader},
    model::{Context, EntityTypeName, RawJson},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use std::fmt::Debug;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{header, method, path},
};

/// The context `A` of the linked data tests.
fn context_a() -> serde_json::Value {
    json!({"Vehicle": "https://example.org/Vehicle", "speed": "https://example.org/speed", "ex": "https://example.org/"})
}

/// The context document serving `A`.
fn document_a() -> serde_json::Value {
    json!({"@context": context_a()})
}

/// A server answering `GET /a.jsonld` with `response`.
async fn serving(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/a.jsonld")).respond_with(response).mount(&server).await;
    server
}

/// The URL of `/a.jsonld` on `server`.
fn url_a(server: &MockServer) -> IriBuf {
    IriBuf::new(format!("{}/a.jsonld", server.uri())).unwrap()
}

/// The document a successful load returned, as JSON.
fn loaded<E: Debug>(result: Result<RawJson, E>) -> serde_json::Value {
    serde_json::from_str(&result.unwrap().to_string()).unwrap()
}

/// The error of a failed load.
fn refused<E>(result: Result<RawJson, E>) -> E {
    result.err().unwrap()
}

/// The type of every error of a load over `ReqwestTransport`.
type Refusal = ContextLoadError<<ReqwestTransport as Transport>::Error>;

#[tokio::test]
async fn a_context_is_loaded_from_its_url_asking_for_json_ld_or_json() {
    let server = serving(ResponseTemplate::new(200).set_body_raw(document_a().to_string(), "application/ld+json")).await;
    let client = client(&server.uri());
    let loader = client
        .context_loader()
        .source(ContextSource::Direct)
        .media_type(MediaTypeCheck::Required)
        .build();
    assert_eq!(loaded(loader.load(url_a(&server).as_ref()).await), document_a());
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let accept: Vec<_> = requests[0]
        .headers
        .get_all(&header::ACCEPT)
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect();
    assert_eq!(accept, ["application/ld+json, application/json"]);
}

#[tokio::test]
async fn a_plain_text_context_is_refused_unless_the_media_type_is_ignored() {
    let server = serving(ResponseTemplate::new(200).set_body_raw(document_a().to_string(), "text/plain")).await;
    let client = client(&server.uri());
    let required: Refusal = refused(client.context_loader().build().load(url_a(&server).as_ref()).await);
    assert!(matches!(required, ContextLoadError::MediaType { found: Some(found) } if found.as_str() == "text/plain"));
    let ignored = client.context_loader().media_type(MediaTypeCheck::Ignored).build();
    assert_eq!(loaded(ignored.load(url_a(&server).as_ref()).await), document_a());
}

#[tokio::test]
async fn another_status_than_200_and_a_body_that_is_not_json_are_refused() {
    let missing = serving(ResponseTemplate::new(404)).await;
    let client = client(&missing.uri());
    let status: Refusal = refused(client.context_loader().build().load(url_a(&missing).as_ref()).await);
    assert!(matches!(status, ContextLoadError::Status { status } if status.as_u16() == 404));
    let broken = serving(ResponseTemplate::new(200).set_body_raw("{", "application/json")).await;
    let client = self::client(&broken.uri());
    let not_json: Refusal = refused(client.context_loader().build().load(url_a(&broken).as_ref()).await);
    assert!(matches!(not_json, ContextLoadError::NotJson { .. }));
}

#[tokio::test]
async fn a_context_is_loaded_from_the_broker_through_serve_context() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts/https:%2F%2Fexample.org%2Fa.jsonld"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(document_a().to_string(), "application/json"))
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let broker = client.context_loader().source(ContextSource::Broker).build();
    let url = iri!("https://example.org/a.jsonld");
    assert_eq!(loaded(broker.load(url).await), document_a());
    let unknown = iri!("https://example.org/b.jsonld");
    let problem = json!({"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "not found"});
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts/https:%2F%2Fexample.org%2Fb.jsonld"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(problem.to_string(), "application/json"))
        .mount(&server)
        .await;
    let error: Refusal = refused(broker.load(unknown).await);
    assert!(matches!(
        error,
        ContextLoadError::Client {
            source: ClientError::Response(_)
        }
    ));
}

#[tokio::test]
async fn an_active_context_is_processed_with_the_client_loader() {
    let server = serving(ResponseTemplate::new(200).set_body_raw(document_a().to_string(), "application/ld+json")).await;
    let client = client(&server.uri());
    let context = Context::from(IriRefBuf::new(format!("{}/a.jsonld", server.uri())).unwrap());
    let active = ContextProcessor::new()
        .unwrap()
        .process(&context, &client.context_loader().build())
        .await
        .unwrap();
    let vehicle: EntityTypeName = "Vehicle".parse().unwrap();
    assert_eq!(active.expand(&vehicle).unwrap().as_str(), "https://example.org/Vehicle");
}

#[tokio::test]
async fn a_context_the_client_loader_cannot_load_is_reported_with_its_url_and_status() {
    let server = serving(ResponseTemplate::new(404)).await;
    let client = client(&server.uri());
    let context = Context::from(IriRefBuf::new(format!("{}/a.jsonld", server.uri())).unwrap());
    let error = ContextProcessor::new()
        .unwrap()
        .process(&context, &client.context_loader().build())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ProcessingError::Loading { url, source: ContextLoadError::Status { status } } if url == url_a(&server) && status.as_u16() == 404
    ));
}
