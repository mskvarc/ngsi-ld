//! Reading snapshot documents (NGSI-LD Tables 5.2.41-1 and 5.2.41-2), with the members only a broker writes when
//! the direction has them.

use crate::{
    model::{
        BuildError,
        DocumentError,
        keyword::CONTEXT,
        reader::{context, identifier, object, once, required, unexpected, within},
    },
    seal::Seal,
    snapshot::{
        EntityQuery,
        Problem,
        Snapshot,
        SnapshotDirection,
        TemporalEvolutionQuery,
        direction::sealed::{OutputMembers, QueryRequirement},
        query_document::reader::read_queries,
        settings::Settings,
    },
    subscription::{
        DocumentType,
        ReadErrorKind,
        direction::sealed::IdMember,
        reader::{error, expect_type},
    },
};
use jstrict::{Value as Json, object::Entry};

/// Reads a snapshot in direction `D`: a member Table 5.2.41-1 does not define, or one of Table 5.2.41-2 in a
/// request, is unexpected, and a request needs a query of either kind.
pub(crate) fn read_snapshot<D: SnapshotDirection>(json: Json) -> Result<Snapshot<D>, DocumentError<ReadErrorKind<Problem>>> {
    let mut id = None;
    let mut typed = None;
    let mut queries = None;
    let mut temporal_queries = None;
    let mut settings = Settings::default();
    let mut snapshot_context = None;
    let mut output = <D::SnapshotOutput as OutputMembers>::Pending::default();
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        match key {
            "id" => once(&mut id, key, within(key, identifier(value, "id"))?)?,
            "type" => once(&mut typed, key, within(key, expect_type(value, DocumentType::Snapshot))?)?,
            "snapshotQueries" => once(&mut queries, key, within(key, read_queries::<EntityQuery>(value))?)?,
            "snapshotTemporalQueries" => once(&mut temporal_queries, key, within(key, read_queries::<TemporalEvolutionQuery>(value))?)?,
            CONTEXT => once(&mut snapshot_context, key, within(key, context(value))?)?,
            other => {
                let rest = match settings.read(other, value)? {
                    Some(value) => D::SnapshotOutput::read(Seal::new(), &mut output, other, value)?,
                    None => None,
                };
                if rest.is_some() {
                    return Err(unexpected(other));
                }
            }
        }
    }
    required(typed, "type")?;
    let snapshot = Snapshot {
        id: D::Id::from_read(Seal::new(), id)?,
        queries,
        temporal_queries,
        settings,
        output: D::SnapshotOutput::complete(Seal::new(), output)?,
        context: snapshot_context,
    };
    BuildError::finish(creation_problems(&snapshot), ()).map_err(|source| error(ReadErrorKind::Invalid { source }))?;
    Ok(snapshot)
}

/// The rule of a creation (Table 5.2.41-1): "at least one of `snapshotQueries` or `snapshotTemporalQueries` shall be
/// present", so a request holds either member, possibly as an empty list.
pub(crate) fn creation_problems<D: SnapshotDirection>(snapshot: &Snapshot<D>) -> Vec<Problem> {
    match D::SnapshotOutput::QUERIES {
        QueryRequirement::AtLeastOne if snapshot.queries.is_none() && snapshot.temporal_queries.is_none() => vec![Problem::NoSnapshotQueries],
        QueryRequirement::AtLeastOne | QueryRequirement::Any => Vec::new(),
    }
}
