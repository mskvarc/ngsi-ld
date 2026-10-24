//! Fragments that name their entity, the elements of Batch Entity Update and Merge (clauses 5.6.9.3 and
//! 5.6.20.3): built with `id` and `type`, written with them first.

use crate::model::{Complete, Entity, EntityFragment, Lossless, ProblemKind, Representation};
use iri_rs::iri;
use pretty_assertions::assert_eq;

#[test]
fn a_fragment_for_an_entity_holds_its_id_and_type_and_writes_them_first() {
    let fragment: EntityFragment<Complete> = EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
        .property("speed", 0)
        .delete("isParked")
        .build()
        .unwrap();
    assert_eq!(fragment.id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(fragment.entity_type.as_one().map(ToString::to_string).as_deref(), Some("Vehicle"));
    assert_eq!(
        fragment.render(Lossless::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":0},"isParked":"urn:ngsi-ld:null"}"#
    );
}

#[test]
fn a_fragment_for_an_entity_reports_an_invalid_type_and_attribute_name_together() {
    let error = EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "1Vehicle")
        .property("1speed", 0)
        .build()
        .unwrap_err();
    let kinds: Vec<&ProblemKind> = error.problems().iter().map(|problem| &problem.kind).collect();
    assert_eq!(kinds.len(), 2, "{error}");
    assert!(matches!(kinds[0], ProblemKind::Term { .. }), "{error}");
    assert!(matches!(kinds[1], ProblemKind::AttributeName { index: 0, .. }), "{error}");
}

#[test]
fn an_entity_turned_into_a_fragment_names_the_entity_and_writes_as_the_entity() {
    let entity = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", 80)
        .build()
        .unwrap();
    let text = entity.render(Representation::Normalized).to_string();
    let fragment: EntityFragment<Complete> = entity.into_fragment();
    assert_eq!(fragment.id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(fragment.render(Lossless::Normalized).to_string(), text);
}
