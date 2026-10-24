use crate::model::{DatasetId, EntityId, InstanceId};
use iri_rs::iri;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;

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
