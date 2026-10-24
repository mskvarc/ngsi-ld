//! Tests of the `GeoJSON` representation (NGSI-LD clauses 4.5.16 and 4.5.17).

use crate::model::{DocumentError, DocumentErrorKind, Entity, Feature, FeatureCollection, GeoJsonType, Lossless, MemberError, Output, SimplifiedEntity};
use pretty_assertions::assert_eq;

/// The parking of Annex C.2.3 as a normalized Feature, in canonical member order, with the Feature's own
/// `@context` (Table 5.2.29-1).
const NORMALIZED: &str = r#"{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"Feature","geometry":{"type":"Point","coordinates":[-8.51,41.1]},"properties":{"type":"OffStreetParking","name":{"type":"Property","value":"Downtown One"},"availableSpotNumber":{"type":"Property","value":121,"observedAt":"2017-07-29T12:05:02Z","reliability":{"type":"Property","value":0.7},"providedBy":{"type":"Relationship","object":"urn:ngsi-ld:Camera:C1"}},"location":{"type":"GeoProperty","value":{"type":"Point","coordinates":[-8.51,41.1]}}},"@context":"https://example.org/parking.jsonld"}"#;

/// The same parking as a concise Feature.
const CONCISE: &str = r#"{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"Feature","geometry":{"type":"Point","coordinates":[-8.51,41.1]},"properties":{"type":"OffStreetParking","name":"Downtown One","availableSpotNumber":{"value":121,"observedAt":"2017-07-29T12:05:02Z","reliability":0.7,"providedBy":{"object":"urn:ngsi-ld:Camera:C1"}},"location":{"type":"Point","coordinates":[-8.51,41.1]}},"@context":"https://example.org/parking.jsonld"}"#;

/// The same parking as a simplified Feature (clause 4.5.17.1).
const SIMPLIFIED: &str = r#"{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"Feature","geometry":{"type":"Point","coordinates":[-8.51,41.1]},"properties":{"type":"OffStreetParking","name":"Downtown One","availableSpotNumber":121,"location":{"type":"Point","coordinates":[-8.51,41.1]}},"@context":"https://example.org/parking.jsonld"}"#;

/// The kind of a document error.
fn kind(error: DocumentError) -> DocumentErrorKind {
    error.kind
}

#[test]
fn a_normalized_feature_round_trips_with_its_entity_and_geometry() {
    let feature = Feature::<Entity<Output>>::parse(NORMALIZED, Lossless::Normalized).unwrap();
    assert_eq!(feature.entity.id.as_str(), "urn:ngsi-ld:OffStreetParking:Downtown1");
    assert_eq!(
        feature.entity.entity_type.as_one().map(ToString::to_string).as_deref(),
        Some("OffStreetParking")
    );
    assert_eq!(feature.entity.attributes.len(), 3);
    assert!(feature.geometry.is_some());
    assert!(feature.context.is_some());
    assert_eq!(feature.entity.context, None);
    assert_eq!(feature.render(Lossless::Normalized).to_string(), NORMALIZED);
}

#[test]
fn a_concise_feature_reads_the_same_entity_and_round_trips() {
    let concise = Feature::<Entity<Output>>::parse(CONCISE, Lossless::Concise).unwrap();
    let normalized = Feature::<Entity<Output>>::parse(NORMALIZED, Lossless::Normalized).unwrap();
    assert_eq!(concise, normalized);
    assert_eq!(concise.render(Lossless::Concise).to_string(), CONCISE);
}

#[test]
fn a_simplified_feature_round_trips() {
    let feature = Feature::<SimplifiedEntity>::parse(SIMPLIFIED).unwrap();
    assert_eq!(feature.entity.attributes.len(), 3);
    assert_eq!(feature.render().to_string(), SIMPLIFIED);
}

#[test]
fn a_null_geometry_reads_as_none_and_is_written_back_as_null() {
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":null,"properties":{"type":"Vehicle","speed":80}}"#;
    let feature = Feature::<SimplifiedEntity>::parse(text).unwrap();
    assert_eq!(feature.geometry, None);
    assert_eq!(feature.render().to_string(), text);
}

#[test]
fn a_context_inside_properties_stays_the_entity_context() {
    let text = r#"{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"Feature","geometry":null,"properties":{"type":"OffStreetParking","name":{"type":"Property","value":"Downtown One"},"@context":["https://example.org/parking.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"]}}"#;
    let feature = Feature::<Entity<Output>>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(feature.context, None);
    assert!(feature.entity.context.is_some());
    assert_eq!(feature.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn a_feature_collection_round_trips_and_may_be_empty() {
    let feature = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":{"type":"Point","coordinates":[13.3986,52.5547]},"properties":{"type":"Vehicle","speed":80}}"#;
    let text = format!(r#"{{"type":"FeatureCollection","features":[{feature},{feature}],"@context":"https://example.org/context.jsonld"}}"#);
    let collection = FeatureCollection::<SimplifiedEntity>::parse(&text).unwrap();
    assert_eq!(collection.features.len(), 2);
    assert!(collection.context.is_some());
    assert_eq!(collection.render().to_string(), text);
    let empty = FeatureCollection::<Entity<Output>>::parse(r#"{"type":"FeatureCollection","features":[]}"#, Lossless::Normalized).unwrap();
    assert_eq!(empty.features, Vec::new());
    assert_eq!(empty.render(Lossless::Normalized).to_string(), r#"{"type":"FeatureCollection","features":[]}"#);
}

#[test]
fn a_wrong_geojson_type_is_named() {
    let point = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Point","geometry":null,"properties":{"type":"Vehicle"}}"#;
    assert!(matches!(
        kind(Feature::<SimplifiedEntity>::parse(point).unwrap_err()),
        DocumentErrorKind::GeoJsonType { expected: GeoJsonType::Feature, found } if found == "Point"
    ));
    let feature = r#"{"type":"Feature","features":[]}"#;
    assert!(matches!(
        kind(FeatureCollection::<SimplifiedEntity>::parse(feature).unwrap_err()),
        DocumentErrorKind::GeoJsonType { expected: GeoJsonType::FeatureCollection, found } if found == "Feature"
    ));
}

#[test]
fn a_feature_without_geometry_or_properties_is_refused() {
    let no_geometry = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","properties":{"type":"Vehicle"}}"#;
    assert!(matches!(
        kind(Feature::<SimplifiedEntity>::parse(no_geometry).unwrap_err()),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "geometry" })
    ));
    let no_properties = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":null}"#;
    assert!(matches!(
        kind(Feature::<SimplifiedEntity>::parse(no_properties).unwrap_err()),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "properties" })
    ));
}

#[test]
fn an_id_inside_properties_or_an_unknown_member_is_refused() {
    let inner_id = r#"{"type":"Feature","geometry":null,"properties":{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle"}}"#;
    let error = Feature::<SimplifiedEntity>::parse(inner_id).unwrap_err();
    assert_eq!(error.path.to_string(), "properties");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "id"));
    let bbox = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","bbox":[0,0,1,1],"geometry":null,"properties":{"type":"Vehicle"}}"#;
    assert!(matches!(
        kind(Feature::<SimplifiedEntity>::parse(bbox).unwrap_err()),
        DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "bbox"
    ));
}

#[test]
fn the_geometry_of_a_feature_keeps_its_own_bounding_box() {
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":{"type":"Point","coordinates":[8,40],"bbox":[8,40,8,40]},"properties":{"type":"Vehicle","speed":80}}"#;
    let feature = Feature::<SimplifiedEntity>::parse(text).unwrap();
    assert!(feature.geometry.as_ref().is_some_and(|geometry| geometry.bbox().is_some()));
    assert_eq!(feature.render().to_string(), text);
}

#[test]
fn a_feature_without_id_or_entity_type_is_refused() {
    let no_id = r#"{"type":"Feature","geometry":null,"properties":{"type":"Vehicle"}}"#;
    assert!(matches!(
        kind(Feature::<Entity<Output>>::parse(no_id, Lossless::Normalized).unwrap_err()),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })
    ));
    let no_type = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":null,"properties":{"speed":80}}"#;
    assert!(matches!(
        kind(Feature::<SimplifiedEntity>::parse(no_type).unwrap_err()),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "type" })
    ));
}

#[test]
fn a_geometry_collection_is_not_a_feature_geometry() {
    let text =
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":{"type":"GeometryCollection","geometries":[]},"properties":{"type":"Vehicle"}}"#;
    let error = Feature::<SimplifiedEntity>::parse(text).unwrap_err();
    assert_eq!(error.path.to_string(), "geometry");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::Geometry { .. })));
}

#[test]
fn a_built_feature_writes_its_members_in_canonical_order() {
    let entity = SimplifiedEntity::parse(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":80}"#).unwrap();
    let feature = Feature::new(entity, None);
    assert_eq!(
        feature.render().to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":null,"properties":{"type":"Vehicle","speed":80}}"#
    );
    let collection = FeatureCollection::new(vec![feature]);
    assert_eq!(collection.features.len(), 1);
    assert_eq!(collection.context, None);
}
