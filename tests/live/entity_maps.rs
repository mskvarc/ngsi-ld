//! Entity maps and the identity of the broker: Retrieve, Update and Delete EntityMap, Create EntityMap for Query
//! Entities and for Query Temporal Evolution, and Retrieve Context Source Identity Information (clauses 6.32 to
//! 6.35).

use crate::broker::{client, create_fleet, remove_all, unique};
use jiff::SignedDuration;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{
        headers::NgsiLdEntityMap,
        operation::{EntityMapRequest, QueryMethod},
    },
    model::{DateTime, Entity, EntityId, ErrorType, Input, Property, evolution::TemporalEntity},
    query::temporal_query::TemporalQuery,
    registration::{EntityMap, EntityMapId},
};
use pretty_assertions::assert_eq;

/// Whether `map` holds every entity of `entities`.
fn holds_all(map: &EntityMap, entities: &[Entity<Input>]) -> bool {
    entities.iter().all(|entity| map.entity_map.contains_key(&entity.id))
}

/// Deletes the map `id`; it must still be there.
async fn delete(client: &Client<ReqwestTransport>, id: &EntityMapId) {
    client.entity_maps().delete(id).send().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_query_names_a_map_that_can_be_retrieved_updated_and_deleted() {
    let client = client();
    let (vehicle_type, vehicles) = create_fleet(&client, "Mapped", 2).await;
    let request = EntityMapRequest::returned_for("PT1H".parse().unwrap());
    let page = client
        .entities()
        .query()
        .entity_type(vehicle_type.as_str())
        .entity_map(&request)
        .send()
        .await
        .unwrap();
    // Clause 6.4.3.2 returns the location of the map the query used when it asked for one.
    let id = page.entity_map().valid().and_then(NgsiLdEntityMap::id).unwrap();
    // Known deviation: Table 6.2-1 places the map at `/entityMaps/{entityMapId}` (clause 6.32.3.1); the broker
    // answers 405 there, and serves the map at `/entityMap/{entityMapId}` instead, without the
    // `linkedMaps` member Table 5.2.39-2 makes mandatory.
    let map = client.entity_maps().retrieve(&id).send().await.unwrap();
    assert_eq!(map.id, id);
    assert!(holds_all(&map, &vehicles), "{map:?}");
    let later: DateTime = "2099-01-01T00:00:00Z".parse().unwrap();
    client.entity_maps().update(&id, &later).send().await.unwrap();
    assert_eq!(client.entity_maps().retrieve(&id).send().await.unwrap().expires_at, later);
    delete(&client, &id).await;
    let gone = client.entity_maps().retrieve(&id).send().await.unwrap_err();
    assert_eq!(gone.error_type(), Some(&ErrorType::ResourceNotFound), "{gone:?}");
    remove_all(&client, &vehicles).await;
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_map_created_for_a_query_by_get_and_by_post_pages_the_entities() {
    let client = client();
    let (vehicle_type, vehicles) = create_fleet(&client, "Pinned", 2).await;
    for method in [QueryMethod::Get, QueryMethod::Post] {
        // Known deviation: clauses 6.34.3.1 and 6.34.3.2 define `GET` and `POST /entityMaps`; the broker
        // answers both with 405.
        let created = client
            .entity_maps()
            .create()
            .entity_type(vehicle_type.as_str())
            .method(method)
            .send()
            .await
            .unwrap();
        assert!(holds_all(created.body(), &vehicles), "{method:?}: {created:?}");
        // Tables 6.34.3.1-1 and 6.34.3.2-1 require the location of the created map in `NGSILD-EntityMap`.
        let location = created.entity_map().valid().cloned().unwrap();
        assert_eq!(location.id().as_ref(), Some(&created.body().id));
        let reuse = EntityMapRequest::reuse(location);
        let page = client
            .entities()
            .query()
            .entity_type(vehicle_type.as_str())
            .entity_map(&reuse)
            .send()
            .await
            .unwrap();
        assert_eq!(page.items().len(), vehicles.len(), "{method:?}");
        delete(&client, &created.body().id).await;
    }
    remove_all(&client, &vehicles).await;
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_map_created_for_a_temporal_query_holds_the_recorded_entity() {
    let client = client();
    let vehicle_type = unique("Recorded");
    let id: EntityId = format!("urn:ngsi-ld:Vehicle:{vehicle_type}").parse().unwrap();
    let recorded = TemporalEntity::builder()
        .id(id.as_str())
        .entity_type(vehicle_type.as_str())
        .property("speed", Property::builder(10).observed_at("2018-08-01T12:01:00Z".parse().unwrap()))
        .property("speed", Property::builder(20).observed_at("2018-08-01T12:02:00Z".parse().unwrap()))
        .build()
        .unwrap();
    client.temporal().create_or_update(&recorded).send().await.unwrap();
    let whole_day = TemporalQuery::between("2018-08-01T00:00:00Z".parse().unwrap(), "2018-08-02T00:00:00Z".parse().unwrap());
    // Known deviation: clauses 6.35.3.1 and 6.35.3.2 define `GET` and `POST /temporal/entityMaps`; the broker
    // answers both with 405.
    let created = client
        .entity_maps()
        .create_temporal(whole_day)
        .entity_type(vehicle_type.as_str())
        .send()
        .await
        .unwrap();
    assert!(created.body().entity_map.contains_key(&id), "{created:?}");
    delete(&client, &created.body().id).await;
    client.temporal().delete(&id).send().await.unwrap();
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn the_broker_names_itself() {
    let client = client();
    let started = DateTime::now();
    // Known deviation: clause 5.15.1.5 answers with a Context Source Identity (Table 5.2.40-1), or 501 when the
    // broker cannot supply one (clause 5.15.1.4); the broker answers 200 with a body that is not JSON,
    // its own URL followed by `)$%^&`.
    let identity = client.source_identity().send().await.unwrap();
    // The broker's clock (clause 4.8) may lag the test's a little.
    let earliest = started.timestamp().checked_sub(SignedDuration::from_mins(5)).unwrap();
    assert!(identity.time_at.timestamp() >= earliest, "{identity:?}");
}
