//! Create Entity (clause 6.4.3.1), Retrieve Entity (clause 6.5.3.1) in each representation, Merge Entity (clause
//! 6.5.3.4) and Replace Entity (clause 6.5.3.3).

use crate::broker::{annex_vehicle, attribute_names, client, create, entity_id, every_kind_vehicle, has_location, parked_vehicle, property_value, remove};
use ngsi_ld::{
    http::{BatchOutcome, Retrieved, operation::SystemAttributes},
    model::{AttributeKind, Entity, EntityFragment, ErrorType, Lossless, SimplifiedEntity, Value},
    query::projection::{self, EntityMember},
};
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn the_annex_vehicle_is_created_and_retrieved_in_normalized_form_as_sent() {
    // Known deviation, Table 5.2.36-1 and clause 4.5.21: the `valueType` of a ListProperty is returned as it was
    // sent. Observed: the broker answers `"valueType": null`, which is no NGSI-LD value, so the entity cannot be
    // read back.
    let client = client();
    let id = entity_id("annexNormalized");
    let vehicle = annex_vehicle(&id);
    let outcome = client.entities().create(&vehicle).send().await.unwrap();
    assert!(outcome.is_created() && has_location(&outcome), "{outcome:?}");
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(stored.unwrap().into_input().unwrap(), vehicle);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn the_annex_vehicle_is_retrieved_in_concise_form_as_sent() {
    // Known deviation, Table 5.2.36-1 and clause 4.5.21: as in the normalized test, the broker answers
    // `"valueType": null` for the ListProperty.
    let client = client();
    let id = entity_id("annexConcise");
    let vehicle = annex_vehicle(&id);
    create(&client, &vehicle).await;
    let stored = client.entities().retrieve(&id).form(Lossless::Concise).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(stored.unwrap().into_input().unwrap(), vehicle);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn every_attribute_kind_is_retrieved_in_normalized_and_concise_form_as_sent() {
    let client = client();
    let id = entity_id("everyKind");
    let vehicle = every_kind_vehicle(&id);
    create(&client, &vehicle).await;
    let normalized = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    let concise = client.entities().retrieve(&id).form(Lossless::Concise).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(normalized.unwrap().into_input().unwrap(), vehicle);
    assert_eq!(concise.unwrap().into_input().unwrap(), vehicle);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn every_attribute_kind_is_retrieved_in_simplified_form_as_clause_4_5_4_writes_it() {
    // Known deviation, clause 4.5.4 with clauses 4.5.18, 4.5.20 and 4.5.22: in simplified form a LanguageProperty
    // is `{"languageMap": {...}}`, a VocabProperty `{"vocab": ...}` and a ListRelationship the array of its
    // objects. Observed: the broker answers the bare language map, the bare term, and an array of
    // `{"object": ...}` objects.
    let client = client();
    let id = entity_id("everyKindSimplified");
    let vehicle = every_kind_vehicle(&id);
    create(&client, &vehicle).await;
    let stored = client.entities().retrieve_simplified(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(stored.unwrap(), SimplifiedEntity::from(&vehicle));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieve_with_pick_returns_only_the_picked_members() {
    let client = client();
    let id = entity_id("pick");
    create(&client, &parked_vehicle(&id)).await;
    let stored = client
        .entities()
        .retrieve(&id)
        .pick(projection::member(EntityMember::Id).member(EntityMember::Type).attribute("brandName"))
        .send()
        .await
        .map(Retrieved::into_body);
    remove(&client, &id).await;
    let stored = stored.unwrap();
    assert_eq!(stored.id, id);
    assert_eq!(attribute_names(&stored), ["brandName"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieve_with_system_attributes_returns_created_at_and_modified_at() {
    let client = client();
    let id = entity_id("sysAttrs");
    create(&client, &parked_vehicle(&id)).await;
    let stored = client
        .entities()
        .retrieve(&id)
        .system_attributes(SystemAttributes::Include)
        .send()
        .await
        .map(Retrieved::into_body);
    remove(&client, &id).await;
    let stored = stored.unwrap();
    assert!(stored.output.created_at.is_some(), "{stored:?}");
    assert!(stored.output.modified_at.is_some(), "{stored:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn creating_an_entity_that_exists_is_already_exists() {
    let client = client();
    let id = entity_id("duplicate");
    let vehicle = parked_vehicle(&id);
    create(&client, &vehicle).await;
    let error = client.entities().create(&vehicle).send().await.unwrap_err();
    remove(&client, &id).await;
    assert_eq!(error.error_type(), Some(&ErrorType::AlreadyExists), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieving_a_missing_entity_is_resource_not_found() {
    let error = client().entities().retrieve(&entity_id("missing")).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn merge_sets_an_attribute_and_deletes_one_written_as_the_bare_null() {
    // Known deviation, clauses 4.5.0, 4.5.3.3 and 5.4: a fragment deletes an attribute with the bare NGSI-LD Null,
    // `"isParked": "urn:ngsi-ld:null"`. Observed: the broker rejects the merge with BadRequestData, "The key
    // isParked is an invalid entry".
    let client = client();
    let id = entity_id("mergeBareNull");
    create(&client, &parked_vehicle(&id)).await;
    let fragment = EntityFragment::builder().property("speed", 50).delete("isParked").build().unwrap();
    let outcome = client.entities().merge(&id, &fragment).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    let stored = stored.unwrap();
    assert_eq!(property_value(&stored, "speed"), Some(&Value::from(50)));
    assert_eq!(attribute_names(&stored), ["brandName", "speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn merge_sets_an_attribute_and_deletes_one_written_as_the_null_of_its_kind() {
    let client = client();
    let id = entity_id("mergeKindNull");
    create(&client, &parked_vehicle(&id)).await;
    let fragment = EntityFragment::builder()
        .property("speed", 50)
        .delete_kind("isParked", AttributeKind::Relationship)
        .build()
        .unwrap();
    let outcome = client.entities().merge(&id, &fragment).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    let stored = stored.unwrap();
    assert_eq!(property_value(&stored, "speed"), Some(&Value::from(50)));
    assert_eq!(attribute_names(&stored), ["brandName", "speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn merging_into_a_missing_entity_is_resource_not_found() {
    let fragment = EntityFragment::builder().property("speed", 50).build().unwrap();
    let error = client().entities().merge(&entity_id("mergeMissing"), &fragment).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn replace_entity_drops_every_attribute_the_replacement_lacks() {
    let client = client();
    let id = entity_id("replace");
    create(&client, &parked_vehicle(&id)).await;
    let replacement = Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("brandName", "Volvo")
        .property("color", "red")
        .build()
        .unwrap();
    let outcome = client.entities().replace(&replacement).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert_eq!(stored.unwrap().into_input().unwrap(), replacement);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn replacing_a_missing_entity_is_resource_not_found() {
    let replacement = parked_vehicle(&entity_id("replaceMissing"));
    let error = client().entities().replace(&replacement).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}
