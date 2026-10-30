//! Unit tests of snapshots, their changes and snapshot notifications.

use crate::{
    model::{DocumentError, MemberError},
    snapshot::Problem as SnapshotProblem,
    subscription::ReadErrorKind,
};
use std::fmt::Debug;

mod changes;
mod documents;
mod notifications;
mod verbatim;

/// The id of `SNAP`.
const S: &str = "urn:ngsi-ld:Snapshot:1";

/// A snapshot with every member a client sends.
const SNAP: &str = r#"{"id":"urn:ngsi-ld:Snapshot:1","type":"Snapshot","snapshotQueries":[{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50"}],"snapshotTemporalQueries":[{"type":"Query","entities":[{"type":"Vehicle"}],"temporalQ":{"timerel":"after","timeAt":"2024-05-01T10:00:00Z"}}],"snapshotLifetime":"PT1H","snapshotPriority":7,"endpoint":"http://my.endpoint.org/snapshots","receiverInfo":[{"key":"Authorization","value":"Bearer t"}]}"#;

/// The members `OUT` adds to `SNAP`, the ones only a broker writes, and an `@context`.
const OUTPUT_MEMBERS: &str = r#""snapshotStatus":"success","snapshotQueriesDetails":[{"resultStatus":"success"}],"snapshotTemporalQueriesDetails":[{"problemDetails":{"type":"https://uri.etsi.org/ngsi-ld/errors/BadRequestData","title":"Bad temporal query"},"resultStatus":"failure"}],"createdAt":"2024-05-01T10:00:00Z","modifiedAt":"2024-05-01T10:05:00Z","expiresAt":"2030-01-01T00:00:00Z","lastUsedAt":"2024-05-01T10:05:00Z","@context":"https://example.org/context.jsonld""#;

/// `SNAP` as a broker returns it, with `OUTPUT_MEMBERS` added.
fn out() -> String {
    format!("{},{OUTPUT_MEMBERS}}}", SNAP.trim_end_matches('}'))
}

/// `text` with `member` and its value removed; the member must be followed by a `,`.
fn without(text: &str, member: &str) -> String {
    let start = text.find(&format!("\"{member}\":")).unwrap();
    let value_end = start + text[start..].find(",\"").unwrap() + 1;
    format!("{}{}", &text[..start], &text[value_end..])
}

/// The error of a read that must fail.
fn refused<T: Debug>(read: Result<T, DocumentError<ReadErrorKind<SnapshotProblem>>>) -> DocumentError<ReadErrorKind<SnapshotProblem>> {
    read.unwrap_err()
}

/// Whether an error names `member` as missing.
fn missing(error: &DocumentError<ReadErrorKind<SnapshotProblem>>, member: &str) -> bool {
    matches!(&error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: found }) if *found == member)
}

/// Whether an error names `member` as unexpected.
fn unexpected(error: &DocumentError<ReadErrorKind<SnapshotProblem>>, member: &str) -> bool {
    matches!(&error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member: found }) if found == member)
}
