//! What a request may state besides its operation: the deprecated `attrs` (Tables 6.4.3.2-1 and 6.5.3.1-1), the
//! `Prefer` header (clause 6.3.6) and the `local` parameter on Retrieve Temporal Evolution (clause 6.3.18).

use crate::broker::{client, client_preferring, entity_id, name};
use ngsi_ld::{
    http::{
        Retrieved,
        headers::{Prefer, SpecVersion},
    },
    model::{
        DateTime,
        Entity,
        EntityId,
        Input,
        Property,
        evolution::{Evolution, TemporalEntity},
    },
    query::{Locality, temporal_query},
};
use pretty_assertions::assert_eq;

/// A vehicle with a `speed` and a `brandName`.
fn vehicle(id: &EntityId) -> Entity<Input> {
    Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("speed", 80)
        .property("brandName", "Mercedes")
        .build()
        .unwrap()
}

/// A time on the day the tests record.
fn at(minute: u32) -> DateTime {
    format!("2018-08-01T12:{minute:02}:00Z").parse().unwrap()
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn attrs_keeps_only_the_attributes_named() {
    let client = client();
    let id = entity_id("attrs");
    client.entities().create(&vehicle(&id)).send().await.unwrap();
    let retrieved = client.entities().retrieve(&id).attrs(vec![name("speed")]).send().await;
    let queried = client.entities().query().ids(vec![id.clone()]).attrs(vec![name("speed")]).send().await;
    client.entities().delete(&id).send().await.unwrap();
    let retrieved = retrieved.unwrap().into_body();
    let names: Vec<String> = retrieved.attributes.names().map(ToString::to_string).collect();
    assert_eq!(names, ["speed"]);
    let queried = queried.unwrap();
    assert_eq!(queried.items().len(), 1);
    assert!(queried.items()[0].attribute(&name("brandName")).is_none());
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_version_preference_is_confirmed_by_preference_applied() {
    let prefer = Prefer::builder().version(SpecVersion::new(1, 9)).build().unwrap();
    let client = client_preferring(Some(prefer));
    let id = entity_id("prefer");
    client.entities().create(&vehicle(&id)).send().await.unwrap();
    let retrieved = client.entities().retrieve(&id).send().await;
    client.entities().delete(&id).send().await.unwrap();
    // Clause 6.3.6: with `Prefer: ngsi-ld=<version>` the response carries `Preference-Applied:
    // ngsi-ld=<conformant-version>`. Observed deviation: the broker sends no `Preference-Applied`.
    assert!(retrieved.unwrap().preference_applied().is_some());
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_local_retrieve_of_a_temporal_evolution_reads_it() {
    let client = client();
    let id = entity_id("temporalLocal");
    let evolution = TemporalEntity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("speed", Property::builder(30).observed_at(at(3)))
        .property("speed", Property::builder(50).observed_at(at(5)))
        .build()
        .unwrap();
    client.temporal().create_or_update(&evolution).send().await.unwrap();
    let window = temporal_query::between(at(0), at(59));
    let local = client.temporal().retrieve(&id).temporal_q(window).locality(Locality::Local).send().await;
    client.temporal().delete(&id).send().await.unwrap();
    let local = local.map(Retrieved::into_body).unwrap();
    assert_eq!(local.attribute(&name("speed")).map(Evolution::len), Some(2));
}
