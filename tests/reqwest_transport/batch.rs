//! The batch operations over the client (clauses 6.14.3.1 and 6.17.3.1).

use crate::{
    fixtures::{client, vehicle},
    links::context_link,
};
use http::header;
use iri_rs::iri;
use ngsi_ld::{
    client::ClientError,
    http::{BatchCreateOutcome, BatchOutcome, RequestError, operation::ArgumentError},
    model::{EntityId, ErrorType},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path},
};

#[tokio::test]
async fn a_batch_create_sends_the_array_and_reads_a_207_entity_by_entity() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entityOperations/create"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(json!([{
            "id": "urn:ngsi-ld:Vehicle:A4567",
            "type": "Vehicle",
            "brandName": {"type": "Property", "value": "Mercedes"}
        }])))
        .respond_with(ResponseTemplate::new(207).set_body_json(json!({
            "success": [],
            "errors": [{
                "entityId": "urn:ngsi-ld:Vehicle:A4567",
                "error": {"type": "https://uri.etsi.org/ngsi-ld/errors/AlreadyExists", "title": "Already exists"}
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let outcome = client(&server.uri()).entities().batch_create(&[vehicle()]).send().await.unwrap();
    let BatchCreateOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.success, Vec::<EntityId>::new());
    assert_eq!(result.errors[0].entity_id, EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")));
    assert_eq!(result.errors[0].error.error_type, Some(ErrorType::AlreadyExists));
}

#[tokio::test]
async fn a_batch_delete_sends_the_ids_and_reads_204_as_done() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entityOperations/delete"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(json!(["urn:ngsi-ld:Vehicle:A4567", "urn:ngsi-ld:Vehicle:B9211"])))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let ids = [
        EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")),
        EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211")),
    ];
    let outcome = client(&server.uri()).entities().batch_delete(&ids).send().await.unwrap();
    assert_eq!(outcome, BatchOutcome::Done);
}

#[tokio::test]
async fn an_empty_batch_is_refused_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .expect(0)
        .mount(&server)
        .await;
    let error = client(&server.uri()).entities().batch_delete(&[]).send().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::EmptyBatch))),
        "{error:?}"
    );
}
