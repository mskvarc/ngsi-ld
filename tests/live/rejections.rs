//! Requests the specification makes a broker reject, built through the unchecked path (clause 5.5.4) and answered
//! with the error types of clause 5.5.2.

use crate::broker::{client, entity_id, parked_vehicle, remove};
use bytes::Bytes;
use ngsi_ld::{
    http::{UncheckedDocument, operation::CreateEntity},
    model::{ErrorType, Lossless},
};
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn creating_an_entity_without_a_type_is_bad_request_data() {
    let client = client();
    let id = entity_id("untyped");
    let vehicle = parked_vehicle(&id);
    let untyped = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized).remove_member("type");
    let create = CreateEntity::builder().entity(&vehicle).build();
    let request = client.unchecked(&create).unwrap().document(&untyped);
    let outcome = client.send_unchecked(request).await;
    if outcome.is_ok() {
        remove(&client, &id).await;
    }
    let error = outcome.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::BadRequestData), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn creating_an_entity_from_a_body_that_is_not_json_is_invalid_request() {
    let client = client();
    let vehicle = parked_vehicle(&entity_id("notJson"));
    let create = CreateEntity::builder().entity(&vehicle).build();
    let request = client.unchecked(&create).unwrap().body(Bytes::from_static(br#"{"id": "urn:ngsi-ld:Vehicle:"#));
    let error = client.send_unchecked(request).await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::InvalidRequest), "{error:?}");
}
