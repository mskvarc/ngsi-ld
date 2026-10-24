//! The `OrderingParams` object of clause 5.2.43.

use crate::{
    model::{DocumentError, MemberError, geometry::Shape},
    query::{
        Problem,
        ordering::{EntityOrdering, OrderingParamsError},
    },
};
use langtag::LangTagBuf;
use pretty_assertions::assert_eq;

/// Reads the text of an `OrderingParams` object.
fn read(text: &str) -> Result<EntityOrdering, DocumentError<OrderingParamsError>> {
    EntityOrdering::parse(text)
}

/// Reads and writes back the text of an `OrderingParams` object.
fn round_trip(text: &str) -> String {
    read(text).unwrap().render().to_string()
}

#[test]
fn text_that_is_not_json_is_refused() {
    assert!(matches!(
        read("{orderBy").map_err(|error| error.kind),
        Err(OrderingParamsError::Member(MemberError::NotJson))
    ));
}

#[test]
fn an_ordering_with_a_line_origin_round_trips() {
    let text =
        r#"{"orderBy":["name","location;dist-asc"],"collation":"und-u-ks-identic","geometry":"LineString","coordinates":[[13.3986,52.5547],[13.4,52.6]]}"#;
    assert_eq!(round_trip(text), text);
    let ordering = read(text).unwrap();
    assert_eq!(ordering.order_by().to_string(), "name,location;dist-asc");
    assert_eq!(ordering.collation().map(LangTagBuf::as_str), Some("und-u-ks-identic"));
}

#[test]
fn an_origin_without_geometry_is_a_point_written_without_it() {
    let text = r#"{"orderBy":["location;dist-desc"],"coordinates":[8,40]}"#;
    assert!(read(text).unwrap().origin().is_some_and(Shape::is_point));
    assert_eq!(round_trip(text), text);
}

#[test]
fn an_ordering_needs_its_terms_and_an_origin_to_sort_by_distance() {
    let kind = |text: &str| read(text).map_err(|error| error.kind);
    assert!(matches!(
        kind(r#"{"collation":"de"}"#),
        Err(OrderingParamsError::Member(MemberError::MissingMember { member: "orderBy" }))
    ));
    match kind(r#"{"orderBy":["location;dist-asc"]}"#) {
        Err(OrderingParamsError::Ordering { source }) => assert_eq!(source.problems(), [Problem::DistanceWithoutOrigin]),
        other => panic!("{other:?}"),
    }
    let not_an_array = read(r#"{"orderBy":"name"}"#).unwrap_err();
    assert!(matches!(not_an_array.kind, OrderingParamsError::Member(MemberError::NotAnArray)));
    assert_eq!(not_an_array.path.to_string(), "orderBy");
    let not_a_string = read(r#"{"orderBy":["name",3]}"#).unwrap_err();
    assert!(matches!(
        not_a_string.kind,
        OrderingParamsError::Member(MemberError::WrongShape { member: "orderBy", .. })
    ));
    assert_eq!(not_a_string.path.to_string(), "orderBy.1");
    assert!(matches!(
        kind(r#"{"orderBy":["name"],"geometry":"Point"}"#),
        Err(OrderingParamsError::Member(MemberError::MissingMember { member: "coordinates" }))
    ));
    assert!(matches!(
        kind(r#"{"orderBy":["name"],"from":1}"#),
        Err(OrderingParamsError::Member(MemberError::UnexpectedMember { .. }))
    ));
}
