//! The output-only members of one attribute kind: the `previous*`
//! members of notifications, the linked entities of inline retrieval and the `lang` of the language filter.

use crate::model::{
    Attribute,
    AttributeKind,
    DocumentErrorKind,
    Entity,
    EntityId,
    GeoValue,
    Input,
    Instances,
    LanguageKey,
    LanguageMap,
    Lossless,
    MemberError,
    OneOrMany,
    Output,
    Representation,
    Value,
    VocabTerm,
    geometry::{Geometry, Position, Shape},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// Reads one response attribute in `form` and checks that it is written back as the same text.
fn round_trip(text: &str, form: Lossless) -> Attribute<Output> {
    let attribute = Attribute::<Output>::parse(text, form).unwrap();
    assert_eq!(attribute.render(form).to_string(), text);
    attribute
}

/// The member an `UnexpectedMember` error names.
fn unexpected_member(text: &str, form: Lossless) -> String {
    let error = Attribute::<Output>::parse(text, form).unwrap_err();
    let DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) = error.kind else {
        panic!("unexpected error {error}");
    };
    member
}

fn point(longitude: f64, latitude: f64) -> Geometry {
    Geometry::from(Shape::Point(Position::new(longitude, latitude).unwrap()))
}

#[test]
fn a_property_with_previous_value_round_trips_in_normalized_form() {
    let Attribute::Property(property) = round_trip(r#"{"type":"Property","value":7,"previousValue":5}"#, Lossless::Normalized) else {
        panic!("not a Property");
    };
    assert_eq!(property.value(), &Value::from(7));
    assert_eq!(property.output().previous_value, Some(Value::from(5)));
}

#[test]
fn a_property_with_previous_value_is_never_bare_in_concise_form() {
    let Attribute::Property(property) = round_trip(r#"{"value":7,"previousValue":5}"#, Lossless::Concise) else {
        panic!("not a Property");
    };
    assert_eq!(property.output().previous_value, Some(Value::from(5)));
}

#[test]
fn a_deleted_property_keeps_its_previous_value_beside_the_ngsi_ld_null() {
    let Attribute::Property(property) = round_trip(
        r#"{"type":"Property","value":"urn:ngsi-ld:null","previousValue":{"a":1}}"#,
        Lossless::Normalized,
    ) else {
        panic!("not a Property");
    };
    assert_eq!(property.value(), &Value::from("urn:ngsi-ld:null"));
    assert!(property.output().previous_value.as_ref().is_some_and(Value::is_object));
}

#[test]
fn a_geo_property_with_previous_value_round_trips_in_both_forms() {
    let normalized = r#"{"type":"GeoProperty","value":{"type":"Point","coordinates":[8,40]},"previousValue":{"type":"Point","coordinates":[8.5,40.5]}}"#;
    let concise = r#"{"value":{"type":"Point","coordinates":[8,40]},"previousValue":{"type":"Point","coordinates":[8.5,40.5]}}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::GeoProperty(property) = round_trip(text, form) else {
            panic!("not a GeoProperty");
        };
        assert_eq!(property.output().previous_value, Some(GeoValue::from(point(8.5, 40.5))));
    }
}

#[test]
fn a_deleted_language_property_keeps_its_previous_language_map() {
    let normalized = r#"{"type":"LanguageProperty","languageMap":{"@none":"urn:ngsi-ld:null"},"previousLanguageMap":{"fr":"Grand Place"}}"#;
    let concise = r#"{"languageMap":{"@none":"urn:ngsi-ld:null"},"previousLanguageMap":{"fr":"Grand Place"}}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::LanguageProperty(property) = round_trip(text, form) else {
            panic!("not a LanguageProperty");
        };
        let previous = property
            .output()
            .previous_language_map
            .as_ref()
            .map(LanguageMap::to_json)
            .map(|json| json.to_string());
        assert_eq!(previous.as_deref(), Some(r#"{"fr":"Grand Place"}"#));
    }
}

#[test]
fn a_vocab_property_with_previous_vocab_round_trips_in_both_forms() {
    let normalized = r#"{"type":"VocabProperty","vocab":"Female","previousVocab":["Male","Other"]}"#;
    let concise = r#"{"vocab":"Female","previousVocab":["Male","Other"]}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::VocabProperty(property) = round_trip(text, form) else {
            panic!("not a VocabProperty");
        };
        let previous: Vec<VocabTerm> = vec!["Male".parse().unwrap(), "Other".parse().unwrap()];
        assert_eq!(property.output().previous_vocab.as_ref().map(OneOrMany::as_slice), Some(previous.as_slice()));
    }
}

#[test]
fn a_list_property_with_previous_value_list_round_trips_in_both_forms() {
    let normalized = r#"{"type":"ListProperty","valueList":[1,2],"previousValueList":[3,4,5]}"#;
    let concise = r#"{"valueList":[1,2],"previousValueList":[3,4,5]}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::ListProperty(property) = round_trip(text, form) else {
            panic!("not a ListProperty");
        };
        assert_eq!(
            property.output().previous_value_list,
            Some(vec![Value::from(3), Value::from(4), Value::from(5)])
        );
    }
}

#[test]
fn a_json_property_with_previous_json_round_trips_and_keeps_the_json_shape_rule() {
    let normalized = r#"{"type":"JsonProperty","json":{"id":"a"},"previousJson":[{"id":"b","value":1}]}"#;
    let concise = r#"{"json":{"id":"a"},"previousJson":[{"id":"b","value":1}]}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::JsonProperty(property) = round_trip(text, form) else {
            panic!("not a JsonProperty");
        };
        assert!(property.output().previous_json.is_some());
    }
    let error = Attribute::<Output>::parse(r#"{"json":{"id":"a"},"previousJson":"text"}"#, Lossless::Concise).unwrap_err();
    assert_eq!(error.path.to_string(), "previousJson");
    assert!(matches!(error.kind, DocumentErrorKind::JsonShape));
}

#[test]
fn a_relationship_with_previous_object_round_trips_in_both_forms() {
    let normalized = r#"{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob","previousObject":"urn:ngsi-ld:Person:Alice"}"#;
    let concise = r#"{"object":"urn:ngsi-ld:Person:Bob","previousObject":"urn:ngsi-ld:Person:Alice"}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::Relationship(relationship) = round_trip(text, form) else {
            panic!("not a Relationship");
        };
        let alice = EntityId::from(iri!("urn:ngsi-ld:Person:Alice"));
        assert_eq!(relationship.output().previous_object, Some(OneOrMany::one(alice)));
    }
}

#[test]
fn previous_object_list_is_written_like_object_list_in_each_form() {
    let normalized = r#"{"type":"ListRelationship","objectList":[{"object":"urn:ngsi-ld:City:Antwerp"}],"previousObjectList":[{"object":"urn:ngsi-ld:City:Rotterdam"},{"object":"urn:ngsi-ld:City:Amsterdam"}]}"#;
    let concise = r#"{"objectList":["urn:ngsi-ld:City:Antwerp"],"previousObjectList":["urn:ngsi-ld:City:Rotterdam","urn:ngsi-ld:City:Amsterdam"]}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::ListRelationship(relationship) = round_trip(text, form) else {
            panic!("not a ListRelationship");
        };
        assert_eq!(relationship.output().previous_object_list.as_ref().map(Vec::len), Some(2));
    }
}

#[test]
fn an_inline_linked_entity_round_trips_in_normalized_form() {
    let text = r#"{"type":"Relationship","objectType":"OffStreetParking","object":"urn:ngsi-ld:OffStreetParking:Downtown1","observedAt":"2017-07-29T12:00:04Z","entity":{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"OffStreetParking","name":{"type":"Property","value":"Top Parking"},"operatedBy":{"type":"Relationship","object":"urn:ngsi-ld:Company:BigParkingCorp"}},"providedBy":{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob"}}"#;
    let Attribute::Relationship(relationship) = round_trip(text, Lossless::Normalized) else {
        panic!("not a Relationship");
    };
    let linked = relationship.output().entity.as_ref().and_then(OneOrMany::as_one).unwrap();
    assert_eq!(linked.id.as_ref().map(EntityId::as_str), Some("urn:ngsi-ld:OffStreetParking:Downtown1"));
    assert_eq!(linked.attributes.len(), 2);
    assert_eq!(relationship.meta().attributes.len(), 1);
}

#[test]
fn a_relationship_with_two_objects_carries_an_array_of_linked_entities() {
    let text = r#"{"type":"Relationship","objectType":"Person","object":["urn:ngsi-ld:Person:Alice","urn:ngsi-ld:Person:Bob"],"entity":[{"id":"urn:ngsi-ld:Person:Alice","type":"Person","name":{"type":"Property","value":"Alice"}},{"id":"urn:ngsi-ld:Person:Bob","type":"Person","name":{"type":"Property","value":"Bob"}}]}"#;
    let Attribute::Relationship(relationship) = round_trip(text, Lossless::Normalized) else {
        panic!("not a Relationship");
    };
    let linked = relationship.output().entity.as_ref().unwrap();
    assert!(linked.is_many());
    assert_eq!(linked.as_slice().len(), 2);
}

#[test]
fn linked_entities_are_read_and_written_in_the_representation_of_the_response() {
    let text = r#"{"objectType":"City","objectList":["urn:ngsi-ld:City:Antwerp","urn:ngsi-ld:City:Rotterdam"],"entityList":[{"id":"urn:ngsi-ld:City:Antwerp","type":"City","name":"Antwerp"},{"id":"urn:ngsi-ld:City:Rotterdam","type":"City","name":"Rotterdam"}]}"#;
    let Attribute::ListRelationship(relationship) = round_trip(text, Lossless::Concise) else {
        panic!("not a ListRelationship");
    };
    let linked = relationship.output().entity_list.as_ref().unwrap();
    assert_eq!(linked.len(), 2);
    let name = "name".parse().unwrap();
    let Some(Attribute::Property(name)) = linked[1].attribute(&name).map(Instances::first) else {
        panic!("no name Property");
    };
    assert_eq!(name.value(), &Value::from("Rotterdam"));
}

#[test]
fn lang_marks_the_language_the_language_filter_picked() {
    let normalized = r#"{"type":"Property","value":"Grand Place","lang":"fr"}"#;
    let concise = r#"{"value":"Grand Place","lang":"fr"}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let Attribute::Property(property) = round_trip(text, form) else {
            panic!("not a Property");
        };
        assert_eq!(property.output().lang, Some("fr".parse::<LanguageKey>().unwrap()));
    }
}

#[test]
fn lang_must_be_a_language_tag() {
    let error = Attribute::<Output>::parse(r#"{"value":"Grand Place","lang":"not a tag"}"#, Lossless::Concise).unwrap_err();
    assert_eq!(error.path.to_string(), "lang");
    assert!(matches!(error.kind, DocumentErrorKind::LanguageTag { .. }));
}

#[test]
fn a_member_another_kind_owns_is_refused_in_a_response() {
    let cases = [
        (r#"{"type":"Property","value":1,"previousObject":"urn:a:b"}"#, "previousObject"),
        (r#"{"type":"Relationship","object":"urn:a:b","previousValue":1}"#, "previousValue"),
        (r#"{"type":"GeoProperty","value":{"type":"Point","coordinates":[8,40]},"lang":"fr"}"#, "lang"),
        (r#"{"type":"Relationship","object":"urn:a:b","entityList":[]}"#, "entityList"),
        (r#"{"type":"ListRelationship","objectList":[{"object":"urn:a:b"}],"entity":{}}"#, "entity"),
        (r#"{"type":"LanguageProperty","languageMap":{"fr":"x"},"previousValue":"y"}"#, "previousValue"),
    ];
    for (text, member) in cases {
        assert_eq!(unexpected_member(text, Lossless::Normalized), member, "{text}");
    }
}

#[test]
fn a_request_refuses_every_output_only_member_naming_it() {
    let cases = [
        (r#"{"value":7,"previousValue":5}"#, "previousValue"),
        (r#"{"value":"Grand Place","lang":"fr"}"#, "lang"),
        (r#"{"languageMap":{"fr":"x"},"previousLanguageMap":{"fr":"y"}}"#, "previousLanguageMap"),
        (r#"{"vocab":"a","previousVocab":"b"}"#, "previousVocab"),
        (r#"{"valueList":[1],"previousValueList":[2]}"#, "previousValueList"),
        (r#"{"json":{"a":1},"previousJson":{"a":2}}"#, "previousJson"),
        (r#"{"object":"urn:a:b","previousObject":"urn:a:c"}"#, "previousObject"),
        (r#"{"object":"urn:a:b","entity":{"id":"urn:a:b","type":"T"}}"#, "entity"),
        (r#"{"objectList":["urn:a:b"],"previousObjectList":["urn:a:c"]}"#, "previousObjectList"),
        (r#"{"objectList":["urn:a:b"],"entityList":[{"id":"urn:a:b","type":"T"}]}"#, "entityList"),
    ];
    for (text, member) in cases {
        let error = Attribute::<Input>::parse_input(text, Lossless::Concise).unwrap_err();
        let DocumentErrorKind::Member(MemberError::UnexpectedMember { member: found }) = error.kind else {
            panic!("unexpected error {error} for {text}");
        };
        assert_eq!(found, member, "{text}");
    }
}

#[test]
fn into_input_drops_the_output_only_members_of_every_kind() {
    let output = Entity::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","street":{"type":"Property","value":"Grand Place","lang":"fr","previousValue":"Grote Markt"},"owner":{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob","entity":{"id":"urn:ngsi-ld:Person:Bob","type":"Person"}}}"#,
        Lossless::Normalized,
    )
    .unwrap();
    assert_eq!(
        output.into_input().unwrap().render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","street":{"type":"Property","value":"Grand Place"},"owner":{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob"}}"#
    );
}

#[test]
fn the_simplified_form_drops_the_output_only_members() {
    let output = Entity::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","speed":{"value":7,"previousValue":5},"owner":{"object":"urn:ngsi-ld:Person:Bob","entity":{"id":"urn:ngsi-ld:Person:Bob","type":"Person"}}}"#,
        Lossless::Concise,
    )
    .unwrap();
    assert_eq!(
        output.render(Representation::Simplified).to_string(),
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","speed":7,"owner":"urn:ngsi-ld:Person:Bob"}"#
    );
}

#[test]
fn an_output_only_member_given_twice_is_refused() {
    let error = Attribute::<Output>::parse(r#"{"type":"Property","value":1,"previousValue":1,"previousValue":2}"#, Lossless::Normalized).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "previousValue"));
}

#[test]
fn the_previous_member_does_not_change_the_inferred_kind() {
    let property = Attribute::<Output>::parse(r#"{"value":1,"previousValue":{"type":"Point","coordinates":[8,40]}}"#, Lossless::Concise).unwrap();
    assert_eq!(property.kind(), AttributeKind::Property);
    // The kind is a GeoProperty from its `value`, so its `previousValue` must be a geometry too.
    let error = Attribute::<Output>::parse(r#"{"value":{"type":"Point","coordinates":[8,40]},"previousValue":1}"#, Lossless::Concise).unwrap_err();
    assert_eq!(error.path.to_string(), "previousValue");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::Geometry { .. })));
}
