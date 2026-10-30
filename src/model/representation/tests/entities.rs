//! Entities read and written whole: their types, `location`, attribute names and order, `scope`, and the simplified
//! representation.

use crate::model::{
    AttributeName,
    AttributesError,
    Entity,
    Lossless,
    NameError,
    Number,
    Output,
    ProblemKind,
    Representation,
    SimplifiedEntity,
    SimplifiedValue,
    geometry::{Geometry, Position, Shape},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

#[test]
fn several_entity_types_round_trip_as_an_array() {
    let text = r#"{"id":"urn:ngsi-ld:B:1","type":["Building","House"]}"#;
    let entity = Entity::<Output>::parse(text, Lossless::Normalized).unwrap();
    assert!(entity.entity_type.is_many());
    assert_eq!(entity.render(Representation::Normalized).to_string(), text);
}

#[test]
fn location_must_be_a_geo_property() {
    let error = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .property("location", "somewhere")
        .build()
        .unwrap_err();
    assert!(matches!(
        &error.problems()[0].kind,
        ProblemKind::Attributes { name, source: AttributesError::GeoPropertyRequired } if name.to_string() == "location"
    ));
}

#[test]
fn a_location_whose_ordinates_hold_more_digits_than_a_double_keeps_them_written_and_read() {
    let longitude: Number = "13.39860000000000000001".parse().unwrap();
    let latitude: Number = "52.520".parse().unwrap();
    let point = Geometry::from(Shape::Point(Position::try_from_numbers(longitude, latitude).unwrap()));
    let entity = Entity::builder()
        .id(iri!("urn:ngsi-ld:Building:B1"))
        .entity_type("Building")
        .geo_property("location", point)
        .build()
        .unwrap();
    let normalized = concat!(
        r#"{"id":"urn:ngsi-ld:Building:B1","type":"Building","#,
        r#""location":{"type":"GeoProperty","value":{"type":"Point","coordinates":[13.39860000000000000001,52.520]}}}"#
    );
    assert_eq!(entity.render(Representation::Normalized).to_string(), normalized);
    let concise = r#"{"id":"urn:ngsi-ld:Building:B1","type":"Building","location":{"type":"Point","coordinates":[13.39860000000000000001,52.520]}}"#;
    assert_eq!(entity.render(Representation::Concise).to_string(), concise);
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let read = Entity::<Output>::parse(text, form).unwrap();
        assert_eq!(read.render(Representation::Normalized).to_string(), normalized);
    }
}

#[test]
fn every_invalid_attribute_name_is_reported_together() {
    let error = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .property("1speed", 1)
        .property("brand-name", "x")
        .build()
        .unwrap_err();
    let indexes: Vec<usize> = error
        .problems()
        .iter()
        .map(|problem| match &problem.kind {
            ProblemKind::AttributeName { index, .. } => *index,
            other @ (ProblemKind::Term { .. }
            | ProblemKind::Scope { .. }
            | ProblemKind::Identifier { .. }
            | ProblemKind::LanguageMap { .. }
            | ProblemKind::Geometry { .. }
            | ProblemKind::Json { .. }
            | ProblemKind::JsonShape
            | ProblemKind::Attributes { .. }
            | ProblemKind::EmptyList { .. }
            | ProblemKind::NgsiLdNull { .. }
            | ProblemKind::SubAttributeOfUnsetAttribute { .. }
            | ProblemKind::ProofNotAnObject
            | ProblemKind::MemberOfOtherKind { .. }
            | ProblemKind::DatasetValue) => panic!("unexpected problem {other}"),
        })
        .collect();
    assert_eq!(indexes, [0, 1]);
}

#[test]
fn an_invalid_attribute_name_is_reported_with_its_position_and_its_cause() {
    let error = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .property(AttributeName::try_from("speed").unwrap(), 1)
        .property("1speed", 2)
        .build()
        .unwrap_err();
    assert!(
        matches!(
            &error.problems()[0].kind,
            ProblemKind::AttributeName {
                index: 1,
                source: NameError::InvalidStart { found: '1', position: 0 }
            }
        ),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        "1 problem(s): attribute 1 is not a valid attribute name: a name must start with a letter, found `1` at byte 0"
    );
}

#[test]
fn attribute_order_is_kept_in_every_representation() {
    let entity = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .language_property("street", [("fr", "Grand Place")])
        .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
        .build()
        .unwrap();
    for representation in [Representation::Normalized, Representation::Concise, Representation::Simplified] {
        let text = entity.render(representation).to_string();
        let positions: Vec<usize> = ["brandName", "street", "isParked"].iter().map(|name| text.find(name).unwrap()).collect();
        assert!(positions.is_sorted(), "{representation}: {text}");
    }
    let parsed = Entity::<Output>::parse(&entity.render(Representation::Concise).to_string(), Lossless::Concise).unwrap();
    assert_eq!(
        parsed.attributes.names().map(ToString::to_string).collect::<Vec<_>>(),
        ["brandName", "street", "isParked"]
    );
}

#[test]
fn one_scope_is_a_string_and_several_an_array() {
    for text in [
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","scope":"/Madrid"}"#,
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","scope":["/Madrid","/CompanyA"]}"#,
    ] {
        assert_eq!(
            Entity::<Output>::parse(text, Lossless::Normalized)
                .unwrap()
                .render(Representation::Normalized)
                .to_string(),
            text
        );
    }
    let built = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .scope(["/Madrid", "/CompanyA"])
        .build()
        .unwrap();
    assert_eq!(built.scope.map(|scope| scope.iter().count()), Some(2));
}

#[test]
fn a_simplified_entity_reads_every_value_form() {
    let entity = SimplifiedEntity::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","a":1,"b":{"languageMap":{"en":"x"}},"c":{"vocab":"v"},"d":{"json":{"k":1}},"e":{"type":"Point","coordinates":[1,2]}}"#,
    )
    .unwrap();
    let kinds: Vec<bool> = entity.attributes.values().map(SimplifiedValue::is_value).collect();
    assert_eq!(kinds, [true, false, false, false, false]);
    assert!(entity.entity_type.as_one().is_some());
}

#[test]
fn an_owned_entity_simplifies_like_a_borrowed_one() {
    let entity = Entity::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":[{"type":"Property","value":55},{"type":"Property","value":60,"datasetId":"urn:ngsi-ld:Dataset:Radar"}],"isParked":{"type":"Relationship","object":"urn:ngsi-ld:OffStreetParking:Downtown1"},"location":{"type":"GeoProperty","value":{"type":"Point","coordinates":[-8.5,41.2]}}}"#,
        Lossless::Normalized,
    )
    .unwrap();
    let borrowed = SimplifiedEntity::from(&entity);
    let owned = SimplifiedEntity::from(entity);
    assert_eq!(owned, borrowed);
    assert_eq!(
        owned.render().to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"dataset":{"@none":55,"urn:ngsi-ld:Dataset:Radar":60}},"isParked":"urn:ngsi-ld:OffStreetParking:Downtown1","location":{"type":"Point","coordinates":[-8.5,41.2]}}"#
    );
}
