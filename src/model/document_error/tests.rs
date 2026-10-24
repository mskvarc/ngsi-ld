//! The paths and messages of a document error.

use crate::model::{DocumentError, DocumentErrorKind, MemberError, PathSegment};
use pretty_assertions::assert_eq;

/// An error at the top of a document.
fn top(kind: MemberError) -> DocumentError {
    DocumentError::top(DocumentErrorKind::from(kind))
}

#[test]
fn de_01_nested_members_are_written_outermost_first() {
    let error = top(MemberError::NotAnObject).within("endpoint").within("notification");
    assert_eq!(error.to_string(), "expected a JSON object at notification.endpoint");
    assert_eq!(
        error.path.segments().collect::<Vec<_>>(),
        [&PathSegment::Member("notification".to_owned()), &PathSegment::Member("endpoint".to_owned())]
    );
}

#[test]
fn de_02_an_array_element_is_a_typed_index() {
    let error = top(MemberError::NotAnObject).at_index(1).within("entities");
    assert_eq!(error.path.to_string(), "entities.1");
    assert_eq!(
        error.path.segments().collect::<Vec<_>>(),
        [&PathSegment::Member("entities".to_owned()), &PathSegment::Index(1)]
    );
    assert_eq!(error.path.segments().len(), 2);
    assert_eq!(error.path.segments().next_back(), Some(&PathSegment::Index(1)));
}

#[test]
fn de_03_an_error_at_the_top_has_no_location() {
    let error = top(MemberError::NotJson);
    assert!(error.path.is_empty());
    assert_eq!(error.to_string(), "the document is not valid JSON");
}
