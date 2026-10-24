//! Snapshots, their query documents and their notifications as linked documents (NGSI-LD clauses 5.2.23, 5.2.41
//! and 5.3.4).

use crate::{
    linked_data::document::{
        printed,
        sealed::Sealed,
        shape::{ARRAY, Shape},
    },
    model::DocumentError,
    seal::Seal,
    snapshot::{Problem as SnapshotProblem, QueryDocument, QueryKind, Snapshot, SnapshotDirection, SnapshotNotification, query_document::reader::read_query},
    subscription::ReadErrorKind,
};
use jstrict::Value as Json;

/// The array members of a query document (Table 5.2.23-1) and of its `ordering` (Table 5.2.43-1).
const QUERY: Shape = Shape::Object(&[
    ("entities", ARRAY),
    ("attrs", ARRAY),
    ("pick", ARRAY),
    ("omit", ARRAY),
    ("containedBy", ARRAY),
    ("datasetId", ARRAY),
    ("ordering", Shape::Object(&[("orderBy", ARRAY)])),
]);

/// The array members of a snapshot (Table 5.2.41-1).
const SNAPSHOT: Shape = Shape::Object(&[
    ("snapshotQueries", Shape::Array(&QUERY)),
    ("snapshotTemporalQueries", Shape::Array(&QUERY)),
    ("receiverInfo", ARRAY),
    ("snapshotQueriesDetails", ARRAY),
    ("snapshotTemporalQueriesDetails", ARRAY),
]);

impl<D: SnapshotDirection> Sealed for Snapshot<D> {
    type ReadError = DocumentError<ReadErrorKind<SnapshotProblem>>;

    const SHAPE: Shape = SNAPSHOT;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

/// A query document carries no `@context` of its own.
impl<K: QueryKind> Sealed for QueryDocument<K> {
    type ReadError = DocumentError<ReadErrorKind<SnapshotProblem>>;

    const SHAPE: Shape = QUERY;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        read_query(json)
    }
}

impl Sealed for SnapshotNotification {
    type ReadError = DocumentError<ReadErrorKind<SnapshotProblem>>;

    const SHAPE: Shape = Shape::Object(&[("snapshotQueriesDetails", ARRAY), ("temporalSnapshotQueriesDetails", ARRAY)]);

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}
