//! The Temporal API: Create or Update, Add Attributes, Delete Attribute, Modify and Delete Attribute Instance,
//! Delete, Retrieve and Query Temporal Evolution (clauses 6.18 to 6.22 and 6.24).

use crate::broker::{client, entity_id, name, unique};
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{Retrieved, TemporalUpsertOutcome, operation::InstanceSelection},
    model::{
        Attribute,
        DateTime,
        EntityId,
        ErrorType,
        Input,
        InstanceId,
        Lossless,
        Output,
        Property,
        evolution::{Evolution, TemporalAttributes, TemporalEntity, TemporalValues},
    },
    query::temporal_query::{self, AggregationMethods, TemporalQuery},
};
use pretty_assertions::assert_eq;

/// A time on the day the tests record.
fn at(minute: u32) -> DateTime {
    format!("2018-08-01T12:{minute:02}:00Z").parse().unwrap()
}

/// The range every recorded instance falls in.
fn whole_day() -> TemporalQuery {
    TemporalQuery::between("2018-08-01T00:00:00Z".parse().unwrap(), "2018-08-02T00:00:00Z".parse().unwrap())
}

/// An evolution of `id`, of type `entity_type`, whose `speed` is recorded at each of `minutes`, its value the
/// minute times ten.
fn recorded(id: &EntityId, entity_type: &str, minutes: &[u32]) -> TemporalEntity<Input> {
    // The builder owns the id of the document it builds; the caller keeps its own to address the evolution.
    minutes
        .iter()
        .fold(TemporalEntity::builder().id(id.clone()).entity_type(entity_type), |builder, &minute| {
            builder.property("speed", Property::builder(i64::from(minute) * 10).observed_at(at(minute)))
        })
        .build()
        .unwrap()
}

/// Records `entity`.
async fn record(client: &Client<ReqwestTransport>, entity: &TemporalEntity<Input>) {
    client.temporal().create_or_update(entity).send().await.unwrap();
}

/// Deletes the evolution of `id` if it is still there.
async fn forget(client: &Client<ReqwestTransport>, id: &EntityId) {
    match client.temporal().delete(id).send().await {
        Ok(()) => {}
        Err(error) => assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}"),
    }
}

/// The evolution of `id` over the whole day, normalized.
async fn stored(client: &Client<ReqwestTransport>, id: &EntityId) -> TemporalEntity<Output> {
    client
        .temporal()
        .retrieve(id)
        .temporal_q(whole_day())
        .send()
        .await
        .map(Retrieved::into_body)
        .unwrap()
}

/// The evolution of `rpm`, recorded at each of `minutes`.
fn rpm(minutes: &[u32]) -> TemporalAttributes<Input> {
    let mut attributes = TemporalAttributes::<Input>::new();
    for &minute in minutes {
        let instance = Property::builder(1000 + i64::from(minute)).observed_at(at(minute)).build().unwrap();
        attributes.insert(name("rpm"), instance.into()).unwrap();
    }
    attributes
}

/// The number of instances of `attribute` in `entity`.
fn instances(entity: &TemporalEntity<Output>, attribute: &str) -> usize {
    entity.attribute(&name(attribute)).map_or(0, Evolution::len)
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn create_or_update_creates_an_evolution_then_adds_to_it() {
    let client = client();
    let id = entity_id("temporalUpsert");
    let created = client.temporal().create_or_update(&recorded(&id, "Vehicle", &[3, 5, 7])).send().await;
    let updated = client.temporal().create_or_update(&recorded(&id, "Vehicle", &[9])).send().await;
    let entity = client.temporal().retrieve(&id).temporal_q(whole_day()).send().await;
    forget(&client, &id).await;
    assert!(matches!(created.unwrap(), TemporalUpsertOutcome::Created { .. }));
    assert_eq!(updated.unwrap(), TemporalUpsertOutcome::Updated);
    assert_eq!(instances(entity.unwrap().body(), "speed"), 4);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieve_without_a_temporal_query_reads_the_last_instances() {
    let client = client();
    let id = entity_id("temporalLastN");
    record(&client, &recorded(&id, "Vehicle", &[3, 5, 7])).await;
    let last_two = client.temporal().retrieve(&id).last_n(2).send().await;
    forget(&client, &id).await;
    assert_eq!(instances(last_two.unwrap().body(), "speed"), 2);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieve_reads_the_simplified_temporal_representation() {
    let client = client();
    let id = entity_id("temporalValues");
    record(&client, &recorded(&id, "Vehicle", &[3, 5, 7])).await;
    let values = client.temporal().retrieve_values(&id).temporal_q(whole_day()).send().await;
    forget(&client, &id).await;
    // Known deviation: Table 6.3.12-1 asks for this representation with `format=temporalValues`; the broker answers
    // it with a 500 `InternalError`, and serves it only for the `options=temporalValues` the table deprecates.
    let values = values.unwrap().into_body();
    assert_eq!(values.attributes.get(&name("speed")).map(TemporalValues::len), Some(3));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieve_reads_the_aggregated_temporal_representation() {
    let client = client();
    let id = entity_id("temporalAggregated");
    record(&client, &recorded(&id, "Vehicle", &[3, 5, 7])).await;
    let max: AggregationMethods = "max".parse().unwrap();
    let aggregated = client
        .temporal()
        .retrieve_aggregated(&id)
        .temporal_q(whole_day().with_aggregation_methods(max))
        .send()
        .await;
    forget(&client, &id).await;
    // Known deviation: Table 6.3.12-1 asks for this representation with `format=aggregatedValues`; the broker
    // answers it with a 500 `InternalError`, and serves it only for the `options=aggregatedValues` the table
    // deprecates.
    let aggregated = aggregated.unwrap().into_body();
    assert!(aggregated.attributes.contains_key(&name("speed")), "{aggregated:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieve_reads_the_concise_temporal_representation() {
    let client = client();
    let id = entity_id("temporalConcise");
    record(&client, &recorded(&id, "Vehicle", &[3, 5])).await;
    let concise = client.temporal().retrieve(&id).temporal_q(whole_day()).form(Lossless::Concise).send().await;
    forget(&client, &id).await;
    // Known deviation: clause 6.3.7 asks for the concise representation with `format=concise`; on a temporal
    // resource the broker answers it with a 500 `InternalError`.
    assert_eq!(instances(concise.unwrap().body(), "speed"), 2);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_selects_the_evolutions_of_a_type_within_a_time_range() {
    let client = client();
    let entity_type = unique("TemporalVehicle");
    let ids = [entity_id("temporalQueryA"), entity_id("temporalQueryB")];
    for id in &ids {
        record(&client, &recorded(id, &entity_type, &[3, 5])).await;
    }
    let page = client
        .temporal()
        .query(temporal_query::between(at(4), at(30)))
        .entity_type(entity_type.as_str())
        .send()
        .await;
    for id in &ids {
        forget(&client, id).await;
    }
    let page = page.unwrap();
    let mut found: Vec<String> = page.items().iter().map(|entity| entity.id.to_string()).collect();
    found.sort_unstable();
    let mut expected: Vec<String> = ids.iter().map(ToString::to_string).collect();
    expected.sort_unstable();
    assert_eq!(found, expected);
    // Only the instance at minute 5 falls in the range (clause 4.11).
    assert!(page.items().iter().all(|entity| instances(entity, "speed") == 1), "{page:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn an_instance_is_modified_then_deleted_by_its_instance_id() {
    let client = client();
    let id = entity_id("temporalInstance");
    record(&client, &recorded(&id, "Vehicle", &[3, 5])).await;
    let before = stored(&client, &id).await;
    let first = before.attribute(&name("speed")).and_then(|evolution| evolution.iter().next());
    let instance_id: Option<InstanceId> = first.and_then(|instance| instance.meta().output.instance_id.clone());
    let Some(instance_id) = instance_id else {
        forget(&client, &id).await;
        panic!("clause 4.5.7 gives every instance of a temporal representation an instanceId: {before:?}")
    };
    let replacement: Attribute<Input> = Property::builder(99).observed_at(at(3)).build().unwrap().into();
    let modified = client.temporal().modify_instance(&id, &name("speed"), &instance_id, &replacement).send().await;
    let after_modify = stored(&client, &id).await;
    let deleted = client.temporal().delete_instance(&id, &name("speed"), &instance_id).send().await;
    let after_delete = stored(&client, &id).await;
    forget(&client, &id).await;
    modified.unwrap();
    deleted.unwrap();
    let rendered = after_modify.render(Lossless::Normalized).to_string();
    assert!(rendered.contains("\"value\":99"), "{rendered}");
    assert_eq!(instances(&after_delete, "speed"), 1);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn an_attribute_is_added_to_and_deleted_from_an_evolution() {
    let client = client();
    let id = entity_id("temporalAttributes");
    record(&client, &recorded(&id, "Vehicle", &[3])).await;
    let added = client.temporal().add_attributes(&id, &rpm(&[4])).send().await;
    let with_rpm = stored(&client, &id).await;
    let removed = client
        .temporal()
        .delete_attribute(&id, &name("rpm"))
        .instances(InstanceSelection::All)
        .send()
        .await;
    let without_rpm = stored(&client, &id).await;
    forget(&client, &id).await;
    added.unwrap();
    removed.unwrap();
    assert_eq!(instances(&with_rpm, "rpm"), 1);
    assert_eq!(instances(&without_rpm, "rpm"), 0);
    assert_eq!(instances(&without_rpm, "speed"), 1);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn several_instances_of_an_attribute_are_added_at_once() {
    let client = client();
    let id = entity_id("temporalInstances");
    record(&client, &recorded(&id, "Vehicle", &[3])).await;
    let added = client.temporal().add_attributes(&id, &rpm(&[4, 6])).send().await;
    let with_rpm = stored(&client, &id).await;
    forget(&client, &id).await;
    // Known deviation: clause 5.6.12.4 adds every instance the fragment holds ("the Attribute instance(s) shall be
    // added"); the broker refuses two instances of the default dataset with a `BadRequestData`.
    added.unwrap();
    assert_eq!(instances(&with_rpm, "rpm"), 2);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_deleted_evolution_is_unknown() {
    let client = client();
    let id = entity_id("temporalDelete");
    record(&client, &recorded(&id, "Vehicle", &[3])).await;
    client.temporal().delete(&id).send().await.unwrap();
    let error = client.temporal().retrieve(&id).temporal_q(whole_day()).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}
