//! Snapshots in both directions.

use crate::{
    model::{BuildError, ErrorType, Input, KeyValuePair, MemberError, MemberProblem, Output},
    query::{EntitySelector, q, temporal_query::TemporalQuery},
    snapshot::{
        ExecutionStatus,
        Problem,
        QueryDocument,
        SettledStatus,
        Snapshot,
        SnapshotField,
        SnapshotId,
        SnapshotMember,
        SnapshotPriority,
        SnapshotStatus,
        tests::{OUTPUT_MEMBERS, S, SNAP, missing, out, refused, unexpected, without},
    },
    subscription::ReadErrorKind,
};
use iri_rs::iri;
use pretty_assertions::assert_eq;
use std::fmt::Debug;
use url::Url;

/// The snapshot of `SNAP`, built.
fn built() -> Snapshot<Input> {
    let vehicles = || EntitySelector::new("Vehicle".parse().unwrap());
    let fast = QueryDocument::builder()
        .entities([vehicles()])
        .q(q::attr("speed").gt(50).build().unwrap())
        .build()
        .unwrap();
    let history = QueryDocument::temporal_builder(TemporalQuery::after("2024-05-01T10:00:00Z".parse().unwrap()))
        .entities([vehicles()])
        .build()
        .unwrap();
    Snapshot::builder()
        .id(iri!("urn:ngsi-ld:Snapshot:1"))
        .query(fast)
        .temporal_query(history)
        .lifetime("PT1H".parse().unwrap())
        .priority(7)
        .endpoint("http://my.endpoint.org/snapshots")
        .receiver_info([KeyValuePair::new("Authorization", "Bearer t")])
        .build()
        .unwrap()
}

/// The problems of a refused build.
fn problems<T: Debug>(build: Result<T, BuildError<Problem>>) -> Vec<Problem> {
    build.unwrap_err().problems().to_vec()
}

#[test]
fn a_built_snapshot_writes_snap_and_reads_back_equal() {
    let snapshot = built();
    assert_eq!(snapshot.render().to_string(), SNAP);
    assert_eq!(Snapshot::<Input>::parse(SNAP).unwrap(), snapshot);
    assert_eq!(snapshot.id().map(SnapshotId::as_str), Some(S));
    assert_eq!(snapshot.priority(), Some(SnapshotPriority::try_new(7).unwrap()));
    assert_eq!(snapshot.endpoint().map(Url::as_str), Some("http://my.endpoint.org/snapshots"));
}

#[test]
fn a_builder_reports_every_problem_together() {
    let found = problems(Snapshot::builder().priority(11).endpoint("not a url").build());
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.contains(&Problem::NoSnapshotQueries));
    assert!(found.iter().any(|problem| matches!(problem, Problem::Priority { .. })));
    assert!(found.iter().any(|problem| matches!(
        problem,
        Problem::Member(MemberProblem::InvalidUrl {
            member: SnapshotField::Snapshot(SnapshotMember::Endpoint),
            ..
        })
    )));
}

#[test]
fn a_returned_snapshot_round_trips_with_its_output_members() {
    let text = out();
    let snapshot = Snapshot::<Output>::parse(&text).unwrap();
    assert_eq!(snapshot.render().to_string(), text);
    assert_eq!(snapshot.status(), SnapshotStatus::Settled(SettledStatus::Success));
    assert_eq!(snapshot.expires_at().to_string(), "2030-01-01T00:00:00Z");
    assert_eq!(
        (snapshot.queries().map(<[_]>::len), snapshot.temporal_queries().map(<[_]>::len)),
        (Some(1), Some(1))
    );
    let details = &snapshot.output().temporal_queries_details[0];
    assert_eq!(details.result_status, ExecutionStatus::Failure);
    assert_eq!(
        details.problem_details.as_ref().and_then(|problem| problem.error_type.clone()),
        Some(ErrorType::BadRequestData)
    );
    assert_eq!(snapshot.output().queries_details[0].result_status, ExecutionStatus::Success);
}

#[test]
fn a_returned_snapshot_needs_its_status_expiry_and_id() {
    let text = out();
    for member in ["snapshotStatus", "expiresAt", "id"] {
        let error = refused(Snapshot::<Output>::parse(&without(&text, member)));
        assert!(missing(&error, member), "{member}: {error:?}");
    }
}

#[test]
fn a_request_refuses_the_members_only_a_broker_writes() {
    let base = SNAP.trim_end_matches('}');
    for (member, value) in [("snapshotStatus", "\"success\""), ("lastUsedAt", "\"2030-01-01T00:00:00Z\"")] {
        let error = refused(Snapshot::<Input>::parse(&format!("{base},\"{member}\":{value}}}")));
        assert!(unexpected(&error, member), "{member}: {error:?}");
    }
    assert!(OUTPUT_MEMBERS.contains("lastUsedAt"));
}

#[test]
fn each_kind_of_query_keeps_to_its_list() {
    let with_temporal = SNAP.replace(
        r#""q":"speed>50"}"#,
        r#""q":"speed>50","temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"}}"#,
    );
    let error = refused(Snapshot::<Input>::parse(&with_temporal));
    assert!(unexpected(&error, "temporalQ"), "{error:?}");
    assert_eq!(error.path.to_string(), "snapshotQueries.0");
    let with_aggregation = SNAP.replace(r#""q":"speed>50"}"#, r#""q":"speed>50","aggrParams":{"aggrMethods":"max"}}"#);
    let error = refused(Snapshot::<Input>::parse(&with_aggregation));
    assert!(unexpected(&error, "aggrParams"), "{error:?}");
    assert_eq!(error.path.to_string(), "snapshotQueries.0");
    let without_temporal = SNAP.replace(r#","temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"}"#, "");
    let error = refused(Snapshot::<Input>::parse(&without_temporal));
    assert!(missing(&error, "temporalQ"), "{error:?}");
    assert_eq!(error.path.to_string(), "snapshotTemporalQueries.0");
}

#[test]
fn only_a_creation_needs_a_list_of_queries() {
    let error = refused(Snapshot::<Input>::parse(r#"{"type":"Snapshot"}"#));
    assert!(
        matches!(&error.kind, ReadErrorKind::Invalid { source } if source.problems() == [Problem::NoSnapshotQueries]),
        "{error:?}"
    );
    let preparing = r#"{"id":"urn:ngsi-ld:Snapshot:1","type":"Snapshot","snapshotStatus":"preparing","expiresAt":"2030-01-01T00:00:00Z"}"#;
    let snapshot = Snapshot::<Output>::parse(preparing).unwrap();
    assert_eq!(snapshot.status(), SnapshotStatus::Preparing);
    assert_eq!(snapshot.render().to_string(), preparing);
    assert_eq!(snapshot.queries(), None);
}

/// Table 5.2.41-1 sets no minimum length for either list, and "at least one of `snapshotQueries` or
/// `snapshotTemporalQueries` shall be present" is met by a present, empty one.
#[test]
fn an_empty_list_of_queries_is_present_and_round_trips() {
    for (member, other) in [("snapshotQueries", "snapshotTemporalQueries"), ("snapshotTemporalQueries", "snapshotQueries")] {
        let text = format!(r#"{{"type":"Snapshot","{member}":[]}}"#);
        let snapshot = Snapshot::<Input>::parse(&text).unwrap();
        assert_eq!(snapshot.render().to_string(), text, "{member}");
        let (present, absent) = match member {
            "snapshotQueries" => (snapshot.queries().map(<[_]>::len), snapshot.temporal_queries().map(<[_]>::len)),
            _ => (snapshot.temporal_queries().map(<[_]>::len), snapshot.queries().map(<[_]>::len)),
        };
        assert_eq!((present, absent), (Some(0), None), "{member} beside an absent {other}");
    }
    let built = Snapshot::builder().queries([]).build().unwrap();
    assert_eq!(built.render().to_string(), r#"{"type":"Snapshot","snapshotQueries":[]}"#);
    let built = Snapshot::builder().temporal_queries([]).build().unwrap();
    assert_eq!(built.render().to_string(), r#"{"type":"Snapshot","snapshotTemporalQueries":[]}"#);
}

#[test]
fn statuses_and_priorities_hold_to_their_tables() {
    let text = out();
    for (token, status) in [
        ("partial", SettledStatus::Partial),
        ("failure", SettledStatus::Failure),
        ("empty", SettledStatus::Empty),
    ] {
        let snapshot = Snapshot::<Output>::parse(&text.replace(r#""snapshotStatus":"success""#, &format!(r#""snapshotStatus":"{token}""#))).unwrap();
        assert_eq!(snapshot.status(), SnapshotStatus::Settled(status));
    }
    let error = refused(Snapshot::<Output>::parse(
        &text.replace(r#""snapshotStatus":"success""#, r#""snapshotStatus":"done""#),
    ));
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "snapshotStatus", .. })),
        "{error:?}"
    );
    for (priority, accepted) in [("0", false), ("10", true), ("11", false), ("2.5", false)] {
        let read = Snapshot::<Input>::parse(&SNAP.replace(r#""snapshotPriority":7"#, &format!(r#""snapshotPriority":{priority}"#)));
        assert_eq!(read.is_ok(), accepted, "{priority}");
    }
    assert!(SnapshotPriority::try_new(1).is_ok());
    assert!(SnapshotPriority::try_new(10).is_ok());
    assert!(SnapshotPriority::try_new(0).is_err());
    assert!(SnapshotPriority::try_new(11).is_err());
}

#[test]
fn a_snapshot_request_may_write_the_keywords_for_its_type_and_those_of_its_queries() {
    let aliased = r#"{"@type":"Snapshot","snapshotQueries":[{"@type":"Query","entities":[{"@type":"Vehicle"}]}]}"#;
    let terms = aliased.replace("@type", "type");
    assert_eq!(Snapshot::<Input>::parse(aliased).unwrap().render().to_string(), terms);
}
