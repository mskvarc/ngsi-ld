//! Delete Entity (clause 6.5.3.2) and Purge Entities (clause 6.4.3.3).

use crate::broker::{attribute_names, client, create, create_fleet, entity_id, name, parked_vehicle, remove_all};
use ngsi_ld::{
    client::{Client, ClientError, ReqwestTransport},
    http::{BatchOutcome, RequestError, operation::ArgumentError},
    model::{Entity, ErrorType, Output},
};
use pretty_assertions::assert_eq;

/// The entities of `vehicle_type` the broker holds.
async fn entities_of_type(client: &Client<ReqwestTransport>, vehicle_type: &str) -> Vec<Entity<Output>> {
    client.entities().query().entity_type(vehicle_type).send().await.unwrap().into_items()
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn delete_entity_removes_the_entity() {
    let client = client();
    let id = entity_id("delete");
    create(&client, &parked_vehicle(&id)).await;
    assert_eq!(client.entities().delete(&id).send().await.unwrap(), BatchOutcome::Done);
    let error = client.entities().retrieve(&id).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn deleting_a_missing_entity_is_resource_not_found() {
    let error = client().entities().delete(&entity_id("deleteMissing")).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn purge_by_type_deletes_every_entity_of_the_type() {
    // Known deviation, clause 6.4.3.3: `DELETE /entities` purges the matching entities. Observed: the broker
    // answers 405 with its own problem type `https://uri.etsi.org/ngsi-ld/errors/MethodNotAllowed`.
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "purge", 3).await;
    let outcome = client.entities().purge().entity_type(vehicle_type.as_str()).send().await;
    let remaining = entities_of_type(&client, &vehicle_type).await;
    remove_all(&client, &fleet).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert!(remaining.is_empty(), "{remaining:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn purge_with_drop_removes_only_the_dropped_attributes() {
    // Known deviation, clause 6.4.3.3: as for the purge by type, the broker answers 405.
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "purgeDrop", 3).await;
    let outcome = client
        .entities()
        .purge()
        .entity_type(vehicle_type.as_str())
        .drop(vec![name("color")])
        .send()
        .await;
    let remaining = entities_of_type(&client, &vehicle_type).await;
    remove_all(&client, &fleet).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert_eq!(remaining.len(), 3);
    assert!(
        remaining.iter().all(|vehicle| attribute_names(vehicle) == ["brandName", "speed"]),
        "{remaining:?}"
    );
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn purge_with_keep_removes_every_other_attribute() {
    // Known deviation, clause 6.4.3.3: as for the purge by type, the broker answers 405.
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "purgeKeep", 3).await;
    let outcome = client
        .entities()
        .purge()
        .entity_type(vehicle_type.as_str())
        .keep(vec![name("speed")])
        .send()
        .await;
    let remaining = entities_of_type(&client, &vehicle_type).await;
    remove_all(&client, &fleet).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert_eq!(remaining.len(), 3);
    assert!(remaining.iter().all(|vehicle| attribute_names(vehicle) == ["speed"]), "{remaining:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn purge_without_type_or_q_is_refused_before_sending() {
    let error = client().entities().purge().send().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::UnboundedQuery))),
        "{error:?}"
    );
}
