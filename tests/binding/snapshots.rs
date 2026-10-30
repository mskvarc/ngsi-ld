//! The snapshot operations and the snapshot specification (clauses 5.16, 6.3.22 and 6.36 to 6.38).

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, link_of, response, vehicle, vehicle_id},
    links::{context_link, link_field, link_value},
    query_vehicles,
    retrieve_entity,
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, header};
use ngsi_ld::{
    http::{
        BatchOutcome,
        BodyError,
        ContextDelivery,
        RequestError,
        ResponseContext,
        ResponseError,
        headers::{NgsiLdSnapshot, NgsiLdTenant, OptionalHeader, RelationType},
        operation::{
            CloneSnapshot,
            CreateEntity,
            CreateSnapshot,
            CreateSubscription,
            DeleteSnapshot,
            DeleteTemporalEvolution,
            OnSnapshot,
            Operation,
            PurgeEntities,
            PurgeSnapshots,
            RetrieveEntityTypes,
            RetrieveSnapshot,
            Simplified,
            UpdateSnapshot,
        },
    },
    model::{ErrorType, Input, Lossless, MemberError, Output, Tenant},
    query::{SelectorType, q},
    snapshot::{SettledStatus, Snapshot, SnapshotClone, SnapshotFragment, SnapshotId, SnapshotMember, SnapshotStatus},
    subscription::{ReadErrorKind, Subscription},
};
use pretty_assertions::assert_eq;
use std::str;

/// `S`.
const S: &str = "urn:ngsi-ld:Snapshot:1";

/// `C`.
const C: &str = "urn:ngsi-ld:Snapshot:2";

/// `SNAP`.
const SNAP: &str = r#"{"id":"urn:ngsi-ld:Snapshot:1","type":"Snapshot","snapshotQueries":[{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50"}],"snapshotTemporalQueries":[{"type":"Query","entities":[{"type":"Vehicle"}],"temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"}}],"snapshotLifetime":"PT1H","snapshotPriority":7,"endpoint":"http://my.endpoint.org/snapshots","receiverInfo":[{"key":"Authorization","value":"Bearer t"}]}"#;

/// `OUT`.
const OUT: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Snapshot:1","type":"Snapshot","snapshotQueries":[{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50"}],"#,
    r#""snapshotTemporalQueries":[{"type":"Query","entities":[{"type":"Vehicle"}],"temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"}}],"#,
    r#""snapshotLifetime":"PT1H","snapshotPriority":7,"endpoint":"http://my.endpoint.org/snapshots","receiverInfo":[{"key":"Authorization","value":"Bearer t"}],"#,
    r#""snapshotStatus":"success","snapshotQueriesDetails":[{"resultStatus":"success"}],"#,
    r#""snapshotTemporalQueriesDetails":[{"problemDetails":{"type":"https://uri.etsi.org/ngsi-ld/errors/BadRequestData","title":"Bad temporal query"},"resultStatus":"failure"}],"#,
    r#""createdAt":"2024-05-01T10:00:00Z","modifiedAt":"2024-05-01T10:05:00Z","expiresAt":"2030-01-01T00:00:00Z","lastUsedAt":"2024-05-01T10:05:00Z","#,
    r#""@context":"https://example.org/context.jsonld"}"#
);

/// The body of `clone`.
const CLONE: &str = r#"{"id":"urn:ngsi-ld:Snapshot:2","type":"Snapshot","snapshotLifetime":"P1D","snapshotPriority":3}"#;

/// The body of `longer`.
const LONGER: &str = r#"{"type":"Snapshot","snapshotLifetime":"PT2H","endpoint":"urn:ngsi-ld:null"}"#;

/// `SNAP`, as a client sends it.
fn snap() -> Snapshot<Input> {
    Snapshot::parse(SNAP).unwrap()
}

/// A clone of `SNAP` as `C`, with its own lifetime and priority.
fn clone() -> SnapshotClone {
    SnapshotClone::builder()
        .id(C.parse::<SnapshotId>().unwrap())
        .lifetime("P1D".parse().unwrap())
        .priority(3)
        .build()
        .unwrap()
}

/// A fragment that lengthens the lifetime and deletes the endpoint.
fn longer() -> SnapshotFragment {
    SnapshotFragment::builder()
        .lifetime("PT2H".parse().unwrap())
        .delete(SnapshotMember::Endpoint)
        .build()
        .unwrap()
}

/// Parses a snapshot id.
fn id(text: &str) -> SnapshotId {
    text.parse().unwrap()
}

/// The text of a request body.
fn body_text(request: &Request<Bytes>) -> &str {
    str::from_utf8(request.body()).unwrap()
}

/// The type of the problem a response error reports.
fn problem_type(error: &ResponseError) -> Option<ErrorType> {
    let ResponseError::Problem { details, .. } = error else {
        panic!("expected a problem, got {error:?}")
    };
    details.error_type.clone()
}

/// The path of the snapshot `encoded` below the API root.
fn snapshot_path(encoded: &str) -> String {
    format!("/ngsi-ld/v1/snapshots/{encoded}")
}

#[test]
fn create_posts_the_snapshot_with_the_link_header_and_no_accept() {
    let request = CreateSnapshot { snapshot: &snap() }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/snapshots");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(SNAP.len().to_string().as_str()));
    assert_eq!(body_text(&request), SNAP);
}

#[test]
fn create_with_ld_json_body_puts_the_context_last_in_the_body() {
    let request = CreateSnapshot { snapshot: &snap() }
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let expected = format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, SNAP.trim_end_matches('}'));
    assert_eq!(body_text(&request), expected);
}

#[test]
fn clone_posts_the_clone_to_the_clone_resource() {
    let snapshot = id(S);
    let clone = clone();
    let request = CloneSnapshot { id: &snapshot, clone: &clone }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1/clone");
    assert_eq!(body_text(&request), CLONE);
}

#[test]
fn retrieve_gets_the_snapshot_as_json() {
    let snapshot = id(S);
    let request = RetrieveSnapshot { id: &snapshot }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), snapshot_path(S));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
}

#[test]
fn update_patches_the_fragment_and_asks_for_the_updated_snapshot() {
    let snapshot = id(S);
    let fragment = longer();
    let request = UpdateSnapshot {
        id: &snapshot,
        fragment: &fragment,
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), snapshot_path(S));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(body_text(&request), LONGER);
}

#[test]
fn delete_sends_no_body_and_no_accept() {
    let snapshot = id(S);
    let request = DeleteSnapshot { id: &snapshot }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), snapshot_path(S));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert!(request.body().is_empty());
}

#[test]
fn purge_deletes_the_snapshots_its_query_selects() {
    let q = q::attr("snapshotPriority").lt(3).build().unwrap();
    let request = PurgeSnapshots { q: &q }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/snapshots");
    assert_eq!(request.uri().query(), Some("q=snapshotPriority%3C3"));
    assert!(request.body().is_empty());
}

#[test]
fn a_snapshot_id_is_one_encoded_path_segment() {
    let snapshot = id("https://example.org/snapshots/1#a");
    let clone = clone();
    let encoded = "https:%2F%2Fexample.org%2Fsnapshots%2F1%23a";
    let paths = [
        RetrieveSnapshot { id: &snapshot }.to_request(&endpoint()).unwrap(),
        CloneSnapshot { id: &snapshot, clone: &clone }.to_request(&endpoint()).unwrap(),
        DeleteSnapshot { id: &snapshot }.to_request(&endpoint()).unwrap(),
    ]
    .map(|request| request.uri().path().to_owned());
    assert_eq!(
        paths,
        [snapshot_path(encoded), format!("{}/clone", snapshot_path(encoded)), snapshot_path(encoded)]
    );
}

#[test]
fn every_snapshot_operation_names_the_tenant_and_no_snapshot() {
    let tenant: Tenant = "t1".parse().unwrap();
    let snapshot = id(S);
    let (snap, clone, fragment) = (snap(), clone(), longer());
    let q = q::attr("snapshotPriority").lt(3).build().unwrap();
    let requests = [
        CreateSnapshot { snapshot: &snap }.to_request(&endpoint().with_tenant(tenant.clone())).unwrap(),
        CloneSnapshot { id: &snapshot, clone: &clone }
            .to_request(&endpoint().with_tenant(tenant.clone()))
            .unwrap(),
        RetrieveSnapshot { id: &snapshot }.to_request(&endpoint().with_tenant(tenant.clone())).unwrap(),
        UpdateSnapshot {
            id: &snapshot,
            fragment: &fragment,
        }
        .to_request(&endpoint().with_tenant(tenant.clone()))
        .unwrap(),
        DeleteSnapshot { id: &snapshot }.to_request(&endpoint().with_tenant(tenant.clone())).unwrap(),
        PurgeSnapshots { q: &q }.to_request(&endpoint().with_tenant(tenant)).unwrap(),
    ];
    for request in &requests {
        assert_eq!(header_value(request, NgsiLdTenant::name()), Some("t1"));
        assert_eq!(header_value(request, NgsiLdSnapshot::name()), None);
    }
}

#[test]
fn a_query_on_a_snapshot_is_the_query_plus_the_snapshot_header() {
    let tenant: Tenant = "t1".parse().unwrap();
    let snapshot = id(S);
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let plain = query.to_request(&endpoint().with_tenant(tenant.clone())).unwrap();
    let aimed = OnSnapshot::new(&snapshot, &query).to_request(&endpoint().with_tenant(tenant)).unwrap();
    assert_eq!(header_value(&aimed, NgsiLdSnapshot::name()), Some(S));
    assert_eq!(header_value(&aimed, NgsiLdTenant::name()), Some("t1"));
    assert_eq!(aimed.uri(), plain.uri());
    assert!(!aimed.uri().query().unwrap_or_default().contains("local"));
    let mut without = aimed.headers().clone();
    without.remove(NgsiLdSnapshot::name());
    assert_eq!(&without, plain.headers());
}

#[test]
fn each_api_of_the_core_and_temporal_apis_can_be_aimed_at_a_snapshot() {
    let snapshot = id(S);
    let vehicle = vehicle();
    let vehicle_id = vehicle_id();
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let subscription: Subscription<Input> =
        Subscription::parse(r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#)
            .unwrap();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let purge = PurgeEntities::builder().entity_type(&entity_type).build();
    let types = RetrieveEntityTypes::default();
    let subscribe = CreateSubscription { subscription: &subscription };
    let forget = DeleteTemporalEvolution { id: &vehicle_id };
    let pairs = [
        (
            create.to_request(&endpoint()).unwrap(),
            OnSnapshot::new(&snapshot, &create).to_request(&endpoint()).unwrap(),
        ),
        (
            purge.to_request(&endpoint()).unwrap(),
            OnSnapshot::new(&snapshot, &purge).to_request(&endpoint()).unwrap(),
        ),
        (
            types.to_request(&endpoint()).unwrap(),
            OnSnapshot::new(&snapshot, &types).to_request(&endpoint()).unwrap(),
        ),
        (
            subscribe.to_request(&endpoint()).unwrap(),
            OnSnapshot::new(&snapshot, &subscribe).to_request(&endpoint()).unwrap(),
        ),
        (
            forget.to_request(&endpoint()).unwrap(),
            OnSnapshot::new(&snapshot, &forget).to_request(&endpoint()).unwrap(),
        ),
    ];
    for (plain, aimed) in &pairs {
        assert_eq!(header_value(aimed, NgsiLdSnapshot::name()), Some(S));
        assert_eq!((aimed.method(), aimed.uri(), aimed.body()), (plain.method(), plain.uri(), plain.body()));
    }
}

#[test]
fn a_snapshot_id_a_header_cannot_carry_is_refused_before_sending() {
    let snapshot = id("urn:ngsi-ld:Snapshot:é");
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let error = OnSnapshot::new(&snapshot, &query).to_request(&endpoint()).unwrap_err();
    assert!(matches!(error, RequestError::Snapshot { .. }), "{error:?}");
}

#[test]
fn the_next_page_of_a_query_on_a_snapshot_stays_on_it() {
    let tenant: Tenant = "t1".parse().unwrap();
    let snapshot = id(S);
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let aimed = OnSnapshot::new(&snapshot, &query);
    let request = aimed.to_request(&endpoint().with_tenant(tenant)).unwrap();
    let link = link_field([link_value("/ngsi-ld/v1/entities?type=Vehicle&offset=1", RelationType::Next)]);
    let page = aimed
        .decode(&ResponseContext::new(&request), response(200, &[(&header::LINK, &link)], "[]"))
        .unwrap();
    assert_eq!(page.snapshot(), Some(&snapshot));
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(next.uri(), "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=1");
    assert_eq!(header_value(&next, NgsiLdSnapshot::name()), Some(S));
    assert_eq!(header_value(&next, NgsiLdTenant::name()), Some("t1"));
}

#[test]
fn the_first_and_last_pages_of_a_query_on_a_snapshot_stay_on_it_and_on_its_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let snapshot = id(S);
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let aimed = OnSnapshot::new(&snapshot, &query);
    let request = aimed.to_request(&endpoint().with_tenant(tenant)).unwrap();
    let link = link_field([
        link_value("/ngsi-ld/v1/entities?type=Vehicle&offset=0", RelationType::First),
        link_value("/ngsi-ld/v1/entities?type=Vehicle&offset=40", RelationType::Last),
    ]);
    let page = aimed
        .decode(&ResponseContext::new(&request), response(200, &[(&header::LINK, &link)], "[]"))
        .unwrap();
    let first = page.first_page().unwrap().to_request(&endpoint()).unwrap();
    let last = page.last_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(first.uri(), "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=0");
    assert_eq!(last.uri(), "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=40");
    for request in [&first, &last] {
        assert_eq!(request.method(), Method::GET);
        assert_eq!(header_value(request, NgsiLdSnapshot::name()), Some(S));
        assert_eq!(header_value(request, NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn create_reads_the_id_of_the_snapshot_from_its_location() {
    let snapshot = snap();
    let create = CreateSnapshot { snapshot: &snapshot };
    let created = create
        .decode(
            &context(),
            response(201, &[(&header::LOCATION, "/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1")], ""),
        )
        .unwrap();
    assert_eq!(created.id(), Some(id(S)));
    let unnamed = create.decode(&context(), response(201, &[], "")).unwrap();
    assert_eq!(unnamed.location, OptionalHeader::Absent);
    assert_eq!(unnamed.id(), None);
}

#[test]
fn clone_reads_the_id_of_the_clone_or_the_conflict() {
    let snapshot = id(S);
    let clone = clone();
    let operation = CloneSnapshot { id: &snapshot, clone: &clone };
    let created = operation
        .decode(
            &context(),
            response(201, &[(&header::LOCATION, "/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:2")], ""),
        )
        .unwrap();
    assert_eq!(created.id(), Some(id(C)));
    let problem = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/AlreadyExists","title":"Already exists"}"#;
    let error = operation.decode(&context(), response(409, &[], problem)).unwrap_err();
    assert_eq!(problem_type(&error), Some(ErrorType::AlreadyExists));
}

#[test]
fn retrieve_and_update_read_the_snapshot_they_answer_with() {
    let snapshot = id(S);
    let fragment = longer();
    let expected = Snapshot::<Output>::parse(OUT).unwrap();
    let retrieved = RetrieveSnapshot { id: &snapshot }.decode(&context(), response(200, &[], OUT)).unwrap();
    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.status(), SnapshotStatus::Settled(SettledStatus::Success));
    let updated = UpdateSnapshot {
        id: &snapshot,
        fragment: &fragment,
    }
    .decode(&context(), response(200, &[], OUT))
    .unwrap();
    assert_eq!(updated, expected);
}

#[test]
fn a_retrieved_snapshot_without_its_status_or_a_missing_one_is_an_error() {
    let snapshot = id(S);
    let error = RetrieveSnapshot { id: &snapshot }.decode(&context(), response(200, &[], SNAP)).unwrap_err();
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Snapshot(error) = *source else {
        panic!("expected a snapshot error, got {source:?}")
    };
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "snapshotStatus" })),
        "{error:?}"
    );
    let problem = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"Not found"}"#;
    let error = RetrieveSnapshot { id: &snapshot }.decode(&context(), response(404, &[], problem)).unwrap_err();
    assert_eq!(problem_type(&error), Some(ErrorType::ResourceNotFound));
}

#[test]
fn update_and_retrieve_read_only_their_success_status() {
    let snapshot = id(S);
    let fragment = longer();
    let update = UpdateSnapshot {
        id: &snapshot,
        fragment: &fragment,
    }
    .decode(&context(), response(204, &[], ""))
    .unwrap_err();
    assert!(matches!(update, ResponseError::UnexpectedStatus { .. }), "{update:?}");
    let retrieve = RetrieveSnapshot { id: &snapshot }.decode(&context(), response(201, &[], OUT)).unwrap_err();
    assert!(matches!(retrieve, ResponseError::UnexpectedStatus { .. }), "{retrieve:?}");
}

#[test]
fn delete_reads_no_content_only() {
    let snapshot = id(S);
    let delete = DeleteSnapshot { id: &snapshot };
    delete.decode(&context(), response(204, &[], "")).unwrap();
    let error = delete.decode(&context(), response(200, &[], "")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
}

#[test]
fn purge_reads_every_snapshot_it_could_not_delete() {
    let q = q::attr("snapshotPriority").lt(3).build().unwrap();
    let purge = PurgeSnapshots { q: &q };
    assert_eq!(purge.decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
    let body = r#"{"success":["urn:ngsi-ld:Snapshot:1"],"errors":[{"entityId":"urn:ngsi-ld:Snapshot:2","error":{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound"}}]}"#;
    let BatchOutcome::Partial(result) = purge.decode(&context(), response(207, &[], body)).unwrap() else {
        panic!("expected a partial outcome")
    };
    assert_eq!((result.success.len(), result.errors.len()), (1, 1));
    let problem = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound"}"#;
    let error = purge.decode(&context(), response(404, &[], problem)).unwrap_err();
    assert_eq!(problem_type(&error), Some(ErrorType::ResourceNotFound));
}

#[test]
fn an_operation_on_a_snapshot_reads_what_the_operation_reads() {
    let snapshot = id(S);
    let vehicle_id = vehicle_id();
    let retrieve = retrieve_entity(&vehicle_id, Lossless::Normalized);
    let body = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80}}"#;
    let aimed = OnSnapshot::new(&snapshot, &retrieve).decode(&context(), response(200, &[], body)).unwrap();
    let plain = retrieve.decode(&context(), response(200, &[], body)).unwrap();
    assert_eq!(aimed, plain);
}
