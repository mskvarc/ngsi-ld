//! Each attribute kind written in both lossless forms: normalized, and concise, which drops what the kind implies
//! (clauses 4.5.2 and 4.5.3).

use crate::model::{
    Attribute,
    EntityId,
    EntityTypeName,
    GeoProperty,
    Input,
    JsonProperty,
    LanguageMap,
    LanguageProperty,
    LanguageText,
    ListProperty,
    ListRelationship,
    Lossless,
    Output,
    Property,
    RawJson,
    Relationship,
    Value,
    VocabProperty,
    VocabTerm,
    geometry::{Geometry, Position, Shape},
    representation::tests::observed,
};
use cefact_units::UnitCode;
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// Writes an attribute in both lossless forms.
fn both(attribute: impl Into<Attribute<Input>>) -> (String, String) {
    let attribute = attribute.into();
    (
        attribute.render(Lossless::Normalized).to_string(),
        attribute.render(Lossless::Concise).to_string(),
    )
}

#[test]
fn a_plain_property_is_its_bare_value_in_concise_form() {
    assert_eq!(
        both(Property::builder("Mercedes").build().unwrap()),
        (r#"{"type":"Property","value":"Mercedes"}"#.to_owned(), r#""Mercedes""#.to_owned())
    );
}

#[test]
fn a_property_with_observed_at_keeps_value_in_concise_form() {
    assert_eq!(
        both(Property::builder(7).observed_at(observed()).build().unwrap()),
        (
            r#"{"type":"Property","value":7,"observedAt":"2022-03-14T12:51:02.000Z"}"#.to_owned(),
            r#"{"value":7,"observedAt":"2022-03-14T12:51:02.000Z"}"#.to_owned()
        )
    );
}

#[test]
fn an_object_value_with_a_type_member_is_wrapped_in_concise_form() {
    let value = Value::from_json(r#"{"type":"Box","w":2}"#.parse().unwrap()).unwrap();
    assert_eq!(
        both(Property::builder(value).build().unwrap()),
        (
            r#"{"type":"Property","value":{"type":"Box","w":2}}"#.to_owned(),
            r#"{"value":{"type":"Box","w":2}}"#.to_owned()
        )
    );
}

#[test]
fn a_property_whose_value_looks_like_a_geometry_keeps_its_type_in_concise_form() {
    let value = Value::from_json(r#"{"type":"Point","coordinates":[8,40]}"#.parse().unwrap()).unwrap();
    let (normalized, concise) = both(Property::builder(value).build().unwrap());
    assert_eq!(normalized, r#"{"type":"Property","value":{"type":"Point","coordinates":[8,40]}}"#);
    assert_eq!(concise, normalized);
    assert!(Attribute::<Output>::parse(&concise, Lossless::Concise).unwrap().is_property());
}

#[test]
fn a_plain_geo_property_is_its_bare_geometry_in_concise_form() {
    let point = Geometry::from(Shape::Point(Position::try_new(8.0, 40.0).unwrap()));
    assert_eq!(
        both(GeoProperty::builder(point).build().unwrap()),
        (
            r#"{"type":"GeoProperty","value":{"type":"Point","coordinates":[8,40]}}"#.to_owned(),
            r#"{"type":"Point","coordinates":[8,40]}"#.to_owned()
        )
    );
}

#[test]
fn a_relationship_drops_its_type_in_concise_form() {
    assert_eq!(
        both(Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Person:Bob"))).build().unwrap()),
        (
            r#"{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob"}"#.to_owned(),
            r#"{"object":"urn:ngsi-ld:Person:Bob"}"#.to_owned()
        )
    );
}

#[test]
fn a_relationship_with_two_objects_and_an_object_type() {
    let objects = vec![EntityId::from(iri!("urn:ngsi-ld:Person:Alice")), EntityId::from(iri!("urn:ngsi-ld:Person:Bob"))];
    let person: EntityTypeName = "Person".parse().unwrap();
    let (normalized, concise) = both(Relationship::builder(objects).object_type(person).build().unwrap());
    assert_eq!(
        normalized,
        r#"{"type":"Relationship","objectType":"Person","object":["urn:ngsi-ld:Person:Alice","urn:ngsi-ld:Person:Bob"]}"#
    );
    assert_eq!(
        concise,
        r#"{"objectType":"Person","object":["urn:ngsi-ld:Person:Alice","urn:ngsi-ld:Person:Bob"]}"#
    );
}

#[test]
fn a_language_property() {
    let map = LanguageMap::try_from_pairs([("fr", "Grand Place"), ("nl", "Grote Markt")]).unwrap();
    assert_eq!(
        both(LanguageProperty::builder(map).build().unwrap()),
        (
            r#"{"type":"LanguageProperty","languageMap":{"fr":"Grand Place","nl":"Grote Markt"}}"#.to_owned(),
            r#"{"languageMap":{"fr":"Grand Place","nl":"Grote Markt"}}"#.to_owned()
        )
    );
}

#[test]
fn a_language_property_with_several_strings_for_one_language_round_trips() {
    let map = LanguageMap::try_from_pairs([
        ("fr", LanguageText::Multiple(vec!["Grand Place".to_owned(), "Grote Markt".to_owned()])),
        ("@none", LanguageText::Single("Market square".to_owned())),
    ])
    .unwrap();
    let (normalized, concise) = both(LanguageProperty::builder(map).build().unwrap());
    assert_eq!(
        normalized,
        r#"{"type":"LanguageProperty","languageMap":{"fr":["Grand Place","Grote Markt"],"@none":"Market square"}}"#
    );
    assert_eq!(concise, r#"{"languageMap":{"fr":["Grand Place","Grote Markt"],"@none":"Market square"}}"#);
    for (text, form) in [(&normalized, Lossless::Normalized), (&concise, Lossless::Concise)] {
        let Attribute::LanguageProperty(property) = Attribute::<Output>::parse(text, form).unwrap() else {
            panic!("not a LanguageProperty");
        };
        assert_eq!(
            property.language_map().get(&"fr".parse().unwrap()),
            Some(&LanguageText::Multiple(vec!["Grand Place".to_owned(), "Grote Markt".to_owned()]))
        );
    }
}

#[test]
fn a_vocab_property() {
    assert_eq!(
        both(VocabProperty::builder("non-commercial".parse::<VocabTerm>().unwrap()).build().unwrap()),
        (
            r#"{"type":"VocabProperty","vocab":"non-commercial"}"#.to_owned(),
            r#"{"vocab":"non-commercial"}"#.to_owned()
        )
    );
}

#[test]
fn a_list_property_with_value_type_and_unit_code() {
    let property = ListProperty::builder([300, 300, 120, 6].into_iter().map(Value::from).collect())
        .value_type("Integer".parse().unwrap())
        .unit_code(UnitCode::Mmt)
        .build()
        .unwrap();
    assert_eq!(
        both(property),
        (
            r#"{"type":"ListProperty","valueList":[300,300,120,6],"valueType":"Integer","unitCode":"MMT"}"#.to_owned(),
            r#"{"valueList":[300,300,120,6],"valueType":"Integer","unitCode":"MMT"}"#.to_owned()
        )
    );
}

#[test]
fn a_list_relationship_wraps_objects_in_normalized_form_and_lists_uris_in_concise_form() {
    let cities = ["urn:ngsi-ld:City:Antwerp", "urn:ngsi-ld:City:Rotterdam", "urn:ngsi-ld:City:Amsterdam"]
        .map(|id| id.parse::<EntityId>().unwrap())
        .to_vec();
    let (normalized, concise) = both(
        ListRelationship::builder(cities)
            .object_type("City".parse::<EntityTypeName>().unwrap())
            .build()
            .unwrap(),
    );
    assert_eq!(
        normalized,
        r#"{"type":"ListRelationship","objectType":"City","objectList":[{"object":"urn:ngsi-ld:City:Antwerp"},{"object":"urn:ngsi-ld:City:Rotterdam"},{"object":"urn:ngsi-ld:City:Amsterdam"}]}"#
    );
    assert_eq!(
        concise,
        r#"{"objectType":"City","objectList":["urn:ngsi-ld:City:Antwerp","urn:ngsi-ld:City:Rotterdam","urn:ngsi-ld:City:Amsterdam"]}"#
    );
}

#[test]
fn a_json_property_keeps_its_members_untouched() {
    let json: RawJson = r#"{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"}"#.parse().unwrap();
    assert_eq!(
        both(JsonProperty::builder(json).build().unwrap()),
        (
            r#"{"type":"JsonProperty","json":{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"}}"#.to_owned(),
            r#"{"json":{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"}}"#.to_owned()
        )
    );
}

#[test]
fn a_relationship_with_a_sub_relationship_and_observed_at() {
    let parked = Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")))
        .observed_at("2017-07-29T12:00:04Z".parse().unwrap())
        .relationship("providedBy", iri!("urn:ngsi-ld:Person:Bob"))
        .build()
        .unwrap();
    assert_eq!(
        both(parked),
        (
            r#"{"type":"Relationship","object":"urn:ngsi-ld:OffStreetParking:Downtown1","observedAt":"2017-07-29T12:00:04Z","providedBy":{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob"}}"#.to_owned(),
            r#"{"object":"urn:ngsi-ld:OffStreetParking:Downtown1","observedAt":"2017-07-29T12:00:04Z","providedBy":{"object":"urn:ngsi-ld:Person:Bob"}}"#.to_owned()
        )
    );
}
