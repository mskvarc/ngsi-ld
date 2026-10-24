//! The deprecated `attrs` parameter (Tables 6.4.3.2-1, 6.5.3.1-1, 6.18.3.2-1, 6.19.3.1-1 and 5.2.23-1).

use crate::{
    fixtures::{endpoint, vehicle_id},
    query_vehicles,
    retrieve_entity,
    temporal_fixtures::{b, body_text, query_string, request, window},
};
use ngsi_ld::{
    http::operation::{Operation, QueryEntities, QueryMethod, QueryTemporalEvolution, RetrieveEntity, RetrieveTemporalEvolution, TemporalSelection},
    model::{AttributeName, Lossless, NameError, ReservedMember, evolution::TemporalAttributeName},
    query::temporal_query::TemporalQuery,
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

/// The attributes `speed` and `brandName`.
fn speed_and_brand() -> [AttributeName; 2] {
    ["speed".parse().unwrap(), "brandName".parse().unwrap()]
}

/// The attributes `speed` and `brandName`, as a temporal request names them.
fn temporal_speed_and_brand() -> [TemporalAttributeName; 2] {
    speed_and_brand().map(TemporalAttributeName::from)
}

/// The temporal query of `attrs` and `window` sent through `method`, with no other parameter.
fn temporal_query<'a>(attrs: &'a [TemporalAttributeName], window: &'a TemporalQuery, method: QueryMethod) -> QueryTemporalEvolution<'a> {
    QueryTemporalEvolution::builder()
        .attrs(attrs)
        .temporal_q(window)
        .method(method)
        .format(Lossless::Normalized)
        .build()
}

#[test]
fn a_get_query_with_attrs_alone_is_bounded_and_sends_them() {
    let attrs = speed_and_brand();
    let query = QueryEntities {
        attrs: &attrs,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert_eq!(query_string(&query).as_deref(), Some("attrs=speed,brandName"));
}

#[test]
fn a_post_query_carries_attrs_as_an_array_in_its_body() {
    let attrs = speed_and_brand();
    let query = QueryEntities {
        attrs: &attrs,
        method: QueryMethod::Post,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entityOperations/query");
    assert_eq!(body_text(&request), r#"{"type":"Query","attrs":["speed","brandName"]}"#);
}

#[test]
fn retrieve_entity_sends_attrs_after_the_type() {
    let id = vehicle_id();
    let attrs = speed_and_brand();
    let retrieve = RetrieveEntity {
        attrs: &attrs,
        ..retrieve_entity(&id, Lossless::Normalized)
    };
    assert_eq!(query_string(&retrieve).as_deref(), Some("attrs=speed,brandName"));
}

#[test]
fn retrieve_temporal_evolution_sends_attrs_before_the_projection() {
    let id = b();
    let attrs = temporal_speed_and_brand();
    let retrieve = RetrieveTemporalEvolution::builder()
        .id(&id)
        .selection(TemporalSelection::Unbounded {
            time_property: None,
            last_n: NonZeroU64::new(3),
            aggregation_methods: None,
            aggregation_period: None,
        })
        .attrs(&attrs)
        .format(Lossless::Normalized)
        .build();
    assert_eq!(query_string(&retrieve).as_deref(), Some("attrs=speed,brandName&lastN=3"));
}

#[test]
fn a_temporal_query_with_attrs_and_no_type_is_bounded() {
    let attrs = temporal_speed_and_brand();
    let window = window();
    let request = request(&temporal_query(&attrs, &window, QueryMethod::Get));
    assert_eq!(
        request.uri().query(),
        Some("attrs=speed,brandName&timerel=between&timeAt=2018-08-01T12%3A03%3A00Z&endTimeAt=2018-08-01T12%3A07%3A00Z")
    );
}

#[test]
fn a_temporal_query_asks_for_the_evolution_of_the_scope_in_attrs_as_annex_c_5_16_2_does() {
    let attrs = [
        TemporalAttributeName::from("speed".parse::<AttributeName>().unwrap()),
        TemporalAttributeName::Scope,
    ];
    let window = window();
    let get = request(&temporal_query(&attrs, &window, QueryMethod::Get));
    assert_eq!(
        get.uri().query(),
        Some("attrs=speed,scope&timerel=between&timeAt=2018-08-01T12%3A03%3A00Z&endTimeAt=2018-08-01T12%3A07%3A00Z")
    );
    let post = request(&temporal_query(&attrs, &window, QueryMethod::Post));
    assert!(body_text(&post).contains(r#""attrs":["speed","scope"]"#), "{}", body_text(&post));
}

#[test]
fn retrieve_temporal_evolution_sends_scope_in_attrs() {
    let id = b();
    let attrs = [TemporalAttributeName::Scope];
    let retrieve = RetrieveTemporalEvolution::builder()
        .id(&id)
        .selection(TemporalSelection::Unbounded {
            time_property: None,
            last_n: None,
            aggregation_methods: None,
            aggregation_period: None,
        })
        .attrs(&attrs)
        .format(Lossless::Normalized)
        .build();
    assert_eq!(query_string(&retrieve).as_deref(), Some("attrs=scope"));
}

#[test]
fn scope_is_no_attribute_name_outside_the_temporal_api() {
    assert_eq!("scope".parse::<AttributeName>(), Err(NameError::Reserved { member: ReservedMember::Scope }));
}
