//! Snapshot notifications.

use crate::{
    model::MemberError,
    snapshot::{
        ExecutionStatus,
        SettledStatus,
        SnapshotNotification,
        tests::{S, missing, refused, unexpected, without},
    },
    subscription::{DocumentType, ReadErrorKind},
};
use pretty_assertions::assert_eq;

/// A snapshot notification with every mandatory member and the details of one query.
const NOTE: &str = r#"{"id":"urn:ngsi-ld:Notification:1","type":"SnapshotNotification","expiresAt":"2030-01-01T00:00:00Z","notifiedAt":"2024-05-01T10:05:00Z","snapshotId":"urn:ngsi-ld:Snapshot:1","snapshotPriority":5,"snapshotQueriesDetails":[{"resultStatus":"success"}],"snapshotStatus":"success"}"#;

#[test]
fn a_notification_round_trips() {
    let notification = SnapshotNotification::parse(NOTE).unwrap();
    assert_eq!(notification.render().to_string(), NOTE);
    assert_eq!(notification.status(), SettledStatus::Success);
    assert_eq!(notification.priority().into_inner(), 5);
    assert_eq!(notification.snapshot_id().as_str(), S);
    assert_eq!(notification.queries_details().len(), 1);
    assert_eq!(notification.queries_details()[0].result_status, ExecutionStatus::Success);
    assert_eq!(notification.temporal_queries_details(), []);
    assert!(!notification.announces_deletion());
}

#[test]
fn a_notification_reports_a_settled_status_only() {
    for (token, status) in [
        ("partial", SettledStatus::Partial),
        ("failure", SettledStatus::Failure),
        ("empty", SettledStatus::Empty),
    ] {
        let text = NOTE.replace(r#""snapshotStatus":"success""#, &format!(r#""snapshotStatus":"{token}""#));
        assert_eq!(SnapshotNotification::parse(&text).unwrap().status(), status);
    }
    let error = refused(SnapshotNotification::parse(
        &NOTE.replace(r#""snapshotStatus":"success""#, r#""snapshotStatus":"preparing""#),
    ));
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "snapshotStatus", .. })),
        "{error:?}"
    );
}

#[test]
fn an_expiry_before_the_notification_announces_the_deletion() {
    let text = NOTE
        .replace(r#""expiresAt":"2030-01-01T00:00:00Z""#, r#""expiresAt":"2024-05-01T10:00:00Z""#)
        .replace(
            r#""snapshotStatus":"success"}"#,
            r#""snapshotStatus":"success","temporalSnapshotQueriesDetails":[{"resultStatus":"empty"}]}"#,
        );
    let notification = SnapshotNotification::parse(&text).unwrap();
    assert!(notification.announces_deletion());
    assert_eq!(notification.temporal_queries_details().len(), 1);
    assert_eq!(notification.render().to_string(), text);
}

#[test]
fn a_notification_holds_the_members_of_its_table_only() {
    for member in ["snapshotId", "snapshotPriority"] {
        let error = refused(SnapshotNotification::parse(&without(NOTE, member)));
        assert!(missing(&error, member), "{member}: {error:?}");
    }
    let error = refused(SnapshotNotification::parse(&NOTE.replace(r#","snapshotStatus":"success""#, "")));
    assert!(missing(&error, "snapshotStatus"), "{error:?}");
    let base = NOTE.trim_end_matches('}');
    for (member, value) in [("snapshotTemporalQueriesDetails", "[]"), ("snapshotReady", "false")] {
        let error = refused(SnapshotNotification::parse(&format!("{base},\"{member}\":{value}}}")));
        assert!(unexpected(&error, member), "{member}: {error:?}");
    }
    let error = refused(SnapshotNotification::parse(&NOTE.replace("SnapshotNotification", "Notification")));
    assert!(
        matches!(
            error.kind,
            ReadErrorKind::WrongType {
                expected: DocumentType::SnapshotNotification,
                ..
            }
        ),
        "{error:?}"
    );
}
