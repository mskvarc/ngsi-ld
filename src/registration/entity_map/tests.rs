//! The `EntityMap` document (clause 5.2.39): reading, writing and each refusal.

use crate::{
    model::{DocumentError, DocumentErrorKind, JsonType, MemberError},
    registration::{EntityMap, EntityMapId, EntitySource, RegistrationId},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// `A`.
const A: &str = "urn:ngsi-ld:Vehicle:A4567";

/// `R`.
const R: &str = "urn:ngsi-ld:ContextSourceRegistration:r1";

/// An entity map with an entity held locally and remotely, and a linked map.
const MAP: &str = concat!(
    r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","#,
    r#""entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none","urn:ngsi-ld:ContextSourceRegistration:r1"],"#,
    r#""urn:ngsi-ld:Vehicle:B9211":["urn:ngsi-ld:ContextSourceRegistration:r1"]},"#,
    r#""linkedMaps":{"urn:ngsi-ld:ContextSourceRegistration:r1":"urn:ngsi-ld:EntityMap:remote7"}}"#
);

/// `MAP` with the text `old` replaced by `new`.
fn replaced(old: &str, new: &str) -> String {
    assert!(MAP.contains(old), "{old}");
    MAP.replacen(old, new, 1)
}

/// The error reading `text`.
fn error(text: &str) -> DocumentError {
    EntityMap::parse(text).unwrap_err()
}

/// `R`.
fn r() -> RegistrationId {
    R.parse().unwrap()
}

#[test]
fn a_map_reads_its_entities_and_linked_maps_in_order_and_round_trips() {
    let map = EntityMap::parse(MAP).unwrap();
    assert_eq!(map.id, EntityMapId::from(iri!("urn:ngsi-ld:EntityMap:1")));
    assert_eq!(map.expires_at.to_string(), "2030-01-01T00:00:00Z");
    let entities: Vec<(&str, &[EntitySource])> = map.entity_map.iter().map(|(id, sources)| (id.as_str(), sources.as_slice())).collect();
    assert_eq!(
        entities,
        [
            (A, [EntitySource::Local, EntitySource::Registration(r())].as_slice()),
            ("urn:ngsi-ld:Vehicle:B9211", [EntitySource::Registration(r())].as_slice()),
        ]
    );
    assert_eq!(map.linked_maps.get(&r()).map(EntityMapId::as_str), Some("urn:ngsi-ld:EntityMap:remote7"));
    assert_eq!(map.context, None);
    assert_eq!(map.render().to_string(), MAP);
}

#[test]
fn empty_maps_are_read_and_a_context_is_written_last() {
    let empty = r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","entityMap":{},"linkedMaps":{}}"#;
    let map = EntityMap::parse(empty).unwrap();
    assert!(map.entity_map.is_empty() && map.linked_maps.is_empty());
    assert_eq!(map.render().to_string(), empty);
    let with_context = format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, MAP.strip_suffix('}').unwrap());
    let map = EntityMap::parse(&with_context).unwrap();
    assert!(map.context.is_some());
    assert_eq!(map.render().to_string(), with_context);
}

#[test]
fn the_expiry_is_expires_at_never_expired_by() {
    let expired_by = error(&replaced(r#""expiresAt""#, r#""expiredBy""#));
    assert!(matches!(expired_by.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "expiredBy"));
    let missing = error(&replaced(r#""expiresAt":"2030-01-01T00:00:00Z","#, ""));
    assert!(matches!(
        missing.kind,
        DocumentErrorKind::Member(MemberError::MissingMember { member: "expiresAt" })
    ));
}

#[test]
fn the_type_and_id_are_checked() {
    let typed = error(&replaced(r#""type":"EntityMap""#, r#""type":"Entity""#));
    assert_eq!(typed.path.to_string(), "type");
    assert!(matches!(typed.kind, DocumentErrorKind::Member(MemberError::InvalidToken { member: "type", ref found }) if found == "Entity"));
    let untyped = error(&replaced(r#""type":"EntityMap","#, ""));
    assert!(matches!(untyped.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "type" })));
    let anonymous = error(&replaced(r#""id":"urn:ngsi-ld:EntityMap:1","#, ""));
    assert!(matches!(anonymous.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })));
}

#[test]
fn both_output_members_are_required_and_unknown_members_refused() {
    let without_entities = error(&replaced(
        concat!(
            r#""entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none","urn:ngsi-ld:ContextSourceRegistration:r1"],"#,
            r#""urn:ngsi-ld:Vehicle:B9211":["urn:ngsi-ld:ContextSourceRegistration:r1"]},"#
        ),
        "",
    ));
    assert!(matches!(
        without_entities.kind,
        DocumentErrorKind::Member(MemberError::MissingMember { member: "entityMap" })
    ));
    let without_linked = error(&replaced(
        r#","linkedMaps":{"urn:ngsi-ld:ContextSourceRegistration:r1":"urn:ngsi-ld:EntityMap:remote7"}"#,
        "",
    ));
    assert!(matches!(
        without_linked.kind,
        DocumentErrorKind::Member(MemberError::MissingMember { member: "linkedMaps" })
    ));
    let unknown = error(&replaced(r#""type":"EntityMap","#, r#""type":"EntityMap","foo":1,"#));
    assert!(matches!(unknown.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "foo"));
}

#[test]
fn every_key_and_value_is_a_uri() {
    let bad_entity = error(&replaced(r#""urn:ngsi-ld:Vehicle:A4567":"#, r#""not a uri":"#));
    assert_eq!(bad_entity.path.to_string(), "entityMap");
    assert!(matches!(bad_entity.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
    let bad_source = error(&replaced(r#"["@none","urn:ngsi-ld:ContextSourceRegistration:r1"]"#, r#"["not a uri"]"#));
    assert_eq!(bad_source.path.to_string(), ["entityMap", A, "0"].join("."));
    assert!(matches!(bad_source.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
    let bad_registration = error(&replaced(r#"{"urn:ngsi-ld:ContextSourceRegistration:r1":"#, r#"{"not a uri":"#));
    assert_eq!(bad_registration.path.to_string(), "linkedMaps");
    assert!(matches!(bad_registration.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
    let bad_map = error(&replaced(r#""urn:ngsi-ld:EntityMap:remote7""#, r#""not a uri""#));
    assert_eq!(bad_map.path.to_string(), ["linkedMaps", R].join("."));
    assert!(matches!(bad_map.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
}

#[test]
fn sources_are_an_array_linked_maps_a_string_and_the_map_an_object() {
    let bare = error(&replaced(r#"["@none","urn:ngsi-ld:ContextSourceRegistration:r1"]"#, r#""@none""#));
    assert_eq!(bare.path.to_string(), ["entityMap", A].join("."));
    assert!(matches!(bare.kind, DocumentErrorKind::Member(MemberError::NotAnArray)));
    let listed = error(&replaced(r#""urn:ngsi-ld:EntityMap:remote7""#, r#"["urn:ngsi-ld:EntityMap:1"]"#));
    assert_eq!(listed.path.to_string(), ["linkedMaps", R].join("."));
    assert!(matches!(
        listed.kind,
        DocumentErrorKind::Member(MemberError::WrongShape {
            member: "linkedMaps",
            expected: JsonType::String
        })
    ));
    let array = error(&replaced(
        concat!(
            r#"{"urn:ngsi-ld:Vehicle:A4567":["@none","urn:ngsi-ld:ContextSourceRegistration:r1"],"#,
            r#""urn:ngsi-ld:Vehicle:B9211":["urn:ngsi-ld:ContextSourceRegistration:r1"]}"#
        ),
        "[]",
    ));
    assert_eq!(array.path.to_string(), "entityMap");
    assert!(matches!(array.kind, DocumentErrorKind::Member(MemberError::NotAnObject)));
}

#[test]
fn a_key_or_member_written_twice_is_refused() {
    let entity = error(&replaced(r#""urn:ngsi-ld:Vehicle:B9211":"#, r#""urn:ngsi-ld:Vehicle:A4567":"#));
    assert_eq!(entity.path.to_string(), "entityMap");
    assert!(matches!(entity.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == A));
    let registration = error(&replaced(
        r#"{"urn:ngsi-ld:ContextSourceRegistration:r1":"urn:ngsi-ld:EntityMap:remote7"}"#,
        r#"{"urn:ngsi-ld:ContextSourceRegistration:r1":"urn:ngsi-ld:EntityMap:remote7","urn:ngsi-ld:ContextSourceRegistration:r1":"urn:ngsi-ld:EntityMap:2"}"#,
    ));
    assert_eq!(registration.path.to_string(), "linkedMaps");
    assert!(matches!(registration.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == R));
    let expiry = error(&replaced(
        r#""expiresAt":"2030-01-01T00:00:00Z","#,
        r#""expiresAt":"2030-01-01T00:00:00Z","expiresAt":"2030-01-01T00:00:00Z","#,
    ));
    assert!(matches!(expiry.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "expiresAt"));
}

#[test]
fn a_built_map_renders_both_maps_even_when_empty() {
    let mut map = EntityMap::new(iri!("urn:ngsi-ld:EntityMap:1").into(), "2030-01-01T00:00:00Z".parse().unwrap());
    assert_eq!(
        map.render().to_string(),
        r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","entityMap":{},"linkedMaps":{}}"#
    );
    map.entity_map.insert(A.parse().unwrap(), vec![EntitySource::Local]);
    assert_eq!(
        map.render().to_string(),
        r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none"]},"linkedMaps":{}}"#
    );
}

#[test]
fn an_entity_map_id_is_a_uri() {
    assert_eq!(EntityMapId::from(iri!("urn:ngsi-ld:EntityMap:1")), "urn:ngsi-ld:EntityMap:1".parse().unwrap());
    assert!("not a uri".parse::<EntityMapId>().is_err());
}
