//! Batch Entity Creation, Upsert, Update, Merge and Delete (clauses 6.14 to 6.17 and 6.31).

use crate::broker::{attribute_names, client, create, entity_id, parked_vehicle, property_value, remove};
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{
        BatchCreateOutcome,
        BatchOutcome,
        Retrieved,
        UpsertOutcome,
        operation::{Overwrite, UpsertMode},
    },
    model::{AttributeKind, Entity, EntityFragment, EntityId, ErrorType, Output, Value},
};
use pretty_assertions::assert_eq;

/// The entity `id` as the broker holds it.
async fn stored(client: &Client<ReqwestTransport>, id: &EntityId) -> Entity<Output> {
    client.entities().retrieve(id).send().await.map(Retrieved::into_body).unwrap()
}

/// The ids, sorted: the broker lists them in any order (Table 6.14.3.1-1).
fn sorted<'a>(ids: impl IntoIterator<Item = &'a EntityId>) -> Vec<&'a str> {
    let mut ids: Vec<&str> = ids.into_iter().map(EntityId::as_str).collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn batch_create_creates_every_entity_and_lists_their_ids() {
    let client = client();
    let ids = [entity_id("batchCreateA"), entity_id("batchCreateB")];
    let vehicles = ids.each_ref().map(parked_vehicle);
    let outcome = client.entities().batch_create(&vehicles).send().await;
    let first = client.entities().retrieve(&ids[0]).send().await;
    let second = client.entities().retrieve(&ids[1]).send().await;
    for id in &ids {
        remove(&client, id).await;
    }
    let outcome = outcome.unwrap();
    let BatchCreateOutcome::Created(created) = &outcome else {
        panic!("expected a 201, got {outcome:?}")
    };
    assert_eq!(sorted(created), sorted(&ids));
    assert!(first.is_ok() && second.is_ok(), "{first:?} {second:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn batch_create_of_an_existing_entity_reports_already_exists_for_it_alone() {
    let client = client();
    let existing = entity_id("batchCreateExisting");
    let fresh = entity_id("batchCreateFresh");
    create(&client, &parked_vehicle(&existing)).await;
    let outcome = client
        .entities()
        .batch_create(&[parked_vehicle(&existing), parked_vehicle(&fresh)])
        .send()
        .await;
    remove(&client, &existing).await;
    remove(&client, &fresh).await;
    let outcome = outcome.unwrap();
    let BatchCreateOutcome::Partial(result) = outcome else {
        panic!("expected a 207, got {outcome:?}")
    };
    assert_eq!(result.success, [fresh]);
    assert_eq!(result.errors.len(), 1, "{result:?}");
    assert_eq!(result.errors[0].entity_id, existing);
    assert_eq!(result.errors[0].error.error_type, Some(ErrorType::AlreadyExists));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn batch_upsert_creates_then_replaces_then_updates() {
    let client = client();
    let id = entity_id("batchUpsert");
    let created = client.entities().batch_upsert(&[parked_vehicle(&id)]).send().await;
    let slim = Entity::builder().id(id.as_str()).entity_type("Vehicle").property("speed", 30).build().unwrap();
    let replaced = client.entities().batch_upsert(&[slim]).send().await;
    let after_replace = stored(&client, &id).await;
    let colored = Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("color", "red")
        .build()
        .unwrap();
    let updated = client.entities().batch_upsert(&[colored]).mode(UpsertMode::Update).send().await;
    let after_update = stored(&client, &id).await;
    remove(&client, &id).await;
    assert_eq!(created.unwrap(), UpsertOutcome::Created(vec![id]));
    assert_eq!(replaced.unwrap(), UpsertOutcome::Updated);
    assert_eq!(attribute_names(&after_replace), ["speed"]);
    assert_eq!(updated.unwrap(), UpsertOutcome::Updated);
    assert_eq!(attribute_names(&after_update), ["color", "speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn batch_update_without_overwrite_keeps_existing_attributes() {
    let client = client();
    let id = entity_id("batchUpdateKeep");
    create(&client, &parked_vehicle(&id)).await;
    let fragment = EntityFragment::for_entity(id.as_str(), "Vehicle")
        .property("speed", 70)
        .property("color", "red")
        .build()
        .unwrap();
    let outcome = client.entities().batch_update(&[fragment]).overwrite(Overwrite::Keep).send().await;
    let after = stored(&client, &id).await;
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert_eq!(property_value(&after, "speed"), Some(&Value::from(0)));
    assert_eq!(attribute_names(&after), ["brandName", "color", "isParked", "speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn batch_merge_sets_an_attribute_and_deletes_one() {
    let client = client();
    let id = entity_id("batchMerge");
    create(&client, &parked_vehicle(&id)).await;
    let fragment = EntityFragment::for_entity(id.as_str(), "Vehicle")
        .property("speed", 50)
        .delete_kind("isParked", AttributeKind::Relationship)
        .build()
        .unwrap();
    let outcome = client.entities().batch_merge(&[fragment]).send().await;
    let after = stored(&client, &id).await;
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert_eq!(property_value(&after, "speed"), Some(&Value::from(50)));
    assert_eq!(attribute_names(&after), ["brandName", "speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn batch_delete_deletes_every_entity_and_reports_missing_ones() {
    let client = client();
    let ids = [entity_id("batchDeleteA"), entity_id("batchDeleteB")];
    for id in &ids {
        create(&client, &parked_vehicle(id)).await;
    }
    let deleted = client.entities().batch_delete(&ids).send().await;
    let again = client.entities().batch_delete(&ids).send().await;
    for id in &ids {
        remove(&client, id).await;
    }
    assert_eq!(deleted.unwrap(), BatchOutcome::Done);
    let again = again.unwrap();
    let BatchOutcome::Partial(result) = again else {
        panic!("expected a 207, got {again:?}")
    };
    assert_eq!(result.success, Vec::<EntityId>::new(), "{result:?}");
    assert_eq!(sorted(result.errors.iter().map(|error| &error.entity_id)), sorted(&ids));
    assert!(
        result.errors.iter().all(|error| error.error.error_type == Some(ErrorType::ResourceNotFound)),
        "{result:?}"
    );
}
