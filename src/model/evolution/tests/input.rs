//! The simplified temporal representation a client sends, and turning an answer into a request.

use crate::model::{
    AttributesError,
    BuildError,
    DateTime,
    Lossless,
    NullMember,
    OneOrMany,
    Output,
    ProblemKind,
    Value,
    evolution::{SimplifiedTemporalInput, TemporalEntity, TemporalValue, TemporalValues},
    geometry::{Geometry, Position, Shape},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// Property values at the given times.
fn speeds(points: &[(i64, &str)]) -> TemporalValues {
    TemporalValues::Property(
        points
            .iter()
            .map(|&(value, at)| TemporalValue {
                value: Value::from(value),
                time: time(at),
            })
            .collect(),
    )
}

/// The problems of a build error, as their kinds.
fn kinds(error: &BuildError) -> Vec<String> {
    error.problems().iter().map(|problem| format!("{:?}", problem.kind)).collect()
}

#[test]
fn a_simplified_temporal_entity_is_built_and_written_with_its_scope() {
    let scope = TemporalValue {
        value: OneOrMany::one("/Madrid".parse().unwrap()),
        time: time("2018-08-01T12:00:00Z"),
    };
    let built = SimplifiedTemporalInput::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .scope(scope)
        .values("speed", speeds(&[(120, "2018-08-01T12:03:00Z"), (80, "2018-08-01T12:05:00Z")]))
        .build()
        .unwrap();
    assert_eq!(
        built.render().to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""scope":{"type":"Property","values":[["/Madrid","2018-08-01T12:00:00Z"]]},"#,
            r#""speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z"],[80,"2018-08-01T12:05:00Z"]]}}"#
        )
    );
    assert_eq!(built.as_entity().attributes.len(), 1);
}

#[test]
fn the_builder_reports_every_problem_together() {
    let deleted = TemporalValues::Property(vec![TemporalValue {
        value: Value::from("urn:ngsi-ld:null"),
        time: time("2018-08-01T12:03:00Z"),
    }]);
    let error = SimplifiedTemporalInput::builder()
        .id("not a uri")
        .entity_type("Vehicle")
        .values("1speed", speeds(&[(1, "2018-08-01T12:03:00Z")]))
        .values("speed", speeds(&[(1, "2018-08-01T12:03:00Z")]))
        .values("speed", speeds(&[(2, "2018-08-01T12:04:00Z")]))
        .values("location", speeds(&[(3, "2018-08-01T12:03:00Z")]))
        .values("rpm", deleted)
        .build()
        .unwrap_err();
    let problems = error.problems();
    assert_eq!(problems.len(), 5, "{:?}", kinds(&error));
    assert!(matches!(problems[0].kind, ProblemKind::Identifier { .. }));
    assert!(matches!(problems[1].kind, ProblemKind::AttributeName { .. }));
    assert!(matches!(
        problems[2].kind,
        ProblemKind::Attributes {
            source: AttributesError::ValuesNamedTwice,
            ..
        }
    ));
    assert!(matches!(
        problems[3].kind,
        ProblemKind::Attributes {
            source: AttributesError::GeoPropertyRequired,
            ..
        }
    ));
    assert!(matches!(problems[4].kind, ProblemKind::NgsiLdNull { member: NullMember::Value }));
    assert_eq!(problems[4].within().map(ToString::to_string).collect::<Vec<_>>(), ["rpm"]);
}

#[test]
fn a_geometry_series_is_accepted_under_location() {
    let point = Geometry::from(Shape::Point(Position::new(8.0, 40.0).unwrap()));
    let located = TemporalValues::GeoProperty(vec![TemporalValue {
        value: point.into(),
        time: time("2018-08-01T12:03:00Z"),
    }]);
    let built = SimplifiedTemporalInput::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .values("location", located)
        .build();
    assert!(built.is_ok());
}

#[test]
fn into_input_drops_what_only_a_broker_writes() {
    let read = TemporalEntity::<Output>::parse(
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","createdAt":"2018-08-01T11:00:00Z","#,
            r#""speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:attribute:instance:1","createdAt":"2018-08-01T12:03:01Z"}]}"#
        ),
        Lossless::Normalized,
    )
    .unwrap();
    let input = read.into_input().unwrap();
    assert_eq!(
        input.render(Lossless::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}]}"#
    );
}

#[test]
fn into_input_refuses_a_deleted_instance_and_a_removed_scope() {
    let read = TemporalEntity::<Output>::parse(
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""scope":[{"type":"Property","value":"urn:ngsi-ld:null","observedAt":"2018-08-01T12:00:00Z"}],"#,
            r#""speed":[{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:03:00Z"}]}"#
        ),
        Lossless::Normalized,
    )
    .unwrap();
    let error = read.into_input().unwrap_err();
    let problems = error.problems();
    assert_eq!(problems.len(), 2, "{:?}", kinds(&error));
    assert!(matches!(problems[0].kind, ProblemKind::NgsiLdNull { member: NullMember::Value }));
    assert_eq!(problems[0].within().map(ToString::to_string).collect::<Vec<_>>(), ["speed"]);
    assert!(matches!(problems[1].kind, ProblemKind::NgsiLdNull { member: NullMember::Scope }));
}
