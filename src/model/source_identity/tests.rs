//! The Context Source Identity document (clause 5.2.40): reading, writing and each refusal.

use crate::model::{ContextSourceAlias, ContextSourceAliasError, ContextSourceId, ContextSourceIdentity, DocumentError, DocumentErrorKind, MemberError};
use pretty_assertions::assert_eq;

/// A Context Source Identity with every member.
const ID: &str = concat!(
    r#"{"id":"urn:ngsi-ld:ContextSource:cs1","type":"ContextSourceIdentity","contextSourceAlias":"broker-1","#,
    r#""contextSourceUptime":"P1DT2H","contextSourceTimeAt":"2024-05-01T10:00:00Z","#,
    r#""contextSourceExtras":{"version":"1.2","tenants":["t1"]}}"#
);

/// The `ID` text without `contextSourceExtras`.
const WITHOUT_EXTRAS: &str = concat!(
    r#"{"id":"urn:ngsi-ld:ContextSource:cs1","type":"ContextSourceIdentity","contextSourceAlias":"broker-1","#,
    r#""contextSourceUptime":"P1DT2H","contextSourceTimeAt":"2024-05-01T10:00:00Z"}"#
);

/// `ID` with the text `old` replaced by `new`.
fn replaced(old: &str, new: &str) -> String {
    assert!(ID.contains(old), "{old}");
    ID.replace(old, new)
}

/// The error reading `text`.
fn error(text: &str) -> DocumentError {
    ContextSourceIdentity::parse(text).unwrap_err()
}

#[test]
fn an_identity_reads_every_member_and_round_trips() {
    let identity = ContextSourceIdentity::parse(ID).unwrap();
    assert_eq!(identity.id.as_str(), "urn:ngsi-ld:ContextSource:cs1");
    assert_eq!(identity.alias.as_ref(), "broker-1");
    assert_eq!(identity.uptime.to_string(), "P1DT2H");
    assert_eq!(identity.time_at.to_string(), "2024-05-01T10:00:00Z");
    assert_eq!(
        identity.extras.as_ref().map(ToString::to_string).as_deref(),
        Some(r#"{"version":"1.2","tenants":["t1"]}"#)
    );
    assert_eq!(identity.context, None);
    assert_eq!(identity.render().to_string(), ID);
}

#[test]
fn extras_are_optional_and_kept_as_written_and_a_context_is_written_last() {
    let with_context = format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, WITHOUT_EXTRAS.trim_end_matches('}'));
    let identity = ContextSourceIdentity::parse(&with_context).unwrap();
    assert_eq!(identity.extras, None);
    assert!(identity.context.is_some());
    assert_eq!(identity.render().to_string(), with_context);
    let text = replaced(
        r#""contextSourceExtras":{"version":"1.2","tenants":["t1"]}"#,
        r#""contextSourceExtras":"plain text""#,
    );
    let identity = ContextSourceIdentity::parse(&text).unwrap();
    assert_eq!(identity.extras.as_ref().map(ToString::to_string).as_deref(), Some(r#""plain text""#));
    assert_eq!(identity.render().to_string(), text);
}

#[test]
fn an_alias_that_is_not_an_http_token_is_refused() {
    for (alias, expected) in [
        ("", ContextSourceAliasError::Empty),
        ("broker 1", ContextSourceAliasError::InvalidCharacter),
        ("a/b", ContextSourceAliasError::InvalidCharacter),
        ("münchen", ContextSourceAliasError::InvalidCharacter),
    ] {
        let error = error(&replaced(r#""contextSourceAlias":"broker-1""#, &format!(r#""contextSourceAlias":"{alias}""#)));
        assert_eq!(error.path.to_string(), "contextSourceAlias");
        assert!(
            matches!(error.kind, DocumentErrorKind::Member(MemberError::ContextSourceAlias { source }) if source == expected),
            "{alias}"
        );
    }
}

#[test]
fn the_uptime_is_a_duration_of_zero_or_more_and_the_time_a_timestamp() {
    for uptime in ["-PT1H", "1 hour"] {
        let error = error(&replaced(r#""contextSourceUptime":"P1DT2H""#, &format!(r#""contextSourceUptime":"{uptime}""#)));
        assert_eq!(error.path.to_string(), "contextSourceUptime");
        assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::Duration { .. })), "{uptime}");
    }
    let zero = replaced(r#""contextSourceUptime":"P1DT2H""#, r#""contextSourceUptime":"PT0S""#);
    assert_eq!(ContextSourceIdentity::parse(&zero).unwrap().render().to_string(), zero);
    let error = error(&replaced(
        r#""contextSourceTimeAt":"2024-05-01T10:00:00Z""#,
        r#""contextSourceTimeAt":"yesterday""#,
    ));
    assert_eq!(error.path.to_string(), "contextSourceTimeAt");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::Temporal { .. })));
}

#[test]
fn the_type_and_every_mandatory_member_are_checked_and_unknown_members_refused() {
    let typed = error(&replaced(r#""type":"ContextSourceIdentity""#, r#""type":"ContextSource""#));
    assert_eq!(typed.path.to_string(), "type");
    assert!(matches!(typed.kind, DocumentErrorKind::Member(MemberError::InvalidToken { member: "type", ref found }) if found == "ContextSource"));
    for (member, text) in [
        ("id", r#""id":"urn:ngsi-ld:ContextSource:cs1","#),
        ("type", r#""type":"ContextSourceIdentity","#),
        ("contextSourceAlias", r#""contextSourceAlias":"broker-1","#),
        ("contextSourceUptime", r#""contextSourceUptime":"P1DT2H","#),
        ("contextSourceTimeAt", r#""contextSourceTimeAt":"2024-05-01T10:00:00Z","#),
    ] {
        let missing = error(&replaced(text, ""));
        assert!(
            matches!(missing.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: found }) if found == member),
            "{member}"
        );
    }
    let unknown = error(&replaced(r#""contextSourceExtras":{"version":"1.2","tenants":["t1"]}"#, r#""foo":1"#));
    assert!(matches!(unknown.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "foo"));
}

#[test]
fn every_http_token_is_an_alias() {
    for alias in ["broker-1", "cs.example_2~", "!#$%&'*+-.^_`|~09AZaz"] {
        assert_eq!(alias.parse::<ContextSourceAlias>().map(|alias| alias.to_string()), Ok(alias.to_owned()));
    }
}

#[test]
fn a_built_identity_renders_in_canonical_order() {
    let identity = ContextSourceIdentity::new(
        "urn:ngsi-ld:ContextSource:cs1".parse::<ContextSourceId>().unwrap(),
        "broker-1".parse().unwrap(),
        "P1DT2H".parse().unwrap(),
        "2024-05-01T10:00:00Z".parse().unwrap(),
    );
    assert_eq!(identity.render().to_string(), WITHOUT_EXTRAS);
}
