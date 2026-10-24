//! Writing snapshot documents in canonical member order (see the crate's design document).

use crate::{
    seal::Seal,
    snapshot::{QueryDocument, QueryKind, Snapshot, SnapshotDirection, direction::sealed::OutputMembers, query_document::writer::query_object},
    subscription::{DocumentType, direction::sealed::IdMember, writer::push},
};
use jstrict::{Object, Value as Json};

/// A list of queries, when the member is present, empty or not.
fn queries<K: QueryKind>(queries: Option<&[QueryDocument<K>]>) -> Option<Json> {
    queries.map(|queries| Json::Array(queries.iter().map(|query| Json::Object(query_object(query.members()))).collect()))
}

/// Writes a snapshot without its `@context`: `id`, `type`, `snapshotQueries`, `snapshotTemporalQueries`, the
/// settings, then the members only a broker writes.
pub(crate) fn snapshot_object<D: SnapshotDirection>(snapshot: &Snapshot<D>) -> Object {
    let mut object = Object::new();
    push(&mut object, "id", snapshot.id.get(Seal::new()).map(|id| Json::from(id.as_str())));
    object.push("type".into(), Json::from(<&str>::from(DocumentType::Snapshot)));
    push(&mut object, "snapshotQueries", queries(snapshot.queries.as_deref()));
    push(&mut object, "snapshotTemporalQueries", queries(snapshot.temporal_queries.as_deref()));
    snapshot.settings.write(&mut object);
    snapshot.output.write(Seal::new(), &mut object);
    object
}
