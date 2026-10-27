//! The keywords `@id` and `@type` read as the members `id` and `type`.

use crate::model::{
    DocumentError,
    DocumentErrorKind,
    MemberError,
    keyword_alias::{unalias_attribute, unalias_members},
};
use jstrict::{Object, Print, Value as Json};

/// The object a text holds.
fn object(text: &str) -> Object {
    match text.parse::<Json>().unwrap() {
        Json::Object(object) => object,
        other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => panic!("not an object: {other:?}"),
    }
}

/// The text of an object, compact.
fn text(object: Object) -> String {
    Json::Object(object).compact_print().to_string()
}

#[test]
fn both_keywords_are_read_as_their_terms_in_the_place_they_stand() {
    let read = unalias_members::<DocumentErrorKind>(object(r#"{"@id":"urn:ngsi-ld:T:1","speed":80,"@type":"T"}"#)).unwrap();
    assert_eq!(text(read), r#"{"id":"urn:ngsi-ld:T:1","speed":80,"type":"T"}"#);
}

#[test]
fn an_object_without_a_keyword_is_returned_as_it_is() {
    let read = unalias_members::<DocumentErrorKind>(object(r#"{"id":"urn:ngsi-ld:T:1","type":"T","@context":"https://example.org/c"}"#)).unwrap();
    assert_eq!(text(read), r#"{"id":"urn:ngsi-ld:T:1","type":"T","@context":"https://example.org/c"}"#);
}

#[test]
fn a_term_beside_its_keyword_is_a_member_given_twice() {
    for (body, member) in [
        (r#"{"id":"urn:ngsi-ld:T:1","@id":"urn:ngsi-ld:T:2"}"#, "id"),
        (r#"{"@type":"T","type":"U"}"#, "type"),
        (r#"{"@id":"urn:ngsi-ld:T:1","@id":"urn:ngsi-ld:T:2"}"#, "id"),
    ] {
        let error: DocumentError = unalias_members(object(body)).unwrap_err();
        assert!(
            matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { member: ref found }) if found == member),
            "{body}"
        );
    }
}

#[test]
fn the_type_of_an_attribute_is_read_when_it_names_a_kind() {
    let read = unalias_attribute::<DocumentErrorKind>(object(r#"{"@type":"Relationship","object":"urn:ngsi-ld:T:1"}"#)).unwrap();
    assert_eq!(text(read), r#"{"type":"Relationship","object":"urn:ngsi-ld:T:1"}"#);
}

#[test]
fn a_json_ld_value_object_keeps_its_datatype() {
    let value = r#"{"@type":"DateTime","@value":"2018-12-04T12:00:00Z"}"#;
    assert_eq!(text(unalias_attribute::<DocumentErrorKind>(object(value)).unwrap()), value);
}

#[test]
fn an_attribute_has_no_id_to_read() {
    let body = r#"{"type":"Property","@id":"urn:ngsi-ld:T:1","value":1}"#;
    assert_eq!(text(unalias_attribute::<DocumentErrorKind>(object(body)).unwrap()), body);
}

#[test]
fn a_type_beside_the_type_keyword_of_an_attribute_is_a_member_given_twice() {
    let error: DocumentError = unalias_attribute(object(r#"{"type":"Property","@type":"Relationship"}"#)).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "type"));
}
