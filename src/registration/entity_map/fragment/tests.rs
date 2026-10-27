//! The `EntityMap` fragment (clauses 5.2.39 and 5.14.2): reading, writing and each refusal.

use crate::{
    model::{DocumentError, DocumentErrorKind, MemberError},
    registration::EntityMapFragment,
};
use pretty_assertions::assert_eq;

/// A fragment that moves the expiry.
const EXPIRY: &str = r#"{"type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z"}"#;

/// The error reading `text`.
fn error(text: &str) -> DocumentError {
    EntityMapFragment::parse(text).unwrap_err()
}

#[test]
fn a_fragment_that_moves_the_expiry_reads_and_writes_back() {
    let fragment = EntityMapFragment::parse(EXPIRY).unwrap();
    assert_eq!(fragment.expires_at.as_ref().map(ToString::to_string).as_deref(), Some("2030-01-01T00:00:00Z"));
    assert_eq!(fragment.render().to_string(), EXPIRY);
    assert_eq!(fragment, EntityMapFragment::new("2030-01-01T00:00:00Z".parse().unwrap()));
}

#[test]
fn a_fragment_may_leave_out_the_type_and_the_expiry() {
    let untyped = EntityMapFragment::parse(r#"{"expiresAt":"2030-01-01T00:00:00Z"}"#).unwrap();
    assert_eq!(untyped.render().to_string(), EXPIRY);
    let empty = EntityMapFragment::parse("{}").unwrap();
    assert_eq!(empty, EntityMapFragment::default());
    assert_eq!(empty.render().to_string(), r#"{"type":"EntityMap"}"#);
}

#[test]
fn the_id_the_output_members_and_the_context_are_checked_and_not_kept() {
    let whole = concat!(
        r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z","#,
        r#""entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none"]},"linkedMaps":{},"#,
        r#""@context":"https://example.org/context.jsonld"}"#
    );
    let fragment = EntityMapFragment::parse(whole).unwrap();
    assert_eq!(fragment.render().to_string(), EXPIRY);
}

#[test]
fn a_member_that_is_malformed_is_refused_even_when_it_would_be_ignored() {
    let id = error(r#"{"id":"not a uri"}"#);
    assert!(matches!(id.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
    let entities = error(r#"{"entityMap":{"not a uri":[]}}"#);
    assert_eq!(entities.path.to_string(), "entityMap");
    assert!(matches!(entities.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
    let linked = error(r#"{"linkedMaps":[]}"#);
    assert!(matches!(linked.kind, DocumentErrorKind::Member(MemberError::NotAnObject)));
}

#[test]
fn the_type_the_expiry_and_unknown_members_are_checked() {
    let typed = error(r#"{"type":"Entity"}"#);
    assert_eq!(typed.path.to_string(), "type");
    assert!(matches!(typed.kind, DocumentErrorKind::Member(MemberError::InvalidToken { member: "type", ref found }) if found == "Entity"));
    let expired_by = error(r#"{"expiredBy":"2030-01-01T00:00:00Z"}"#);
    assert!(matches!(expired_by.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "expiredBy"));
    let not_a_date = error(r#"{"expiresAt":"tomorrow"}"#);
    assert_eq!(not_a_date.path.to_string(), "expiresAt");
    let deleted = error(r#"{"expiresAt":"urn:ngsi-ld:null"}"#);
    assert_eq!(deleted.path.to_string(), "expiresAt");
}

#[test]
fn a_member_written_twice_is_refused() {
    let expiry = error(r#"{"expiresAt":"2030-01-01T00:00:00Z","expiresAt":"2031-01-01T00:00:00Z"}"#);
    assert!(matches!(expiry.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "expiresAt"));
}
