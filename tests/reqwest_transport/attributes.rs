//! Partial Attribute Update over the client, with each body the operation takes (clauses 5.6.4 and 6.7.3.1).

use crate::fixtures::client;
use http::header;
use iri_rs::iri;
use ngsi_ld::{
    http::UpdateOutcome,
    model::{Attribute, AttributeKind, AttributeMembers, AttributeName, EntityId, InstanceDeletion, Property},
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path},
};

/// The entity whose attribute is updated.
fn vehicle_id() -> EntityId {
    EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"))
}

/// A server that answers 204 once to a PATCH of `speed` carrying `body`.
async fn server_expecting(body: Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(body_json(body))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn update_attribute_sends_a_fragment_with_a_payload_given_as_a_patch() {
    let server = server_expecting(json!({"type": "Property", "value": 90})).await;
    let speed: AttributeName = "speed".parse().unwrap();
    let patch = Attribute::from(Property::builder(90).build().unwrap()).into_patch();
    let outcome = client(&server.uri())
        .entities()
        .update_attribute(&vehicle_id(), &speed, patch)
        .send()
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::Done);
}

#[tokio::test]
async fn update_attribute_sends_a_fragment_without_a_payload_given_as_its_members() {
    let server = server_expecting(json!({"observedAt": "2022-03-14T12:51:02Z"})).await;
    let speed: AttributeName = "speed".parse().unwrap();
    let members = AttributeMembers::builder()
        .observed_at("2022-03-14T12:51:02Z".parse().unwrap())
        .build()
        .unwrap();
    let outcome = client(&server.uri())
        .entities()
        .update_attribute(&vehicle_id(), &speed, members)
        .send()
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::Done);
}

#[tokio::test]
async fn update_attribute_sends_the_deletion_of_the_instance_as_the_null_payload() {
    let server = server_expecting(json!({"type": "Property", "value": "urn:ngsi-ld:null"})).await;
    let speed: AttributeName = "speed".parse().unwrap();
    let deletion = InstanceDeletion::new(AttributeKind::Property);
    let outcome = client(&server.uri())
        .entities()
        .update_attribute(&vehicle_id(), &speed, deletion)
        .send()
        .await
        .unwrap();
    assert_eq!(outcome, UpdateOutcome::Done);
}
