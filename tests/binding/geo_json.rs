//! The `GeoJSON` representation (clauses 4.5.16, 4.5.17 and 6.3.15):
//! the `Accept` header and `geometryProperty` each request sends, and the Features and FeatureCollections read back.

use crate::{
    fixtures::{context, endpoint, header_value, json, response, vehicle_id},
    links::{link_field, link_value},
    query_vehicles,
    retrieve_entity,
};
use http::header;
use jstrict::Unordered;
use ngsi_ld::{
    http::{
        BodyError,
        ResponseError,
        headers::RelationType,
        operation::{GeoJson, Operation, QueryEntities, QueryMethod, Simplified},
    },
    model::{AttributeName, Lossless},
    query::{SelectorType, projection::Projection},
};
use pretty_assertions::assert_eq;

/// A Feature of the vehicle, in normalized representation.
const VEHICLE_FEATURE: &str = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":{"type":"Point","coordinates":[13.3986,52.5547]},"properties":{"type":"Vehicle","speed":{"type":"Property","value":80},"site":{"type":"GeoProperty","value":{"type":"Point","coordinates":[13.3986,52.5547]}}}}"#;

/// The `GeoProperty` the tests select as geometry.
fn site() -> AttributeName {
    "site".parse().unwrap()
}

#[test]
fn a_geo_json_retrieve_asks_for_geo_json_and_sends_the_geometry_property() {
    let id = vehicle_id();
    let site = site();
    let retrieve = retrieve_entity(
        &id,
        GeoJson {
            form: Lossless::Concise,
            geometry_property: Some(&site),
        },
    );
    let request = retrieve.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), Some("format=concise&geometryProperty=site"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/geo+json"));
    let default = retrieve_entity(
        &id,
        GeoJson {
            form: Lossless::Normalized,
            geometry_property: None,
        },
    );
    let request = default.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/geo+json"));
}

#[test]
fn a_geo_json_query_via_get_sends_the_format_and_the_geometry_property() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let site = site();
    let query = query_vehicles(
        Some(&entity_type),
        None,
        GeoJson {
            form: Simplified,
            geometry_property: Some(&site),
        },
    );
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), Some("type=Vehicle&format=simplified&geometryProperty=site"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/geo+json"));
}

#[test]
fn a_geo_json_query_via_post_keeps_the_geometry_property_in_the_query_string() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let site = site();
    let query = QueryEntities {
        method: QueryMethod::Post,
        ..query_vehicles(
            Some(&entity_type),
            None,
            GeoJson {
                form: Simplified,
                geometry_property: Some(&site),
            },
        )
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), Some("format=simplified&geometryProperty=site"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/geo+json"));
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"type": "Query", "entities": [{"type": "Vehicle"}]}"#))
    );
}

#[test]
fn a_geo_json_query_may_pick_attributes_without_id_or_type() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let pick: Projection = "speed".parse().unwrap();
    let query = QueryEntities {
        pick: Some(&pick),
        ..query_vehicles(
            Some(&entity_type),
            None,
            GeoJson {
                form: Lossless::Normalized,
                geometry_property: None,
            },
        )
    };
    assert_eq!(query.to_request(&endpoint()).unwrap().uri().query(), Some("type=Vehicle&pick=speed"));
}

#[test]
fn a_geo_json_retrieve_reads_a_feature() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(
        &id,
        GeoJson {
            form: Lossless::Normalized,
            geometry_property: None,
        },
    );
    let retrieved = retrieve.decode(&context(), response(200, &[], VEHICLE_FEATURE)).unwrap();
    let feature = retrieved.into_body();
    assert_eq!(feature.entity.id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
    assert!(feature.geometry.is_some());
    assert_eq!(feature.render(Lossless::Normalized).to_string(), VEHICLE_FEATURE);
}

#[test]
fn a_geo_json_query_reads_a_feature_collection_into_a_page() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(
        Some(&entity_type),
        None,
        GeoJson {
            form: Lossless::Normalized,
            geometry_property: None,
        },
    );
    let body = format!(r#"{{"type":"FeatureCollection","features":[{VEHICLE_FEATURE},{VEHICLE_FEATURE}],"@context":"https://example.org/context.jsonld"}}"#);
    let page = query
        .decode(
            &context(),
            response(
                200,
                &[(
                    &header::LINK,
                    &link_field([link_value("/ngsi-ld/v1/entities?type=Vehicle&offset=2", RelationType::Next)]),
                )],
                &body,
            ),
        )
        .unwrap();
    assert_eq!(page.items().len(), 2);
    assert_eq!(
        page.context().map(|context| context.references().map(ToString::to_string).collect::<Vec<_>>()),
        Some(vec!["https://example.org/context.jsonld".to_owned()])
    );
    assert_eq!(
        page.next().map(ToString::to_string).as_deref(),
        Some("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=2")
    );
}

#[test]
fn a_json_array_is_not_the_answer_to_a_geo_json_query() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(
        Some(&entity_type),
        None,
        GeoJson {
            form: Simplified,
            geometry_property: None,
        },
    );
    let error = query.decode(&context(), response(200, &[], "[]")).unwrap_err();
    assert!(matches!(error, ResponseError::Body { source, .. } if matches!(*source, BodyError::Document(_))));
}

#[test]
fn the_next_page_of_a_geo_json_query_asks_for_geo_json_again() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(
        Some(&entity_type),
        None,
        GeoJson {
            form: Simplified,
            geometry_property: None,
        },
    );
    let page = query
        .decode(
            &context(),
            response(
                200,
                &[(
                    &header::LINK,
                    &link_field([link_value("?type=Vehicle&format=simplified&offset=2", RelationType::Next)]),
                )],
                r#"{"type":"FeatureCollection","features":[]}"#,
            ),
        )
        .unwrap();
    let request = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(
        request.uri().to_string(),
        "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&format=simplified&offset=2"
    );
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/geo+json"));
}
