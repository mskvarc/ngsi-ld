use crate::model::{
    Attribute,
    AttributeKind,
    EntityId,
    LanguageMap,
    Lossless,
    NullMember,
    OneOrMany,
    Output,
    Relationship,
    SimplifiedValue,
    Value,
    VocabTerm,
    null::{NGSI_LD_NULL, check_language_map, check_object, check_object_list, check_value, check_value_list, check_vocab},
};
use jstrict::Value as Json;
use pretty_assertions::assert_eq;
use strum::IntoEnumIterator;

fn value(json: &str) -> Value {
    Value::from_json(json.parse::<Json>().unwrap()).unwrap()
}

fn id(text: &str) -> EntityId {
    text.parse().unwrap()
}

#[test]
fn a_value_that_is_the_null_or_holds_it_as_a_member_at_any_depth_is_refused() {
    assert_eq!(check_value(&Value::from(NGSI_LD_NULL.as_str())), Err(NullMember::Value));
    assert_eq!(check_value(&value(r#"{"country":"urn:ngsi-ld:null"}"#)), Err(NullMember::ValueMember));
    assert_eq!(check_value(&value(r#"{"a":{"b":[1,{"c":"urn:ngsi-ld:null"}]}}"#)), Err(NullMember::ValueMember));
}

#[test]
fn a_value_holding_the_null_outside_an_object_member_is_kept() {
    assert_eq!(check_value(&value(r#"["urn:ngsi-ld:null"]"#)), Ok(()));
    assert_eq!(check_value(&Value::from("urn:ngsi-ld:other")), Ok(()));
}

#[test]
fn a_language_map_whose_default_text_is_the_null_is_refused() {
    let null = LanguageMap::try_from_pairs([("@none", NGSI_LD_NULL.as_str())]).unwrap();
    assert_eq!(check_language_map(&null), Err(NullMember::LanguageMap));
    let english = LanguageMap::try_from_pairs([("en", NGSI_LD_NULL.as_str())]).unwrap();
    assert_eq!(check_language_map(&english), Ok(()));
}

#[test]
fn the_list_shaped_payloads_are_refused_when_they_are_the_null_alone() {
    let null_term: VocabTerm = NGSI_LD_NULL.as_str().parse().unwrap();
    assert_eq!(check_vocab(&OneOrMany::one(null_term.clone())), Err(NullMember::Vocab));
    assert_eq!(check_vocab(&OneOrMany::try_from(vec![null_term, "x".parse().unwrap()]).unwrap()), Ok(()));
    assert_eq!(check_value_list(&[Value::from(NGSI_LD_NULL.as_str())]), Err(NullMember::ValueList));
    assert_eq!(check_value_list(&[Value::from(NGSI_LD_NULL.as_str()), Value::from(1)]), Ok(()));
    assert_eq!(check_object(&OneOrMany::one(id(NGSI_LD_NULL.as_str()))), Err(NullMember::Object));
    assert_eq!(
        check_object(&OneOrMany::try_from(vec![id(NGSI_LD_NULL.as_str())]).unwrap()),
        Err(NullMember::Object)
    );
    assert_eq!(check_object_list(&[id(NGSI_LD_NULL.as_str())]), Err(NullMember::ObjectList));
    assert_eq!(check_object_list(&[]), Ok(()));
}

fn attribute(json: &str) -> Attribute<Output> {
    Attribute::<Output>::parse(json, Lossless::Normalized).unwrap()
}

#[test]
fn the_deletion_record_of_every_kind_is_deleted() {
    for kind in AttributeKind::iter() {
        assert!(Attribute::deleted(kind).build().is_deleted(), "{kind}");
        let dated = Attribute::deleted(kind).deleted_at("2024-01-02T00:00:00Z".parse().unwrap()).build();
        assert!(dated.is_deleted(), "{kind} with deletedAt");
    }
    let previous: Attribute<Output> = Relationship::deleted().previous_object(id("urn:ngsi-ld:Person:Bob")).build().into();
    assert!(previous.is_deleted());
}

#[test]
fn a_deletion_read_from_a_notification_is_deleted_in_each_shape_clause_4_5_0_gives() {
    for json in [
        r#""urn:ngsi-ld:null""#,
        r#"{"type": "Property", "value": "urn:ngsi-ld:null", "previousValue": 80}"#,
        r#"{"type": "LanguageProperty", "languageMap": {"@none": "urn:ngsi-ld:null"}}"#,
        r#"{"type": "VocabProperty", "vocab": "urn:ngsi-ld:null"}"#,
        r#"{"type": "ListProperty", "valueList": ["urn:ngsi-ld:null"]}"#,
        r#"{"type": "JsonProperty", "json": "urn:ngsi-ld:null"}"#,
        r#"{"type": "Relationship", "object": "urn:ngsi-ld:null", "datasetId": "urn:ngsi-ld:dataset:gps"}"#,
        r#"{"type": "ListRelationship", "objectList": ["urn:ngsi-ld:null"]}"#,
    ] {
        assert!(attribute(json).is_deleted(), "{json}");
    }
}

#[test]
fn an_attribute_that_only_holds_the_null_somewhere_inside_its_payload_is_not_deleted() {
    for json in [
        r#"{"type": "Property", "value": 80}"#,
        r#"{"type": "Property", "value": {"country": "urn:ngsi-ld:null"}}"#,
        r#"{"type": "Property", "value": ["urn:ngsi-ld:null"]}"#,
        r#"{"type": "LanguageProperty", "languageMap": {"en": "urn:ngsi-ld:null"}}"#,
        r#"{"type": "ListProperty", "valueList": ["urn:ngsi-ld:null", 1]}"#,
        r#"{"type": "Relationship", "object": "urn:ngsi-ld:Person:Bob"}"#,
    ] {
        assert!(!attribute(json).is_deleted(), "{json}");
    }
}

#[test]
fn a_simplified_value_is_deleted_in_each_shape_the_null_takes_and_only_then() {
    let deleted = [
        SimplifiedValue::Value(Value::from(NGSI_LD_NULL.as_str())),
        SimplifiedValue::Value(value(r#"["urn:ngsi-ld:null"]"#)),
        SimplifiedValue::LanguageMap(LanguageMap::try_from_pairs([("@none", NGSI_LD_NULL.as_str())]).unwrap()),
        SimplifiedValue::Vocab(OneOrMany::one(NGSI_LD_NULL.as_str().parse().unwrap())),
    ];
    for value in &deleted {
        assert!(value.is_deleted(), "{value:?}");
    }
    let kept = [
        SimplifiedValue::Value(Value::from(55)),
        SimplifiedValue::Value(value(r#"{"country":"urn:ngsi-ld:null"}"#)),
        SimplifiedValue::Value(value(r#"["urn:ngsi-ld:null", 1]"#)),
        SimplifiedValue::LanguageMap(LanguageMap::try_from_pairs([("en", NGSI_LD_NULL.as_str())]).unwrap()),
    ];
    for value in &kept {
        assert!(!value.is_deleted(), "{value:?}");
    }
}
