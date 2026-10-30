use crate::model::{DatasetId, EntityId, InstanceId};
use iri_rs::iri;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;
use std::collections::HashSet;

#[test]
fn entity_id_accepts_a_urn() {
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    assert_eq!(id.to_string(), "urn:ngsi-ld:Vehicle:A4567");
}

#[test]
fn entity_id_rejects_a_string_that_is_not_a_uri() {
    assert!("Vehicle A4567".parse::<EntityId>().is_err());
}

#[test]
fn entity_id_from_a_compile_time_literal_equals_the_parsed_one() {
    let parsed: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    assert_eq!(EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")), parsed);
}

#[test]
fn dataset_id_accepts_the_default_keyword_and_an_iri() {
    let default: DatasetId = "@none".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    assert_eq!(default, DatasetId::DefaultInstance);
    assert_eq!(default.to_string(), "@none");
    assert_eq!(gps.to_string(), "urn:ngsi-ld:dataset:gps");
}

#[test]
fn dataset_id_round_trips_through_serde_as_written() {
    for input in ["@none", "urn:ngsi-ld:dataset:gps"] {
        let id: DatasetId = jstrict::from_value(Json::from(input)).unwrap();
        assert_eq!(jstrict::to_value(&id).unwrap(), Json::from(input));
    }
}

#[test]
fn dataset_id_rejects_a_relative_reference() {
    assert!("gps".parse::<DatasetId>().is_err());
}

#[test]
fn instance_id_from_a_compile_time_literal_equals_the_parsed_one() {
    let parsed: InstanceId = "urn:ngsi-ld:attribute:instance:1".parse().unwrap();
    assert_eq!(InstanceId::from(iri!("urn:ngsi-ld:attribute:instance:1")), parsed);
}

#[test]
fn a_verbatim_identifier_writes_its_upper_case_scheme_back() {
    let verbatim = EntityId::verbatim("URN:NGSI-LD:Vehicle:B9211").unwrap();
    assert_eq!(verbatim.to_string(), "URN:NGSI-LD:Vehicle:B9211");
    assert_eq!(verbatim.as_str(), "URN:NGSI-LD:Vehicle:B9211");
    assert_eq!(String::from(verbatim.as_uri().clone()), "URN:NGSI-LD:Vehicle:B9211");
    let parsed: EntityId = "URN:NGSI-LD:Vehicle:B9211".parse().unwrap();
    assert_eq!(parsed.to_string(), "urn:ngsi-ld:Vehicle:B9211");
}

#[test]
fn a_verbatim_identifier_equals_its_canonical_form_and_hashes_with_it() {
    let verbatim = EntityId::verbatim("URN:NGSI-LD:Vehicle:B9211").unwrap();
    let parsed: EntityId = "urn:ngsi-ld:Vehicle:B9211".parse().unwrap();
    assert_eq!(verbatim, parsed);
    assert_eq!(HashSet::from([verbatim, parsed]).len(), 1);
}

#[test]
fn a_verbatim_identifier_that_is_already_canonical_keeps_nothing() {
    let verbatim = EntityId::verbatim("urn:ngsi-ld:Vehicle:B9211").unwrap();
    assert_eq!(verbatim.to_string(), "urn:ngsi-ld:Vehicle:B9211");
    assert_eq!(verbatim, "urn:ngsi-ld:Vehicle:B9211".parse().unwrap());
}

#[test]
fn a_verbatim_identifier_is_validated_like_a_parsed_one() {
    assert_eq!(
        EntityId::verbatim("Vehicle A4567").unwrap_err(),
        "Vehicle A4567".parse::<EntityId>().unwrap_err()
    );
}

#[test]
fn a_verbatim_dataset_and_instance_id_write_their_text_back() {
    assert_eq!(DatasetId::verbatim("URN:ngsi-ld:dataset:gps").unwrap().to_string(), "URN:ngsi-ld:dataset:gps");
    assert_eq!(DatasetId::verbatim("@none").unwrap(), DatasetId::DefaultInstance);
    assert_eq!(InstanceId::verbatim("URN:ngsi-ld:instance:1").unwrap().to_string(), "URN:ngsi-ld:instance:1");
}
