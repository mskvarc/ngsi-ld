//! A lone attribute read and written as a JSON-LD document, with its `@context` (clause 6.3.5): the body of Replace
//! Attribute and of Partial Attribute Update.

use crate::model::{
    Attribute,
    AttributeDocument,
    AttributeFragment,
    AttributeKind,
    Change,
    Context,
    DocumentErrorKind,
    Input,
    InstanceDeletion,
    Lossless,
    MemberError,
    NullMember,
};
use iri_rs::IriRefBuf;
use pretty_assertions::assert_eq;

/// The body of Replace Attribute.
type Replacement = AttributeDocument<Attribute<Input>>;

/// The body of Partial Attribute Update.
type Update = AttributeDocument<Change<AttributeFragment, InstanceDeletion>>;

/// The `@context` member every document below carries.
const CONTEXT_MEMBER: &str = r#""@context":"https://example.org/context.jsonld""#;

/// The context of [`CONTEXT_MEMBER`].
fn context() -> Context {
    Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap())
}

/// The object `body` with [`CONTEXT_MEMBER`] added last.
fn with_context(body: &str) -> String {
    match body.strip_suffix('}') {
        Some("{") => format!("{{{CONTEXT_MEMBER}}}"),
        Some(members) => format!("{members},{CONTEXT_MEMBER}}}"),
        None => panic!("{body} is not an object"),
    }
}

#[test]
fn a_replace_attribute_body_keeps_its_context_and_reads_the_attribute_as_an_attribute_alone_is_read() {
    for (form, attribute) in [
        (Lossless::Normalized, r#"{"type":"Property","value":30,"observedAt":"2018-08-01T12:03:00Z"}"#),
        (Lossless::Concise, r#"{"value":30,"observedAt":"2018-08-01T12:03:00Z"}"#),
        (Lossless::Normalized, r#"{"type":"Relationship","object":"urn:ngsi-ld:Person:Bob"}"#),
    ] {
        let body = with_context(attribute);
        let document = Replacement::parse(&body, form).unwrap();
        assert_eq!(document.attribute, Attribute::<Input>::parse_input(attribute, form).unwrap());
        assert_eq!(document.context, Some(context()));
        assert_eq!(document.render(form).to_string(), body);
    }
}

#[test]
fn a_context_written_before_the_attribute_is_read_and_written_last() {
    let document = Replacement::parse(
        r#"{"@context":["https://example.org/a.jsonld",{"speed":"https://example.org/speed"}],"type":"Property","value":1}"#,
        Lossless::Normalized,
    )
    .unwrap();
    assert_eq!(
        document.render(Lossless::Normalized).to_string(),
        r#"{"type":"Property","value":1,"@context":["https://example.org/a.jsonld",{"speed":"https://example.org/speed"}]}"#
    );
}

#[test]
fn a_body_without_a_context_holds_none_and_a_bare_concise_value_is_written_as_an_object() {
    let normalized = r#"{"type":"Property","value":30}"#;
    let document = Replacement::parse(normalized, Lossless::Normalized).unwrap();
    assert_eq!(document.context, None);
    assert_eq!(document.render(Lossless::Normalized).to_string(), normalized);
    let bare = Replacement::parse("30", Lossless::Concise).unwrap();
    assert_eq!(bare.attribute, Attribute::<Input>::parse_input("30", Lossless::Concise).unwrap());
    assert_eq!(bare.context, None);
    assert_eq!(bare.render(Lossless::Concise).to_string(), r#"{"value":30}"#);
}

#[test]
fn a_partial_attribute_update_body_keeps_its_context_whatever_it_changes() {
    for (form, fragment) in [
        (Lossless::Normalized, r#"{"type":"Property","value":55}"#),
        (Lossless::Concise, r#"{"observedAt":"2022-03-14T12:51:02Z"}"#),
        (Lossless::Normalized, r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#),
        (Lossless::Concise, "{}"),
    ] {
        let body = with_context(fragment);
        let document = Update::parse(&body, form).unwrap();
        assert_eq!(
            document.attribute,
            Change::<AttributeFragment, InstanceDeletion>::parse(fragment, form).unwrap(),
            "{body}"
        );
        assert_eq!(document.context, Some(context()), "{body}");
        assert_eq!(document.render(form).to_string(), body);
    }
}

#[test]
fn a_partial_attribute_update_body_with_a_context_alone_changes_nothing_and_one_with_the_null_deletes_the_instance() {
    let nothing = Update::parse(&with_context("{}"), Lossless::Concise).unwrap();
    assert!(matches!(nothing.attribute, Change::Set(AttributeFragment::WithoutPayload(_))));
    let deletion = Update::parse(&with_context(r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#), Lossless::Normalized).unwrap();
    assert_eq!(deletion.attribute, Change::Delete(InstanceDeletion::new(AttributeKind::Property)));
}

#[test]
fn a_context_that_is_no_context_is_refused_at_its_member_in_either_body() {
    let body = r#"{"type":"Property","value":1,"@context":5}"#;
    for error in [
        Replacement::parse(body, Lossless::Normalized).unwrap_err(),
        Update::parse(body, Lossless::Normalized).unwrap_err(),
    ] {
        assert_eq!(error.path.to_string(), "@context", "{error}");
        assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::Context { .. })), "{error}");
    }
}

#[test]
fn a_context_given_twice_is_refused() {
    let error = Replacement::parse(
        r#"{"@context":"https://example.org/a.jsonld","type":"Property","value":1,"@context":"https://example.org/b.jsonld"}"#,
        Lossless::Normalized,
    )
    .unwrap_err();
    assert!(
        matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "@context"),
        "{error}"
    );
}

#[test]
fn the_document_refuses_what_its_attribute_refuses() {
    let output_only = Replacement::parse(
        &with_context(r#"{"type":"Property","value":1,"createdAt":"2020-01-01T00:00:00Z"}"#),
        Lossless::Normalized,
    )
    .unwrap_err();
    assert!(
        matches!(output_only.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "createdAt"),
        "{output_only}"
    );
    let null = Replacement::parse(&with_context(r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#), Lossless::Normalized).unwrap_err();
    assert!(matches!(null.kind, DocumentErrorKind::NgsiLdNull { .. }), "{null}");
    let bare_null = Update::parse(r#""urn:ngsi-ld:null""#, Lossless::Concise).unwrap_err();
    assert!(
        matches!(bare_null.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }),
        "{bare_null}"
    );
}
