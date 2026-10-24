//! The geoquery and scope query parameters: the `georel`, `geometry`, `coordinates`,
//! `geoproperty` and `scopeQ` parameters of Query Entities and Purge Entities, and the `geoQ` and `scopeQ` members of
//! a `Query` body.

use crate::{fixtures::endpoint, query_vehicles};
use http::Method;
use ngsi_ld::{
    http::{
        RequestError,
        operation::{ArgumentError, Operation, PurgeEntities, QueryEntities, QueryMethod},
    },
    model::Lossless,
    query::{SelectorType, geoquery::GeoQuery, q, scope_query::ScopeQuery},
};
use pretty_assertions::assert_eq;

/// The geoquery of clause 4.10 example 1.
fn near_observation_space() -> GeoQuery {
    GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40],"geoproperty":"observationSpace"}"#).unwrap()
}

/// The geoquery of clause 4.10 example 2.
fn within_polygon() -> GeoQuery {
    GeoQuery::parse(
        r#"{"georel":"within","geometry":"Polygon","coordinates":[[[100.0,0.0],[101.0,0.0],[101.0,1.0],[100.0,1.0],[100.0,0.0]]],"geoproperty":"location"}"#,
    )
    .unwrap()
}

/// The geoquery of clause 4.10 example 3.
fn near_location() -> GeoQuery {
    GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}"#).unwrap()
}

/// The query-string parameters of clause 4.10 example 2, encoded.
const WITHIN_POLYGON: &str = "georel=within&geometry=Polygon&coordinates=%5B%5B%5B100.0%2C0.0%5D%2C%5B101.0%2C0.0%5D%2C%5B101.0%2C1.0%5D%2C%5B100.0%2C1.0%5D%2C%5B100.0%2C0.0%5D%5D%5D&geoproperty=location";

/// The query string of the request `operation` builds.
fn query_string(operation: &impl Operation) -> Option<String> {
    operation.to_request(&endpoint()).unwrap().uri().query().map(str::to_owned)
}

/// A purge with no parameter.
fn purge() -> PurgeEntities<'static> {
    PurgeEntities::builder().build()
}

#[test]
fn a_get_query_writes_the_geoquery_of_example_1() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let geo_q = near_observation_space();
    let query = QueryEntities {
        geo_q: Some(&geo_q),
        ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
    };
    assert_eq!(
        query_string(&query).as_deref(),
        Some("type=Vehicle&georel=near;maxDistance==2000&geometry=Point&coordinates=%5B8%2C40%5D&geoproperty=observationSpace")
    );
}

#[test]
fn a_geoquery_alone_bounds_a_query() {
    let geo_q = within_polygon();
    let query = QueryEntities {
        geo_q: Some(&geo_q),
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert_eq!(query_string(&query).as_deref(), Some(WITHIN_POLYGON));
}

#[test]
fn a_get_query_writes_scope_queries_with_their_syntax() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let cases = [
        (
            "(/Madrid/Districts;/CompanyA)|/CompanyB",
            "type=Vehicle&scopeQ=(/Madrid/Districts;/CompanyA)%7C/CompanyB",
        ),
        ("/Madrid/+/ParqueNorte", "type=Vehicle&scopeQ=/Madrid/%2B/ParqueNorte"),
        ("/Madrid/Gardens/#", "type=Vehicle&scopeQ=/Madrid/Gardens/%23"),
        ("/#", "type=Vehicle&scopeQ=/%23"),
    ];
    for (text, expected) in cases {
        let scope_q: ScopeQuery = text.parse().unwrap();
        let query = QueryEntities {
            scope_q: Some(&scope_q),
            ..query_vehicles(Some(&entity_type), None, Lossless::Normalized)
        };
        assert_eq!(query_string(&query).as_deref(), Some(expected), "{text}");
    }
}

#[test]
fn a_scope_query_alone_does_not_bound_a_query() {
    let scope_q: ScopeQuery = "/Madrid".parse().unwrap();
    let query = QueryEntities {
        scope_q: Some(&scope_q),
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    assert!(matches!(
        query.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::UnboundedQuery))
    ));
}

#[test]
fn a_post_query_carries_the_geoquery_and_the_scope_query_in_its_body() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let speed = q::attr("speed").gt(50).build().unwrap();
    let geo_q = near_observation_space();
    let scope_q: ScopeQuery = "/Madrid/#".parse().unwrap();
    let query = QueryEntities {
        geo_q: Some(&geo_q),
        scope_q: Some(&scope_q),
        method: QueryMethod::Post,
        ..query_vehicles(Some(&entity_type), Some(&speed), Lossless::Normalized)
    };
    let sent = query.to_request(&endpoint()).unwrap();
    assert_eq!(sent.method(), Method::POST);
    assert_eq!(sent.uri().query(), None);
    assert_eq!(
        str::from_utf8(sent.body()).unwrap(),
        r#"{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50","geoQ":{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40],"geoproperty":"observationSpace"},"scopeQ":"/Madrid/#"}"#
    );
}

#[test]
fn a_post_query_with_a_geoquery_alone_is_sent() {
    let geo_q = near_location();
    let query = QueryEntities {
        geo_q: Some(&geo_q),
        method: QueryMethod::Post,
        ..query_vehicles(None, None, Lossless::Normalized)
    };
    let sent = query.to_request(&endpoint()).unwrap();
    assert_eq!(
        str::from_utf8(sent.body()).unwrap(),
        r#"{"type":"Query","geoQ":{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}}"#
    );
}

#[test]
fn purge_takes_a_geoquery_and_a_scope_query_but_a_scope_query_alone_does_not_bound_it() {
    let geo_q = within_polygon();
    let scope_q: ScopeQuery = "/Madrid".parse().unwrap();
    let bounded = PurgeEntities {
        geo_q: Some(&geo_q),
        scope_q: Some(&scope_q),
        ..purge()
    };
    let sent = bounded.to_request(&endpoint()).unwrap();
    assert_eq!(sent.method(), Method::DELETE);
    assert_eq!(sent.uri().path(), "/ngsi-ld/v1/entities");
    assert_eq!(sent.uri().query().map(str::to_owned), Some(format!("{WITHIN_POLYGON}&scopeQ=/Madrid")));
    let scoped_only = PurgeEntities {
        scope_q: Some(&scope_q),
        ..purge()
    };
    assert!(matches!(
        scoped_only.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::UnboundedQuery))
    ));
}
