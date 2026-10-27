//! Reading snapshot documents (NGSI-LD Tables 5.2.41-1 and 5.2.41-2), with the members only a broker writes when
//! the direction has them.

use crate::{
    model::{
        BuildError,
        DocumentError,
        MemberChange,
        keyword::CONTEXT,
        member_change::read_change,
        reader::{context, duplicate, identifier, object, once, required, unexpected, within},
    },
    seal::Seal,
    snapshot::{
        EntityQuery,
        Problem,
        Snapshot,
        SnapshotClone,
        SnapshotDirection,
        SnapshotFragment,
        SnapshotId,
        SnapshotMember,
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

/// Reads the body of Clone Snapshot: the `id` and settings of a snapshot, without its queries, which the clone
/// copies from the snapshot it clones (clause 5.16.2.3). The `type` is required, as Table 5.2.41-1 requires it of a
/// Snapshot; the `@context` belongs to the binding (clause 6.3.5), so it is checked and not kept.
pub(crate) fn read_snapshot_clone(json: Json) -> Result<SnapshotClone, DocumentError<ReadErrorKind<Problem>>> {
    let mut id = None;
    let mut typed = None;
    let mut settings = Settings::default();
    let mut body_context = None;
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        match key {
            "id" => once(&mut id, key, within(key, identifier(value, "id"))?)?,
            "type" => once(&mut typed, key, within(key, expect_type(value, DocumentType::Snapshot))?)?,
            CONTEXT => once(&mut body_context, key, within(key, context(value))?)?,
            other => {
                if settings.read(other, value)?.is_some() {
                    return Err(unexpected(other));
                }
            }
        }
    }
    required(typed, "type")?;
    Ok(SnapshotClone { id, settings })
}

/// Reads the fragment of Update Snapshot Status: the settings it sets, and the ones it deletes with the NGSI-LD Null
/// (clause 5.5.8). Clause 5.4 makes `id` and `type` optional where the operation names them, so both may be left
/// out; an `id` is checked, as is the `@context`, which belongs to the binding (clause 6.3.5), and neither is kept.
pub(crate) fn read_snapshot_fragment(json: Json) -> Result<SnapshotFragment, DocumentError<ReadErrorKind<Problem>>> {
    let mut id = None;
    let mut typed = None;
    let mut settings = Settings::default();
    let mut deletions = Vec::new();
    let mut set = Vec::new();
    let mut body_context = None;
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        match key {
            "id" => once(&mut id, key, within(key, identifier::<SnapshotId, _>(value, "id"))?)?,
            "type" => once(&mut typed, key, within(key, expect_type(value, DocumentType::Snapshot))?)?,
            CONTEXT => once(&mut body_context, key, within(key, context(value))?)?,
            other => {
                let Ok(member) = other.parse::<SnapshotMember>() else {
                    return Err(unexpected(other));
                };
                if set.contains(&member) || deletions.contains(&member) {
                    return Err(duplicate(other));
                }
                // The member is one of the settings, so reading it leaves no value unread.
                match within(other, read_change(value, |value| settings.read(other, value).map(|_| ())))? {
                    MemberChange::Delete => deletions.push(member),
                    MemberChange::Set(()) => set.push(member),
                }
            }
        }
    }
    Ok(SnapshotFragment { settings, deletions })
}

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
