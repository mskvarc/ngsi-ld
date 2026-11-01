//! Set Attribute Value, over the client (ETSI TS 104 176 clause 7.6.3.2).

use crate::{fixtures::client, links::context_link};
use http::header;
use ngsi_ld::{
    http::BatchOutcome,
    model::{AttributeName, DatasetId, DateTime, EntityId, ErrorType, Value},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

#[tokio::test]
async fn set_attribute_value_sends_the_bare_value_and_its_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value"))
        .and(query_param("datasetId", "urn:ngsi-ld:dataset:gps"))
        .and(query_param("observedAt", "2024-05-01T10:00:00Z"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(json!(30)))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let speed: AttributeName = "speed".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let observed: DateTime = "2024-05-01T10:00:00Z".parse().unwrap();
    let slower = Value::from(30);
    let outcome = client(&server.uri())
        .entities()
        .set_attribute_value(&id, &speed, &slower)
        .dataset_id(&gps)
        .observed_at(&observed)
        .send()
        .await
        .unwrap();
    assert_eq!(outcome, BatchOutcome::Done);
}

#[tokio::test]
async fn setting_the_value_of_a_missing_attribute_comes_back_as_resource_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Not found"})))
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let speed: AttributeName = "speed".parse().unwrap();
    let slower = Value::from(30);
    let error = client(&server.uri())
        .entities()
        .set_attribute_value(&id, &speed, &slower)
        .send()
        .await
        .unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound));
}
