//! Retrieve Attribute Value, over the client (ETSI TS 104 176 clause 7.6.3.1).

use crate::{fixtures::client, links::context_link};
use http::header;
use ngsi_ld::model::{AttributeName, DatasetId, EntityId, ErrorType};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{header, method, path, query_param},
};

#[tokio::test]
async fn retrieve_attribute_value_sends_its_parameters_and_reads_the_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("datasetId", "urn:ngsi-ld:dataset:gps"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(80)))
        .expect(1)
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let speed: AttributeName = "speed".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let retrieved = client(&server.uri())
        .entities()
        .retrieve_attribute_value(&id, &speed)
        .entity_type("Vehicle")
        .dataset_id(&gps)
        .send()
        .await
        .unwrap();
    assert_eq!(retrieved.body().to_string(), "80");
}

#[tokio::test]
async fn a_missing_attribute_value_comes_back_as_resource_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Not found"})))
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let speed: AttributeName = "speed".parse().unwrap();
    let error = client(&server.uri()).entities().retrieve_attribute_value(&id, &speed).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound));
}
