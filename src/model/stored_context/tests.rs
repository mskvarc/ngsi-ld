//! Stored `@context` documents and their metadata (clause 5.13): reading, writing and each refusal.

use crate::model::{
    Context,
    ContextEntry,
    ContextMetadata,
    DocumentError,
    DocumentErrorKind,
    ExtraMemberName,
    ExtraMemberNameError,
    InlineContext,
    JsonType,
    LocalContext,
    MemberError,
    StoredContextKind,
    StoredContextKindError,
    Value,
    ValueError,
};
use iri_rs::IriRefBuf;
use pretty_assertions::assert_eq;
use url::Url;

/// The metadata of a cached remote context, with every member.
const CACHED: &str = concat!(
    r#"{"URL":"https://example.org/context.jsonld","localId":"urn:ngsi-ld:context:c1","kind":"Cached","#,
    r#""createdAt":"2024-05-01T10:00:00Z","expiresAt":"2024-05-02T10:00:00Z","lastUsage":"2024-05-01T11:00:00Z","#,
    r#""numberOfHits":3,"extraInfo":{"etag":"x1"}}"#
);

/// The metadata of a context the broker hosts, with the mandatory members only.
const HOSTED: &str = concat!(
    r#"{"URL":"http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1","localId":"urn:ngsi-ld:context:c1","#,
    r#""kind":"Hosted","createdAt":"2024-05-01T10:00:00Z"}"#
);

/// `inline`.
fn inline() -> Context {
    Context::Single(ContextEntry::Inline(
        r#"{"speed":"https://example.org/speed"}"#.parse::<InlineContext>().unwrap(),
    ))
}

/// The remote context, as an IRI reference.
fn remote_reference() -> IriRefBuf {
    IriRefBuf::new(remote().as_str()).unwrap()
}

fn remote() -> Url {
    Url::parse("https://example.org/context.jsonld").unwrap()
}

/// The kind of an error.
fn kind(error: DocumentError) -> DocumentErrorKind {
    error.kind
}

#[test]
fn the_metadata_of_a_cached_context_reads_every_member() {
    let metadata = ContextMetadata::parse(CACHED).unwrap();
    assert_eq!(metadata.url, remote());
    assert_eq!(metadata.local_id.as_str(), "urn:ngsi-ld:context:c1");
    assert_eq!(metadata.kind, StoredContextKind::Cached);
    assert_eq!(metadata.created_at.to_string(), "2024-05-01T10:00:00Z");
    assert_eq!(metadata.expires_at.map(|at| at.to_string()).as_deref(), Some("2024-05-02T10:00:00Z"));
    assert_eq!(metadata.last_usage.map(|at| at.to_string()).as_deref(), Some("2024-05-01T11:00:00Z"));
    assert_eq!(metadata.number_of_hits, Some(3));
    assert_eq!(metadata.extra_info.as_ref().map(ToString::to_string).as_deref(), Some(r#"{"etag":"x1"}"#));
    assert_eq!(metadata.render().to_string(), CACHED);
}

#[test]
fn the_metadata_of_a_hosted_context_needs_no_optional_member() {
    let metadata = ContextMetadata::parse(HOSTED).unwrap();
    assert_eq!(metadata.kind, StoredContextKind::Hosted);
    assert_eq!(
        (metadata.expires_at, metadata.last_usage, metadata.number_of_hits, metadata.extra_info.is_none()),
        (None, None, None, true)
    );
    assert_eq!(ContextMetadata::parse(HOSTED).unwrap().render().to_string(), HOSTED);
}

#[test]
fn metadata_out_of_shape_is_refused_at_its_member() {
    let error = ContextMetadata::parse(&HOSTED.replace("\"kind\":\"Hosted\"", "\"kind\":\"\"")).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::InvalidToken { member: "kind", found: ref token }) if token.is_empty()));
    assert_eq!(error.path.to_string(), "kind");
    let without_id = HOSTED.replace(r#""localId":"urn:ngsi-ld:context:c1","#, "");
    assert!(matches!(
        kind(ContextMetadata::parse(&without_id).unwrap_err()),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "localId" })
    ));
    let error = ContextMetadata::parse(&HOSTED.replace("http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1", "not a url")).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::Url { .. })));
    assert_eq!(error.path.to_string(), "URL");
    let hits = ContextMetadata::parse(&CACHED.replace(r#""numberOfHits":3"#, r#""numberOfHits":-1"#)).unwrap_err();
    assert!(matches!(
        hits.kind,
        DocumentErrorKind::Member(MemberError::WrongShape {
            member: "numberOfHits",
            expected: JsonType::Count
        })
    ));
    let foo = ContextMetadata::parse(&HOSTED.replace(r#""kind""#, r#""foo":1,"kind""#)).unwrap_err();
    assert!(matches!(foo.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "foo"));
}

#[test]
fn sc03b_a_kind_clause_5_13_1_does_not_define_is_kept_as_written() {
    for found in ["Remote", "ImplicitlyGenerated"] {
        let text = HOSTED.replace("Hosted", found);
        let metadata = ContextMetadata::parse(&text).unwrap();
        assert!(matches!(&metadata.kind, StoredContextKind::Other(other) if other.as_ref() == found));
        assert_eq!(metadata.kind.to_string(), found);
        assert_eq!(metadata.render().to_string(), text);
    }
    assert_eq!("".parse::<StoredContextKind>(), Err(StoredContextKindError::Empty));
}

#[test]
fn the_kinds_are_written_as_clause_5_13_1_spells_them() {
    for (text, kind) in [
        ("Cached", StoredContextKind::Cached),
        ("Hosted", StoredContextKind::Hosted),
        ("ImplicitlyCreated", StoredContextKind::ImplicitlyCreated),
    ] {
        assert_eq!(kind.to_string(), text);
        assert_eq!(text.parse(), Ok(kind));
    }
}

#[test]
fn the_local_context_document_holds_the_context_under_its_root_node() {
    let cases = [
        (inline(), r#"{"@context":{"speed":"https://example.org/speed"}}"#),
        (Context::from(remote_reference()), r#"{"@context":"https://example.org/context.jsonld"}"#),
        (
            Context::Multiple(vec![
                ContextEntry::Reference(remote_reference()),
                ContextEntry::Inline(r#"{"speed":"https://example.org/speed"}"#.parse().unwrap()),
            ]),
            r#"{"@context":["https://example.org/context.jsonld",{"speed":"https://example.org/speed"}]}"#,
        ),
    ];
    for (context, text) in cases {
        assert_eq!(LocalContext::new(context.clone()).render().to_string(), text);
        assert_eq!(LocalContext::parse(text).unwrap().context, context);
    }
}

#[test]
fn a_local_context_document_keeps_the_members_beside_its_context_in_order() {
    let text = r#"{"@context":{"speed":"https://example.org/speed"},"note":"x","extra":{"n":[1,2]}}"#;
    let document = LocalContext::parse(text).unwrap();
    assert_eq!(document.context, inline());
    let names: Vec<&str> = document.extra_members.keys().map(AsRef::as_ref).collect();
    assert_eq!(names, ["note", "extra"]);
    assert_eq!(document.extra_members.get(&"note".parse::<ExtraMemberName>().unwrap()), Some(&Value::from("x")));
    assert_eq!(document.render().to_string(), text);
    let mut built = LocalContext::new(inline());
    built.extra_members.insert("note".parse().unwrap(), Value::from("x"));
    assert_eq!(built.render().to_string(), r#"{"@context":{"speed":"https://example.org/speed"},"note":"x"}"#);
    let null = LocalContext::parse(r#"{"@context":{"speed":"https://example.org/speed"},"note":null}"#).unwrap_err();
    assert!(matches!(null.kind, DocumentErrorKind::Member(MemberError::Value { source: ValueError::Null })));
    assert_eq!(null.path.to_string(), "note");
    let unnamed = LocalContext::parse(r#"{"@context":{"speed":"https://example.org/speed"},"":1}"#).unwrap_err();
    assert!(matches!(unnamed.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member.is_empty()));
    assert!(matches!(
        kind(LocalContext::parse("{}").unwrap_err()),
        DocumentErrorKind::Member(MemberError::MissingMember { member: "@context" })
    ));
    assert!(matches!(
        kind(LocalContext::parse("[]").unwrap_err()),
        DocumentErrorKind::Member(MemberError::NotAnObject)
    ));
}

#[test]
fn a_member_beside_the_context_is_named_by_anything_but_the_context_and_the_empty_name() {
    assert_eq!("note".parse::<ExtraMemberName>().map(|name| name.to_string()), Ok("note".to_owned()));
    assert_eq!("@graph".parse::<ExtraMemberName>().map(|name| name.to_string()), Ok("@graph".to_owned()));
    assert_eq!("@context".parse::<ExtraMemberName>(), Err(ExtraMemberNameError::Context));
    assert_eq!("".parse::<ExtraMemberName>(), Err(ExtraMemberNameError::Empty));
}
