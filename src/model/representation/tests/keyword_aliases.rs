//! The keywords `@id` and `@type`, which a request may write instead of `id` and `type` (clause 4.4): read in every
//! representation and in fragments and temporal entities, written back as the terms.

use crate::model::{
    Complete,
    DocumentErrorKind,
    Entity,
    EntityFragment,
    Input,
    Lossless,
    MemberError,
    Output,
    Representation,
    SimplifiedEntity,
    evolution::TemporalEntity,
};
use pretty_assertions::assert_eq;

/// A building whose height is observed by a device, `id` and `type` written as the keywords.
const WITH_KEYWORDS: &str = concat!(
    r#"{"@id":"urn:ngsi-ld:Building:B1","@type":"Building","#,
    r#""height":{"@type":"Property","value":12,"observedBy":{"@type":"Relationship","object":"urn:ngsi-ld:Device:D1"}}}"#
);

/// The same building, written as the terms.
const WITH_TERMS: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Building:B1","type":"Building","#,
    r#""height":{"type":"Property","value":12,"observedBy":{"type":"Relationship","object":"urn:ngsi-ld:Device:D1"}}}"#
);

#[test]
fn a_request_may_write_the_keywords_instead_of_the_terms_in_both_forms() {
    for form in [Lossless::Normalized, Lossless::Concise] {
        let aliased = Entity::<Input>::parse(WITH_KEYWORDS, form).unwrap();
        assert_eq!(aliased, Entity::<Input>::parse(WITH_TERMS, form).unwrap(), "{form:?}");
        assert_eq!(aliased.render(Representation::Normalized).to_string(), WITH_TERMS, "{form:?}");
    }
}

#[test]
fn a_concise_attribute_with_several_instances_may_write_the_type_keyword() {
    let text = |key: &str| {
        format!(
            r#"{{"id":"urn:ngsi-ld:T:1","type":"T","x":[{{"{key}":"Property","value":1,"datasetId":"urn:a"}},{{"{key}":"Property","value":2,"datasetId":"urn:b"}}]}}"#
        )
    };
    let aliased = Entity::<Input>::parse(&text("@type"), Lossless::Concise).unwrap();
    assert_eq!(aliased, Entity::<Input>::parse(&text("type"), Lossless::Concise).unwrap());
}

#[test]
fn a_value_object_keeps_the_type_keyword_of_its_datatype() {
    let text = r#"{"id":"urn:ngsi-ld:T:1","type":"T","at":{"@type":"DateTime","@value":"2018-12-04T12:00:00Z"}}"#;
    let entity = Entity::<Input>::parse(text, Lossless::Concise).unwrap();
    let written = entity.render(Representation::Normalized).to_string();
    assert!(written.contains(r#""value":{"@type":"DateTime","@value":"2018-12-04T12:00:00Z"}"#), "{written}");
}

#[test]
fn a_term_beside_its_keyword_is_a_member_given_twice() {
    for (text, member) in [
        (r#"{"id":"urn:ngsi-ld:T:1","@id":"urn:ngsi-ld:T:2","type":"T"}"#, "id"),
        (r#"{"id":"urn:ngsi-ld:T:1","type":"T","@type":"U"}"#, "type"),
        (
            r#"{"id":"urn:ngsi-ld:T:1","type":"T","x":{"type":"Property","@type":"Property","value":1}}"#,
            "type",
        ),
    ] {
        let error = Entity::<Input>::parse(text, Lossless::Normalized).unwrap_err();
        assert!(
            matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { member: ref found }) if found == member),
            "{text}: {error}"
        );
    }
}

#[test]
fn an_attribute_is_still_no_place_for_the_id_keyword() {
    let text = r#"{"id":"urn:ngsi-ld:T:1","type":"T","x":{"type":"Property","@id":"urn:ngsi-ld:X:1","value":1}}"#;
    assert!(Entity::<Input>::parse(text, Lossless::Normalized).is_err());
}

#[test]
fn a_reader_of_a_response_takes_the_keywords_too() {
    // Clause 4.4 has a response write the terms only; the keywords are a reader's leniency and a writer never uses them.
    let read = Entity::<Output>::parse(WITH_KEYWORDS, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Representation::Normalized).to_string(), WITH_TERMS);
}

#[test]
fn a_fragment_may_write_the_keywords() {
    let fragment = r#"{"@id":"urn:ngsi-ld:Building:B1","@type":"Building","height":{"@type":"Property","value":13}}"#;
    let terms = r#"{"id":"urn:ngsi-ld:Building:B1","type":"Building","height":{"type":"Property","value":13}}"#;
    for form in [Lossless::Normalized, Lossless::Concise] {
        let aliased = EntityFragment::<Complete>::parse(fragment, form).unwrap();
        assert_eq!(aliased, EntityFragment::<Complete>::parse(terms, form).unwrap(), "{form:?}");
        assert_eq!(aliased.render(Lossless::Normalized).to_string(), terms, "{form:?}");
    }
}

#[test]
fn a_temporal_entity_and_its_instances_may_write_the_keywords() {
    let aliased = concat!(
        r#"{"@id":"urn:ngsi-ld:Building:B1","@type":"Building","height":["#,
        r#"{"@type":"Property","value":1,"observedAt":"2020-01-01T00:00:00Z"},"#,
        r#"{"@type":"Property","value":2,"observedAt":"2020-01-02T00:00:00Z"}]}"#
    );
    let terms = aliased.replace("@id", "id").replace("@type", "type");
    for form in [Lossless::Normalized, Lossless::Concise] {
        let read = TemporalEntity::<Input>::parse(aliased, form).unwrap();
        assert_eq!(read, TemporalEntity::<Input>::parse(&terms, form).unwrap(), "{form:?}");
    }
    let read = TemporalEntity::<Input>::parse(aliased, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Lossless::Normalized).to_string(), terms);
}

#[test]
fn a_simplified_entity_may_write_the_keywords() {
    let read = SimplifiedEntity::parse(r#"{"@id":"urn:ngsi-ld:Vehicle:A4567","@type":"Vehicle","speed":80}"#).unwrap();
    assert_eq!(read.render().to_string(), r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":80}"#);
}
