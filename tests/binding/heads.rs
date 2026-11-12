//! The HEAD request of a retrieval (ETSI TS 104 176 clause 5): the GET of the operation with the method HEAD, and its
//! answer, 204 No Content with the head of the GET's answer and no body.

use crate::{
    fixtures::{context, endpoint, response},
    query_vehicles,
    retrieve_entity,
    temporal_fixtures::window,
};
use headers::Header;
use http::{Method, StatusCode, header};
use ngsi_ld::{
    http::{
        RequestError,
        ResponseError,
        headers::{NgsiLdResultsCount, NgsiLdSnapshot, NgsiLdTenant},
        operation::{
            ArgumentError,
            Head,
            ListContexts,
            NormalizedTemporal,
            OnSnapshot,
            Operation,
            QueryEntities,
            QueryMethod,
            QueryTemporalEvolution,
            ResultsCount,
            RetrieveEntityTypes,
            RetrieveSubscription,
            ServeContextWithCore,
            WithLocality,
        },
    },
    model::{ConformanceVersion, ContextId, EntityId, Lossless, Tenant},
    query::{Locality, SelectorType},
    snapshot::SnapshotId,
    subscription::{EntitySubscription, SubscriptionId},
};
use pretty_assertions::assert_eq;

/// The vehicle the retrievals name.
fn vehicle_id() -> EntityId {
    "urn:ngsi-ld:Vehicle:A4567".parse().unwrap()
}

/// The entity type the queries name.
fn vehicles() -> SelectorType {
    "Vehicle".parse().unwrap()
}

#[test]
fn the_head_of_a_retrieval_is_its_get_with_the_method_head() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let get = retrieve.to_request(&endpoint()).unwrap();
    let head = Head::new(&retrieve).to_request(&endpoint()).unwrap();
    assert_eq!(get.method(), Method::GET);
    assert_eq!(head.method(), Method::HEAD);
    assert_eq!(head.uri(), get.uri());
    assert_eq!(head.headers(), get.headers());
    assert!(head.body().is_empty());
}

#[test]
fn the_head_of_a_query_keeps_its_parameters_and_tenant() {
    let entity_type = vehicles();
    let query = QueryEntities {
        count: ResultsCount::Include,
        limit: Some(0),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    let tenant: Tenant = "fleet".parse().unwrap();
    let head = Head::new(&query).to_request(&endpoint().with_tenant(tenant)).unwrap();
    assert_eq!(head.method(), Method::HEAD);
    assert_eq!(
        head.uri().path_and_query().map(ToString::to_string).as_deref(),
        Some("/ngsi-ld/v1/entities?type=Vehicle&limit=0&count=true")
    );
    assert_eq!(head.headers()[NgsiLdTenant::name()], "fleet");
}

#[test]
fn the_head_of_a_query_through_post_is_refused() {
    let entity_type = vehicles();
    let query = QueryEntities {
        method: QueryMethod::Post,
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    let window = window();
    let temporal = QueryTemporalEvolution::builder()
        .entity_type(&entity_type)
        .temporal_q(&window)
        .format(NormalizedTemporal)
        .method(QueryMethod::Post)
        .build();
    for error in [
        Head::new(&query).to_request(&endpoint()).unwrap_err(),
        Head::new(&temporal).to_request(&endpoint()).unwrap_err(),
    ] {
        assert!(matches!(error, RequestError::Argument(ArgumentError::HeadWithoutGet)), "{error:?}");
    }
}

#[test]
fn a_head_aimed_at_a_snapshot_or_limited_to_local_scope_keeps_both() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let snapshot: SnapshotId = "urn:ngsi-ld:Snapshot:1".parse().unwrap();
    let on_snapshot = OnSnapshot::new(&snapshot, &retrieve);
    let head = Head::new(&on_snapshot).to_request(&endpoint()).unwrap();
    assert_eq!(head.method(), Method::HEAD);
    assert_eq!(head.headers()[NgsiLdSnapshot::name()], "urn:ngsi-ld:Snapshot:1");
    let local = WithLocality::new(Locality::Local, &retrieve);
    let head = Head::new(&local).to_request(&endpoint()).unwrap();
    assert_eq!(head.method(), Method::HEAD);
    assert_eq!(head.uri().query(), Some("local=true"));
}

#[test]
fn the_heads_of_the_other_apis_mirror_their_gets() {
    let id: SubscriptionId = "urn:ngsi-ld:Subscription:1".parse().unwrap();
    let subscription = RetrieveSubscription::builder().id(&id).kind(EntitySubscription).build();
    let types = RetrieveEntityTypes::default();
    let contexts = ListContexts::builder().build();
    let context_id: ContextId = "urn:ngsi-ld:context:c1".parse().unwrap();
    let with_core = ServeContextWithCore {
        id: &context_id,
        core: ConformanceVersion::new(1, 9),
    };
    let requests = [
        (
            subscription.to_request(&endpoint()).unwrap(),
            Head::new(&subscription).to_request(&endpoint()).unwrap(),
        ),
        (types.to_request(&endpoint()).unwrap(), Head::new(&types).to_request(&endpoint()).unwrap()),
        (contexts.to_request(&endpoint()).unwrap(), Head::new(&contexts).to_request(&endpoint()).unwrap()),
        (
            with_core.to_request(&endpoint()).unwrap(),
            Head::new(&with_core).to_request(&endpoint()).unwrap(),
        ),
    ];
    for (get, head) in requests {
        assert_eq!(head.method(), Method::HEAD);
        assert_eq!(head.uri(), get.uri());
        assert_eq!(head.headers(), get.headers());
    }
}

#[test]
fn a_204_answer_is_the_head_of_the_response() {
    let entity_type = vehicles();
    let query = query_vehicles(Some(&entity_type), None, Lossless::Normalized);
    let head = Head::new(&query)
        .decode(
            &context(),
            response(
                204,
                &[
                    (NgsiLdResultsCount::name(), "42"),
                    (&header::LINK, r#"</ngsi-ld/v1/entities?type=Vehicle&offset=10>; rel="next""#),
                ],
                "",
            ),
        )
        .unwrap();
    assert_eq!(head.typed::<NgsiLdResultsCount>().into_valid().map(NgsiLdResultsCount::count), Some(42));
    assert_eq!(head.headers()[&header::LINK], r#"</ngsi-ld/v1/entities?type=Vehicle&offset=10>; rel="next""#);
}

#[test]
fn a_200_answer_to_a_head_is_not_defined() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let error = Head::new(&retrieve).decode(&context(), response(200, &[], "")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { status: StatusCode::OK, .. }), "{error:?}");
}

#[test]
fn an_error_answer_to_a_head_is_a_status_without_a_body() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let error = Head::new(&retrieve)
        .decode(&context(), response(404, &[(NgsiLdTenant::name(), "fleet")], ""))
        .unwrap_err();
    assert!(
        matches!(
            error,
            ResponseError::Status {
                status: StatusCode::NOT_FOUND,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.head().headers()[NgsiLdTenant::name()], "fleet");
}
