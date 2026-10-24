//! The `local` parameter of clause 6.3.18 on the operations whose tables do not list it, and the implicit locality of
//! an operation on a snapshot (clauses 5.5.15 and 6.3.18).

use crate::{
    fixtures::{endpoint, vehicle, vehicle_id},
    query_vehicles,
    retrieve_entity,
    temporal_fixtures::{b, query_string, request},
};
use headers::Header;
use ngsi_ld::{
    http::{
        RequestError,
        headers::NgsiLdSnapshot,
        operation::{ArgumentError, BatchDelete, CreateEntity, DeleteEntity, DeleteTemporalEvolution, OnSnapshot, Operation, PurgeEntities, WithLocality},
    },
    model::Lossless,
    query::{Locality, SelectorType},
    snapshot::SnapshotId,
};
use pretty_assertions::assert_eq;

/// The snapshot the operations are aimed at.
fn snapshot() -> SnapshotId {
    "urn:ngsi-ld:Snapshot:1".parse().unwrap()
}

/// A purge with no restriction of the entities.
fn unbounded_purge() -> PurgeEntities<'static> {
    PurgeEntities::builder().build()
}

#[test]
fn local_is_added_to_an_operation_without_parameters() {
    let id = vehicle_id();
    let delete = DeleteEntity::builder().id(&id).build();
    let local = WithLocality::new(Locality::Local, &delete);
    assert_eq!(
        request(&local).uri().path_and_query().map(ToString::to_string).as_deref(),
        Some("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567?local=true")
    );
}

#[test]
fn local_follows_the_parameters_of_the_operation_and_unspecified_adds_nothing() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Concise);
    assert_eq!(
        query_string(&WithLocality::new(Locality::Distributed, &retrieve)).as_deref(),
        Some("format=concise&local=false")
    );
    assert_eq!(
        query_string(&WithLocality::new(Locality::Unspecified, &retrieve)).as_deref(),
        Some("format=concise")
    );
}

#[test]
fn local_reaches_the_body_operations_and_the_temporal_api() {
    let entity = vehicle();
    let create = CreateEntity::builder().entity(&entity).build();
    let ids = [vehicle_id()];
    let batch = BatchDelete { ids: &ids };
    let forget_id = b();
    let forget = DeleteTemporalEvolution { id: &forget_id };
    for request in [
        request(&WithLocality::new(Locality::Local, &create)),
        request(&WithLocality::new(Locality::Local, &batch)),
        request(&WithLocality::new(Locality::Local, &forget)),
    ] {
        assert_eq!(request.uri().query(), Some("local=true"), "{}", request.uri());
    }
}

#[test]
fn a_query_or_purge_without_restriction_is_still_refused_off_a_snapshot() {
    let query = query_vehicles(None, None, Lossless::Normalized);
    assert!(matches!(
        query.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::UnboundedQuery))
    ));
    assert!(matches!(
        unbounded_purge().to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::UnboundedQuery))
    ));
}

#[test]
fn on_a_snapshot_a_query_or_purge_needs_no_restriction_and_sends_no_local() {
    let snapshot = snapshot();
    let query = query_vehicles(None, None, Lossless::Normalized);
    let queried = OnSnapshot::new(&snapshot, &query).to_request(&endpoint()).unwrap();
    assert_eq!(queried.uri().path_and_query().map(ToString::to_string).as_deref(), Some("/ngsi-ld/v1/entities"));
    assert_eq!(queried.headers()[NgsiLdSnapshot::name()], "urn:ngsi-ld:Snapshot:1");
    let purge = unbounded_purge();
    let purged = OnSnapshot::new(&snapshot, &purge).to_request(&endpoint()).unwrap();
    assert_eq!(purged.uri().query(), None);
}

#[test]
fn a_bounded_query_on_a_snapshot_is_sent_as_off_it() {
    let snapshot = snapshot();
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&vehicles), None, Lossless::Normalized);
    let on_snapshot = OnSnapshot::new(&snapshot, &query).to_request(&endpoint()).unwrap();
    assert_eq!(on_snapshot.uri(), query.to_request(&endpoint()).unwrap().uri());
}
