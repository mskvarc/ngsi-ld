//! Append Attributes (clause 6.6.3.1), Update Attributes (clause 6.6.3.2), Partial Attribute Update (clause
//! 6.7.3.1), Replace Attribute (clause 6.7.3.3) and Delete Attribute (clause 6.7.3.2).

use crate::broker::{attribute_names, client, create, entity_id, name, property, property_value, remove};
use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::{
    http::{
        Retrieved,
        UpdateOutcome,
        UpdateResult,
        operation::{InstanceSelection, Overwrite},
    },
    model::{Attribute, AttributeName, DatasetId, Entity, EntityFragment, EntityId, ErrorType, Input, Instances, Output, Property, Value},
};
use pretty_assertions::assert_eq;

/// A vehicle with a `brandName` and a `speed` in km/h.
fn vehicle(id: &EntityId) -> Entity<Input> {
    Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", Property::builder(80).unit_code(UnitCode::Kmh))
        .build()
        .unwrap()
}

/// A vehicle whose `speed` has a default instance and the instances `gps` and `radar` (clause 4.5.5).
fn multi_instance_vehicle(id: &EntityId) -> Entity<Input> {
    Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("speed", 55)
        .property("speed", Property::builder(54).dataset_id(iri!("urn:ngsi-ld:dataset:gps")))
        .property("speed", Property::builder(56).dataset_id(iri!("urn:ngsi-ld:dataset:radar")))
        .build()
        .unwrap()
}

/// The dataset ids of the instances of `attribute`, the default instance as `None`.
fn dataset_ids<'e>(entity: &'e Entity<Output>, attribute: &str) -> Vec<Option<&'e DatasetId>> {
    let mut ids: Vec<Option<&DatasetId>> = entity
        .attribute(&name(attribute))
        .map(|instances| instances.iter().map(|instance| instance.meta().dataset_id.as_ref()).collect())
        .unwrap_or_default();
    ids.sort_unstable_by_key(|id| id.map(ToString::to_string));
    ids
}

/// Whether `name` is the term `term`, short or expanded under the core context's default vocabulary: a partial
/// result carries no `@context` (clause 6.3.6), so a broker may name the attribute either way.
fn names_term(name: &AttributeName, term: &str) -> bool {
    name.as_ref() == term || name.as_ref() == format!("https://uri.etsi.org/ngsi-ld/default-context/{term}")
}

/// Whether the partial result lists `term` as not updated.
fn not_updated(result: &UpdateResult, term: &str) -> bool {
    result.not_updated.iter().any(|details| names_term(&details.attribute_name, term))
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn append_attributes_adds_new_attributes_and_overwrites_existing_ones() {
    let client = client();
    let id = entity_id("append");
    create(&client, &vehicle(&id)).await;
    let fragment = EntityFragment::builder().property("color", "white").property("speed", 70).build().unwrap();
    let outcome = client.entities().append_attributes(&id, &fragment).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    let stored = stored.unwrap();
    assert_eq!(attribute_names(&stored), ["brandName", "color", "speed"]);
    assert_eq!(property_value(&stored, "color"), Some(&Value::from("white")));
    assert_eq!(property(&stored, "speed").unwrap().value(), &Value::from(70));
    assert_eq!(property(&stored, "speed").unwrap().unit_code(), None);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn append_attributes_without_overwrite_keeps_existing_ones_and_reports_them_not_updated() {
    // Known deviation, clause 6.6.3.1: with `options=noOverwrite` an attribute that exists is not appended, so
    // the answer is 207 with an UpdateResult listing it in `notUpdated` (clause 5.2.18). Observed: the broker
    // keeps the attribute, as it must, but answers 204.
    let client = client();
    let id = entity_id("appendKeep");
    create(&client, &vehicle(&id)).await;
    let fragment = EntityFragment::builder().property("mileage", 12000).property("speed", 0).build().unwrap();
    let outcome = client.entities().append_attributes(&id, &fragment).overwrite(Overwrite::Keep).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    let stored = stored.unwrap();
    assert_eq!(attribute_names(&stored), ["brandName", "mileage", "speed"]);
    assert_eq!(property_value(&stored, "speed"), Some(&Value::from(80)));
    assert_eq!(property_value(&stored, "mileage"), Some(&Value::from(12000)));
    match outcome.unwrap() {
        UpdateOutcome::Partial(result) => assert!(not_updated(&result, "speed"), "{result:?}"),
        UpdateOutcome::Done => panic!("an attribute that was kept is reported as appended"),
    }
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn update_attributes_changes_attributes_the_entity_has() {
    let client = client();
    let id = entity_id("update");
    create(&client, &vehicle(&id)).await;
    let fragment = EntityFragment::builder().property("brandName", "Mercedes-Benz").build().unwrap();
    let outcome = client.entities().update_attributes(&id, &fragment).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    assert_eq!(property_value(&stored.unwrap(), "brandName"), Some(&Value::from("Mercedes-Benz")));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn update_attributes_appends_attributes_the_entity_lacks() {
    // Clause 5.6.2.4: an attribute of the fragment that the target entity does not include "shall be appended".
    let client = client();
    let id = entity_id("updateMissingAttribute");
    create(&client, &vehicle(&id)).await;
    let fragment = EntityFragment::builder().property("speed", 70).property("color", "red").build().unwrap();
    let outcome = client.entities().update_attributes(&id, &fragment).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    let stored = stored.unwrap();
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    assert_eq!(property(&stored, "speed").unwrap().value(), &Value::from(70));
    assert_eq!(property(&stored, "color").unwrap().value(), &Value::from("red"));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn update_attributes_of_a_missing_entity_is_resource_not_found() {
    let fragment = EntityFragment::builder().property("speed", 70).build().unwrap();
    let error = client()
        .entities()
        .update_attributes(&entity_id("updateMissingEntity"), &fragment)
        .send()
        .await
        .unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn partial_attribute_update_changes_the_members_it_names_and_keeps_the_others() {
    // Known deviation, clause 6.7.3.1: the fragment changes only the members it names. Observed: the broker
    // replaces the whole attribute, so the `unitCode` the fragment does not name is lost.
    let client = client();
    let id = entity_id("partialUpdate");
    create(&client, &vehicle(&id)).await;
    let observed_at = "2024-05-01T08:30:00Z".parse().unwrap();
    let patch = Attribute::from(Property::builder(90).observed_at(observed_at).build().unwrap()).into_patch();
    let outcome = client.entities().update_attribute(&id, &name("speed"), patch).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    let stored = stored.unwrap();
    let speed = property(&stored, "speed").unwrap();
    assert_eq!(speed.value(), &Value::from(90));
    assert_eq!(speed.unit_code(), Some(&UnitCode::Kmh));
    assert_eq!(speed.meta().observed_at, Some(observed_at));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn partial_attribute_update_of_a_missing_attribute_is_resource_not_found() {
    // Known deviation, clause 6.7.3.1: an attribute the entity lacks is answered with 404 ResourceNotFound.
    // Observed: the broker answers 204 and changes nothing.
    let client = client();
    let id = entity_id("partialUpdateMissing");
    create(&client, &vehicle(&id)).await;
    let patch = Attribute::from(Property::builder("red").build().unwrap()).into_patch();
    let outcome = client.entities().update_attribute(&id, &name("color"), patch).send().await;
    remove(&client, &id).await;
    let error = outcome.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn replace_attribute_replaces_the_whole_instance() {
    // Known deviation, clause 6.7.3.3: the attribute is replaced by the one sent. Observed: the broker rejects
    // every Replace Attribute request with 400 BadRequestData, detail "resolved size 1".
    let client = client();
    let id = entity_id("replaceAttribute");
    create(&client, &vehicle(&id)).await;
    let replacement: Attribute<Input> = Property::builder(50).property("source", "radar").build().unwrap().into();
    let outcome = client.entities().replace_attribute(&id, &name("speed"), &replacement).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    let stored = stored.unwrap().into_input().unwrap();
    assert_eq!(stored.attribute(&name("speed")), Some(&Instances::one(replacement)));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn replace_attribute_of_a_missing_attribute_is_resource_not_found() {
    // Known deviation, clause 6.7.3.3: an attribute the entity lacks is answered with 404 ResourceNotFound.
    // Observed: the broker answers 400 BadRequestData, detail "resolved size 1", as for every Replace Attribute.
    let client = client();
    let id = entity_id("replaceAttributeMissing");
    create(&client, &vehicle(&id)).await;
    let replacement: Attribute<Input> = Property::builder("red").build().unwrap().into();
    let outcome = client.entities().replace_attribute(&id, &name("color"), &replacement).send().await;
    remove(&client, &id).await;
    let error = outcome.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn delete_attribute_removes_the_attribute() {
    let client = client();
    let id = entity_id("deleteAttribute");
    create(&client, &vehicle(&id)).await;
    let outcome = client.entities().delete_attribute(&id, &name("brandName")).send().await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    assert_eq!(attribute_names(&stored.unwrap()), ["speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn delete_attribute_with_a_dataset_id_removes_only_that_instance() {
    // Known deviation, clause 6.7.3.2: `datasetId` selects the instance to delete. Observed: the broker answers
    // 404 ResourceNotFound, "Attribute .../speed with datasetId urn:ngsi-ld:dataset:gps ... was not found", for an
    // instance it returns on Retrieve Entity.
    let client = client();
    let id = entity_id("deleteDataset");
    create(&client, &multi_instance_vehicle(&id)).await;
    let gps = DatasetId::from(iri!("urn:ngsi-ld:dataset:gps"));
    let outcome = client
        .entities()
        .delete_attribute(&id, &name("speed"))
        .instances(InstanceSelection::Dataset(&gps))
        .send()
        .await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    let radar = DatasetId::from(iri!("urn:ngsi-ld:dataset:radar"));
    assert_eq!(dataset_ids(&stored.unwrap(), "speed"), [None, Some(&radar)]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn delete_attribute_with_delete_all_removes_every_instance() {
    let client = client();
    let id = entity_id("deleteAllInstances");
    create(&client, &multi_instance_vehicle(&id)).await;
    let outcome = client
        .entities()
        .delete_attribute(&id, &name("speed"))
        .instances(InstanceSelection::All)
        .send()
        .await;
    let stored = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    remove(&client, &id).await;
    assert_eq!(outcome.unwrap(), UpdateOutcome::Done);
    assert_eq!(attribute_names(&stored.unwrap()), Vec::<&str>::new());
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn deleting_a_missing_attribute_is_resource_not_found() {
    let client = client();
    let id = entity_id("deleteAttributeMissing");
    create(&client, &vehicle(&id)).await;
    let outcome = client.entities().delete_attribute(&id, &name("color")).send().await;
    remove(&client, &id).await;
    let error = outcome.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}
