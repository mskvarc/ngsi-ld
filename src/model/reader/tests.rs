//! The shared member readers.

use crate::model::{
    DocumentError,
    DocumentErrorKind,
    JsonType,
    MemberError,
    Representation,
    reader::{array, count, object, once, parse_json, string, token},
};
use jstrict::Value as Json;

/// The kind of an error, when there is one.
fn kind<T>(read: Result<T, DocumentError>) -> Option<DocumentErrorKind> {
    read.err().map(|error| error.kind)
}

#[test]
fn rd_01_a_string_member_of_another_type_is_the_wrong_shape() {
    assert!(matches!(
        kind(string(Json::from(3), "description")),
        Some(DocumentErrorKind::Member(MemberError::WrongShape {
            member: "description",
            expected: JsonType::String,
        }))
    ));
}

#[test]
fn rd_02_a_count_is_a_whole_number_of_zero_or_more() {
    for text in ["-1", "1.5"] {
        let read = text.parse::<Json>().map(|json| kind(count(json, "entityCount")));
        assert!(
            matches!(
                read,
                Ok(Some(DocumentErrorKind::Member(MemberError::WrongShape {
                    member: "entityCount",
                    expected: JsonType::Count,
                })))
            ),
            "{text}"
        );
    }
    assert!(matches!(count::<DocumentErrorKind>(Json::from(0), "entityCount"), Ok(0)));
}

#[test]
fn rd_03_a_second_occurrence_of_a_member_is_refused() {
    let mut slot = Some(1);
    assert!(matches!(
        kind(once(&mut slot, "q", 2)),
        Some(DocumentErrorKind::Member(MemberError::DuplicateMember { member })) if member == "q"
    ));
    assert_eq!(slot, Some(1));
}

#[test]
fn rd_04_text_objects_and_arrays_are_checked() {
    assert!(matches!(kind(parse_json("{")), Some(DocumentErrorKind::Member(MemberError::NotJson))));
    assert!(matches!(
        kind(object(Json::Array(jstrict::Array::new()))),
        Some(DocumentErrorKind::Member(MemberError::NotAnObject))
    ));
    assert!(matches!(
        kind(array(Json::Object(jstrict::Object::new()))),
        Some(DocumentErrorKind::Member(MemberError::NotAnArray))
    ));
}

#[test]
fn rd_05_a_token_the_specification_does_not_list_is_refused() {
    assert!(matches!(
        kind(token::<Representation, _>(Json::from("maybe"), "format")),
        Some(DocumentErrorKind::Member(MemberError::InvalidToken { member: "format", found })) if found == "maybe"
    ));
}
