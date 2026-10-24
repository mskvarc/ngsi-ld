//! Merge Entity with a fragment in simplified representation, over the client (clause 6.5.3.4).

use crate::fixtures::client;
use http::header;
use iri_rs::iri;
use langtag::LangTagBuf;
use ngsi_ld::{
    client::ClientError,
    http::{BatchOutcome, RequestError, operation::ArgumentError},
    model::{EntityId, SimplifiedFragment},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// The entity the fragments merge into.
fn vehicle_id() -> EntityId {
    EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"))
}

#[tokio::test]
async fn merge_simplified_sends_the_bare_values_with_format_and_lang() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(query_param("format", "simplified"))
        .and(query_param("lang", "fr"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(body_json(json!({"street": "Grand Place", "isParked": "urn:ngsi-ld:null"})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let fragment = SimplifiedFragment::builder().value("street", "Grand Place").delete("isParked").build().unwrap();
    let outcome = client(&server.uri())
        .entities()
        .merge_simplified(&vehicle_id(), &fragment)
        .lang("fr")
        .send()
        .await
        .unwrap();
    assert_eq!(outcome, BatchOutcome::Done);
}

#[tokio::test]
async fn merge_simplified_takes_a_language_tag_parsed_beforehand() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(query_param("format", "simplified"))
        .and(query_param("lang", "nl-BE"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let fragment = SimplifiedFragment::builder().value("street", "Grote Markt").build().unwrap();
    let flemish: LangTagBuf = "nl-BE".parse().unwrap();
    let outcome = client(&server.uri())
        .entities()
        .merge_simplified(&vehicle_id(), &fragment)
        .lang(&flemish)
        .send()
        .await
        .unwrap();
    assert_eq!(outcome, BatchOutcome::Done);
}

#[tokio::test]
async fn an_invalid_lang_is_refused_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(204))
        .expect(0)
        .mount(&server)
        .await;
    let fragment = SimplifiedFragment::builder().value("street", "Grand Place").build().unwrap();
    let error = client(&server.uri())
        .entities()
        .merge_simplified(&vehicle_id(), &fragment)
        .lang("not a tag")
        .send()
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::LanguageTag { .. }))));
}
