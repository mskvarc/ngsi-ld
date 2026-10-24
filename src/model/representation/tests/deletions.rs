//! Fragments that delete: the NGSI-LD Null each attribute kind is deleted with (clauses 4.5.0 and 5.4).

use crate::model::{AttributeKind, EntityFragment, EntityId, Lossless, ProblemKind, Property, representation::tests::observed};
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// The normalized deletion of a Property is `{"type": "Property", "value": "urn:ngsi-ld:null"}`. A normalized
/// attribute has a mandatory `type` (clause 4.5.2.2), which only a deletion of a known kind can write: `delete_kind`.
/// `delete` knows no kind, so it writes the bare Null of clauses 4.5.3.3 and 5.4 instead.
#[test]
fn deleting_a_property_writes_the_null_value() {
    let fragment = EntityFragment::builder().delete_kind("uncharged", AttributeKind::Property).build().unwrap();
    assert_eq!(
        fragment.render(Lossless::Normalized).to_string(),
        r#"{"uncharged":{"type":"Property","value":"urn:ngsi-ld:null"}}"#
    );
}

#[test]
fn deleting_a_member_in_concise_form_writes_the_bare_null() {
    let fragment = EntityFragment::builder().delete("isParked").build().unwrap();
    assert_eq!(fragment.render(Lossless::Concise).to_string(), r#"{"isParked":"urn:ngsi-ld:null"}"#);
}

#[test]
fn deleting_a_language_property_writes_the_null_language_map() {
    let fragment = EntityFragment::builder()
        .delete_kind("street", AttributeKind::LanguageProperty)
        .build()
        .unwrap();
    assert_eq!(
        fragment.render(Lossless::Normalized).to_string(),
        r#"{"street":{"type":"LanguageProperty","languageMap":{"@none":"urn:ngsi-ld:null"}}}"#
    );
}

#[test]
fn deleting_a_list_property_writes_a_one_null_list() {
    let fragment = EntityFragment::builder()
        .delete_kind("tyreTreadDepths", AttributeKind::ListProperty)
        .build()
        .unwrap();
    assert_eq!(
        fragment.render(Lossless::Normalized).to_string(),
        r#"{"tyreTreadDepths":{"type":"ListProperty","valueList":["urn:ngsi-ld:null"]}}"#
    );
}

#[test]
fn the_fragment_of_clause_5_4_example_2() {
    let fragment = EntityFragment::builder()
        .id(EntityId::from(iri!("urn:ngsi-ld:TemperatureSensor:001")))
        .entity_type("TemperatureSensor")
        .property("batteryLevel", Property::builder(7).observed_at(observed()))
        .delete_sub_attribute("batteryLevel", "providedBy")
        .delete_kind("uncharged", AttributeKind::Property)
        .build()
        .unwrap();
    assert_eq!(
        fragment.render(Lossless::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:TemperatureSensor:001","type":"TemperatureSensor","batteryLevel":{"type":"Property","value":7,"observedAt":"2022-03-14T12:51:02.000Z","providedBy":"urn:ngsi-ld:null"},"uncharged":{"type":"Property","value":"urn:ngsi-ld:null"}}"#
    );
}

#[test]
fn deleting_one_instance_writes_its_dataset_id() {
    let fragment = EntityFragment::builder()
        .delete_instance("speed", AttributeKind::Property, iri!("urn:ngsi-ld:dataset:gps"))
        .build()
        .unwrap();
    assert_eq!(
        fragment.render(Lossless::Normalized).to_string(),
        r#"{"speed":{"type":"Property","value":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:dataset:gps"}}"#
    );
}

#[test]
fn deleting_a_sub_attribute_of_an_attribute_the_fragment_does_not_set_is_reported() {
    let error = EntityFragment::builder()
        .delete_sub_attribute("batteryLevel", "providedBy")
        .build()
        .unwrap_err();
    assert!(matches!(&error.problems()[0].kind, ProblemKind::SubAttributeOfUnsetAttribute { .. }));
}

#[test]
fn a_kindless_deletion_is_the_bare_null_in_normalized_form_too() {
    let fragment = EntityFragment::builder().delete("uncharged").build().unwrap();
    assert_eq!(fragment.render(Lossless::Normalized).to_string(), r#"{"uncharged":"urn:ngsi-ld:null"}"#);
}
