//! Entities a projection reduced (clauses 4.21 and 6.4.3.2).

use crate::model::{Complete, DocumentErrorKind, Entity, EntityId, Lossless, MemberError, Output, Projected, Representation, SimplifiedEntity};
use pretty_assertions::assert_eq;

#[test]
fn a_projected_entity_without_id_or_type_round_trips() {
    let text = r#"{"speed":{"type":"Property","value":80}}"#;
    let entity = Entity::<Output, Projected>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(entity.id, None);
    assert_eq!(entity.entity_type, None);
    assert_eq!(entity.attributes.len(), 1);
    assert_eq!(entity.render(Representation::Normalized).to_string(), text);
    assert_eq!(entity.render(Representation::Concise).to_string(), r#"{"speed":80}"#);
}

#[test]
fn a_projected_entity_keeps_the_identifying_members_it_has() {
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","speed":{"type":"Property","value":80}}"#;
    let entity = Entity::<Output, Projected>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(entity.id.as_ref().map(EntityId::as_str), Some("urn:ngsi-ld:Vehicle:A4567"));
    assert_eq!(entity.entity_type, None);
    assert_eq!(entity.render(Representation::Normalized).to_string(), text);
}

#[test]
fn a_projected_simplified_entity_round_trips() {
    let text = r#"{"type":"Vehicle","speed":80}"#;
    let entity = SimplifiedEntity::parse_projected(text).unwrap();
    assert_eq!(entity.id, None);
    assert!(entity.entity_type.is_some());
    assert_eq!(entity.render().to_string(), text);
}

#[test]
fn a_complete_entity_still_requires_id_and_type() {
    let error = Entity::<Output>::parse(r#"{"speed":{"type":"Property","value":80}}"#, Lossless::Normalized).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })));
    let error = SimplifiedEntity::parse(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","speed":80}"#).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "type" })));
}

#[test]
fn simplifying_a_projected_entity_keeps_its_completeness() {
    let entity = Entity::<Output, Projected>::parse(r#"{"speed":{"type":"Property","value":80}}"#, Lossless::Normalized).unwrap();
    let simplified = SimplifiedEntity::<Projected>::from(&entity);
    assert_eq!(simplified.id, None);
    assert_eq!(simplified.render().to_string(), r#"{"speed":80}"#);
    let complete = Entity::<Output, Complete>::parse(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle"}"#, Lossless::Concise).unwrap();
    assert_eq!(SimplifiedEntity::from(&complete).id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
}
