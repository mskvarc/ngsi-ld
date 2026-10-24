//! The snapshot operations (NGSI-LD clause 5.16), bound to `snapshots`, `snapshots/{snapshotId}` and
//! `snapshots/{snapshotId}/clone` (clauses 6.36 to 6.38): one request type per operation, and how their responses
//! are read.

use crate::{
    http::{BodyError, operation::stored_document::DocumentReader},
    model::Output,
    snapshot::{Problem, Snapshot},
    subscription::ReadErrorKind,
};

pub(super) mod clone;
pub(super) mod create;
pub(super) mod delete;
pub(super) mod purge;
pub(super) mod retrieve;
pub(super) mod update;

/// Reads the snapshot a 200 answers with (Tables 6.37.3.1-1 and 6.37.3.2-1).
const READER: DocumentReader<Snapshot<Output>, ReadErrorKind<Problem>> = DocumentReader {
    read: Snapshot::from_json,
    error: BodyError::Snapshot,
};
