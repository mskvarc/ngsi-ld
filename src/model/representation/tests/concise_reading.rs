//! The rules a concise attribute is read by, and the attributes no rule can read (clauses 4.5.2.3 and 4.5.3.3).

use crate::model::{Attribute, AttributeKind, DocumentErrorKind, Entity, Lossless, MemberError, Output, Value};
use cefact_units::UnitCode;
use pretty_assertions::assert_eq;

/// Reads one concise attribute.
fn concise(text: &str) -> Attribute<Output> {
    Attribute::<Output>::parse(text, Lossless::Concise).unwrap()
}

#[test]
fn a_concise_geometry_with_a_bounding_box_is_a_geo_property_and_keeps_its_box() {
    let text = r#"{"type":"Point","coordinates":[8,40],"bbox":[8,40,8,40]}"#;
    let read = concise(text);
    assert!(read.is_geo_property());
    assert_eq!(
        read.render(Lossless::Normalized).to_string(),
        format!(r#"{{"type":"GeoProperty","value":{text}}}"#)
    );
    assert_eq!(read.render(Lossless::Concise).to_string(), text);
}

#[test]
fn a_bare_string_is_a_property() {
    assert_eq!(concise(r#""Mercedes""#).kind(), AttributeKind::Property);
}

#[test]
fn a_bare_geometry_is_a_geo_property() {
    assert_eq!(concise(r#"{"type": "Point", "coordinates": [8, 40]}"#).kind(), AttributeKind::GeoProperty);
}

#[test]
fn a_value_holding_a_geometry_is_a_geo_property() {
    assert_eq!(
        concise(r#"{"value": {"type": "Point", "coordinates": [8, 40]}}"#).kind(),
        AttributeKind::GeoProperty
    );
}

#[test]
fn a_value_with_observed_at_is_a_property_with_observed_at() {
    let attribute = concise(r#"{"value": 7, "observedAt": "2022-03-14T12:51:02Z"}"#);
    assert_eq!(attribute.kind(), AttributeKind::Property);
    assert_eq!(attribute.meta().observed_at, Some("2022-03-14T12:51:02Z".parse().unwrap()));
}

#[test]
fn to_p11_the_payload_member_decides_the_kind() {
    for (text, kind) in [
        (r#"{"object": "urn:ngsi-ld:Person:Bob"}"#, AttributeKind::Relationship),
        (
            r#"{"objectList": ["urn:ngsi-ld:City:Antwerp", "urn:ngsi-ld:City:Rotterdam"]}"#,
            AttributeKind::ListRelationship,
        ),
        (r#"{"objectList": [{"object": "urn:ngsi-ld:City:Antwerp"}]}"#, AttributeKind::ListRelationship),
        (r#"{"languageMap": {"en": "yes"}}"#, AttributeKind::LanguageProperty),
        (r#"{"vocab": "Male"}"#, AttributeKind::VocabProperty),
        (r#"{"valueList": [1, 2]}"#, AttributeKind::ListProperty),
        (r#"{"json": {"id": "a"}}"#, AttributeKind::JsonProperty),
    ] {
        assert_eq!(concise(text).kind(), kind, "{text}");
    }
}

#[test]
fn an_object_with_type_is_read_as_normalized() {
    let attribute = concise(r#"{"type": "Property", "value": 1}"#);
    assert_eq!(attribute.kind(), AttributeKind::Property);
    assert!(attribute.meta().attributes.is_empty());
}

#[test]
fn other_members_beside_value_are_sub_attributes() {
    let attribute = concise(r#"{"value": 1, "unitCode": "CEL", "accuracy": 0.5}"#);
    let Attribute::Property(property) = &attribute else { panic!("a Property") };
    assert_eq!(property.unit_code(), Some(&UnitCode::Cel));
    assert_eq!(attribute.meta().attributes.names().map(ToString::to_string).collect::<Vec<_>>(), ["accuracy"]);
}

#[test]
fn a_bare_array_is_the_value_of_a_property() {
    let Attribute::Property(property) = concise("[1, 2, 3]") else {
        panic!("a Property")
    };
    assert_eq!(property.value(), &Value::from(vec![1, 2, 3]));
}

#[test]
fn a_normalized_relationship_without_object_names_the_attribute() {
    let error = Entity::<Output>::parse(r#"{"id": "urn:ngsi-ld:T:1", "type": "T", "x": {"type": "Relationship"}}"#, Lossless::Normalized).unwrap_err();
    assert_eq!(error.path.to_string(), "x");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "object" })));
}

#[test]
fn an_unknown_attribute_type_is_an_error() {
    let error = Attribute::<Output>::parse(r#"{"type": "Unknown", "value": 1}"#, Lossless::Concise).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::UnknownAttributeType { ref found } if found == "\"Unknown\""));
}

#[test]
fn two_kind_defining_members_are_ambiguous() {
    let error = Attribute::<Output>::parse(r#"{"object": "urn:a:b", "vocab": "x"}"#, Lossless::Concise).unwrap_err();
    assert!(matches!(
        error.kind,
        DocumentErrorKind::AmbiguousKind {
            first: "object",
            second: "vocab"
        }
    ));
}

#[test]
fn a_member_another_kind_owns_is_refused() {
    let error = Attribute::<Output>::parse(r#"{"type": "Relationship", "object": "urn:a:b", "unitCode": "CEL"}"#, Lossless::Normalized).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "unitCode"));
}
