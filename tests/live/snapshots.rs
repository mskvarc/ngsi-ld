//! Snapshots: Create, Clone, Retrieve, Update and Delete Snapshot, Purge Snapshots, and the operations aimed at a
//! snapshot with `NGSILD-Snapshot` (clauses 5.16, 6.3.22 and 6.36 to 6.38).

use crate::broker::{client, create_fleet, remove_all, unique};
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::BatchOutcome,
    model::{BuildError, EntityFragment, ErrorType, Input, SimplifiedValue, Value},
    query::q,
    snapshot::{Problem, QueryDocument, SettledStatus, Snapshot, SnapshotClone, SnapshotFragment, SnapshotId, SnapshotStatus},
};
use pretty_assertions::assert_eq;
use std::time::Duration;
use tokio::time::sleep;

/// A snapshot of the entities of `vehicle_type`, with a priority.
fn snapshot_of(vehicle_type: &str, priority: u8) -> Result<Snapshot<Input>, BuildError<Problem>> {
    let vehicles = QueryDocument::builder().entities([vehicle_type]).build()?;
    Snapshot::builder().query(vehicles).lifetime("PT1H".parse().unwrap()).priority(priority).build()
}

/// Creates a snapshot of the entities of `vehicle_type` and returns its id.
async fn create_snapshot(client: &Client<ReqwestTransport>, vehicle_type: &str, priority: u8) -> SnapshotId {
    let snapshot = snapshot_of(vehicle_type, priority).unwrap();
    // Clause 6.36.3.1 answers 201 with the location of the snapshot, whose last segment is its id.
    // Known deviation: the broker answers `POST /snapshots` with 405; it does not implement the
    // Snapshot API, which clause 4.3.5 makes optional, while Table 6.2-1 defines the resource.
    let created = client.snapshots().create(&snapshot).send().await.unwrap();
    created.id().unwrap()
}

/// Waits until the snapshot `id` has run its queries (clause 5.16.1.4) and returns the status it settled in.
async fn settle(client: &Client<ReqwestTransport>, id: &SnapshotId) -> SnapshotStatus {
    let mut status = SnapshotStatus::Preparing;
    for _ in 0..20 {
        status = client.snapshots().retrieve(id).send().await.unwrap().status();
        if status != SnapshotStatus::Preparing {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
    status
}

/// Whether retrieving the snapshot `id` is `ResourceNotFound`.
async fn is_gone(client: &Client<ReqwestTransport>, id: &SnapshotId) -> bool {
    let error = client.snapshots().retrieve(id).send().await.unwrap_err();
    error.error_type() == Some(&ErrorType::ResourceNotFound)
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_snapshot_freezes_the_entities_it_queries() {
    let client = client();
    let (vehicle_type, vehicles) = create_fleet(&client, "Frozen", 2).await;
    let id = create_snapshot(&client, &vehicle_type, 5).await;
    assert_eq!(settle(&client, &id).await, SnapshotStatus::Settled(SettledStatus::Success));
    let stopped = EntityFragment::builder().property("speed", 0).build().unwrap();
    for vehicle in &vehicles {
        client.entities().merge(&vehicle.id, &stopped).send().await.unwrap();
    }
    // Clause 5.5.15 runs an operation aimed at a snapshot on the information of the snapshot only.
    let page = client
        .on_snapshot(&id)
        .entities()
        .query_simplified()
        .entity_type(vehicle_type.as_str())
        .send()
        .await
        .unwrap();
    assert_eq!(page.snapshot(), Some(&id));
    let speeds: Vec<&SimplifiedValue> = page
        .items()
        .iter()
        .filter_map(|vehicle| vehicle.attributes.iter().find(|(name, _)| name.as_ref() == "speed"))
        .map(|(_, value)| value)
        .collect();
    // The fleet's speeds are 10 and 20; the snapshot keeps them although every vehicle stopped since.
    assert_eq!(speeds.len(), 2, "{page:?}");
    assert!(!speeds.contains(&&SimplifiedValue::Value(Value::from(0))), "{speeds:?}");
    client.snapshots().delete(&id).send().await.unwrap();
    remove_all(&client, &vehicles).await;
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_snapshot_is_cloned_updated_and_deleted() {
    let client = client();
    let (vehicle_type, vehicles) = create_fleet(&client, "Cloned", 1).await;
    let id = create_snapshot(&client, &vehicle_type, 5).await;
    let clone_id: SnapshotId = format!("urn:ngsi-ld:Snapshot:{}", unique("Clone")).parse().unwrap();
    let clone = SnapshotClone::builder().id(clone_id.clone()).priority(3).build().unwrap();
    // Clause 6.38.3.1 answers 201 with the location of the clone.
    let cloned = client.snapshots().clone_snapshot(&id, &clone).send().await.unwrap();
    assert_eq!(cloned.id(), Some(clone_id.clone()));
    assert_eq!(client.snapshots().retrieve(&clone_id).send().await.unwrap().id(), &clone_id);
    // Clause 6.37.3.2 answers 200 with the updated snapshot.
    let longer = SnapshotFragment::builder().lifetime("PT2H".parse().unwrap()).build().unwrap();
    let updated = client.snapshots().update(&id, &longer).send().await.unwrap();
    assert_eq!(updated.id(), &id);
    client.snapshots().delete(&clone_id).send().await.unwrap();
    assert!(is_gone(&client, &clone_id).await);
    client.snapshots().delete(&id).send().await.unwrap();
    remove_all(&client, &vehicles).await;
}

#[tokio::test]
#[ignore = "needs a broker: set NGSI_LD_TEST_BROKER_URL"]
async fn snapshots_of_a_low_priority_are_purged() {
    let client = client();
    let (vehicle_type, vehicles) = create_fleet(&client, "Purged", 1).await;
    let id = create_snapshot(&client, &vehicle_type, 1).await;
    // Clause 6.36.3.2 answers 204, or 207 naming the snapshots it could not delete.
    let outcome = client.snapshots().purge(q::attr("snapshotPriority").lt(2)).send().await.unwrap();
    assert!(matches!(outcome, BatchOutcome::Done | BatchOutcome::Partial(_)), "{outcome:?}");
    assert!(is_gone(&client, &id).await);
    remove_all(&client, &vehicles).await;
}
