//! Retrieve Attribute, over the client (ETSI TS 104 176 clause 7.5.3.1).

use crate::{fixtures::client, links::context_link};
use http::header;
use ngsi_ld::{
    http::operation::SystemAttributes,
    model::{AttributeName, DatasetId, EntityId, ErrorType, Lossless},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{header, method, path, query_param},
};

#[tokio::test]
async fn retrieve_attribute_sends_its_parameters_and_reads_the_attribute() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("datasetId", "urn:ngsi-ld:dataset:gps"))
        .and(query_param("options", "sysAttrs"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "Property",
            "value": 80,
            "datasetId": "urn:ngsi-ld:dataset:gps",
            "createdAt": "2024-05-01T10:00:00Z"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let speed: AttributeName = "speed".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let retrieved = client(&server.uri())
        .entities()
        .retrieve_attribute(&id, &speed)
        .entity_type("Vehicle")
        .dataset_id(&gps)
        .system_attributes(SystemAttributes::Include)
        .send()
        .await
        .unwrap();
    assert_eq!(
        retrieved.body().attribute.render(Lossless::Concise).to_string(),
        r#"{"value":80,"datasetId":"urn:ngsi-ld:dataset:gps","createdAt":"2024-05-01T10:00:00Z"}"#
    );
}

#[tokio::test]
async fn a_missing_attribute_comes_back_as_resource_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Not found"})))
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let speed: AttributeName = "speed".parse().unwrap();
    let error = client(&server.uri()).entities().retrieve_attribute(&id, &speed).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound));
}
