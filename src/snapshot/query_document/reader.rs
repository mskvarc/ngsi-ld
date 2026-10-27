//! Reading the `Query` data type (NGSI-LD Table 5.2.23-1) of either kind, and the rules its members obey together.

use crate::{
    model::{
        DocumentError,
        EntityId,
        MemberError,
        keyword_alias::unalias_members,
        reader::{boolean, duration, identifier, object, once, required, token, unexpected, within},
    },
    query::{EntityMapReturn, EntitySplitting, Join, ordering::EntityOrdering},
    seal::Seal,
    snapshot::{Problem, QueryDocument, QueryKind, QueryMember, query_document::kind::sealed::KindMembers},
    subscription::{
        DocumentType,
        ReadErrorKind,
        reader::{
            Emptiness,
            comma_separated_names,
            dataset_id_list,
            error,
            expect_type,
            geo_query,
            language,
            list,
            names,
            notification_params::{level, projection},
            selectors,
        },
    },
};
use jstrict::{Value as Json, object::Entry};
use std::error::Error;

/// Reads `containedBy`: a non-empty array of entity ids (Table 5.2.23-1: "Empty array (0 length) is not allowed").
///
/// Table 5.2.23-1 says `joinLevel` is "only applicable if join parameter is flat or inline" and `containedBy` "only
/// applicable if joinLevel is present": both describe what a broker does with the member rather than forbid it, so
/// neither is refused without the other.
fn entity_ids<P: Error>(json: Json) -> Result<Vec<EntityId>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Refused)?
        .into_iter()
        .map(|item| identifier(item, "containedBy"))
        .collect()
}

/// Reads a query of kind `K` (Table 5.2.23-1); a member only the other kind has is unexpected, and a temporal query
/// without `temporalQ` misses it.
pub(crate) fn read_query<K: QueryKind>(json: Json) -> Result<QueryDocument<K>, DocumentError<ReadErrorKind<Problem>>> {
    let mut typed = None;
    let mut entities = None;
    let mut q = None;
    let mut geo_q = None;
    let mut scope_q = None;
    let mut kind_members = <K::Members as KindMembers>::Pending::default();
    let mut attrs = None;
    let mut pick = None;
    let mut omit = None;
    let mut csf = None;
    let mut contained_by = None;
    let mut dataset_ids = None;
    let mut expand_values = None;
    let mut entity_map = None;
    let mut entity_map_lifetime = None;
    let mut json_keys = None;
    let mut join = None;
    let mut join_level = None;
    let mut lang = None;
    let mut ordering = None;
    let mut split_entities = None;
    for Entry { key, value } in unalias_members(object(json)?)? {
        let Ok(member) = key.as_str().parse::<QueryMember>() else {
            match key.as_str() {
                "type" => once(&mut typed, "type", within("type", expect_type(value, DocumentType::Query))?)?,
                other => return Err(unexpected(other)),
            }
            continue;
        };
        let key = member.into();
        match member {
            QueryMember::Entities => once(&mut entities, key, within(key, selectors(value))?)?,
            QueryMember::Q => once(&mut q, key, within(key, language(value, key))?)?,
            QueryMember::GeoQ => once(&mut geo_q, key, within(key, geo_query(value))?)?,
            QueryMember::ScopeQ => once(&mut scope_q, key, within(key, language(value, key))?)?,
            QueryMember::Attrs => once(&mut attrs, key, within(key, names::<K::AttrsName, _>(value, key, Emptiness::Refused))?)?,
            QueryMember::Pick => once(&mut pick, key, within(key, projection(value, key))?)?,
            QueryMember::Omit => once(&mut omit, key, within(key, projection(value, key))?)?,
            QueryMember::Csf => once(&mut csf, key, within(key, language(value, key))?)?,
            QueryMember::ContainedBy => once(&mut contained_by, key, within(key, entity_ids(value))?)?,
            QueryMember::DatasetId => once(&mut dataset_ids, key, within(key, dataset_id_list(value, key))?)?,
            QueryMember::ExpandValues => once(&mut expand_values, key, within(key, comma_separated_names(value, key))?)?,
            QueryMember::EntityMap => once(&mut entity_map, key, within(key, boolean(&value, key)).map(EntityMapReturn::from_flag)?)?,
            QueryMember::EntityMapLifetime => once(&mut entity_map_lifetime, key, within(key, duration(value, key))?)?,
            QueryMember::JsonKeys => once(&mut json_keys, key, within(key, comma_separated_names(value, key))?)?,
            QueryMember::Join => once(&mut join, key, within(key, token::<Join, _>(value, key))?)?,
            QueryMember::JoinLevel => once(&mut join_level, key, within(key, level(value))?)?,
            QueryMember::Lang => once(&mut lang, key, within(key, language(value, key))?)?,
            QueryMember::Ordering => {
                let read = EntityOrdering::from_json(value)
                    .map_err(|error| error.map_kind(|source| ReadErrorKind::Ordering { source: Box::new(source) }).within(key))?;
                once(&mut ordering, key, read)?;
            }
            QueryMember::SplitEntities => once(&mut split_entities, key, within(key, boolean(&value, key)).map(EntitySplitting::from_flag)?)?,
            QueryMember::TemporalQ | QueryMember::AggrParams => {
                if K::Members::read(Seal::new(), &mut kind_members, member, value)?.is_some() {
                    return Err(unexpected(key));
                }
            }
        }
    }
    required(typed, "type")?;
    let query = QueryDocument {
        entities: entities.unwrap_or_default(),
        q,
        geo_q,
        scope_q,
        kind_members: K::Members::complete(Seal::new(), kind_members).ok_or_else(|| {
            error(
                MemberError::MissingMember {
                    member: QueryMember::TemporalQ.into(),
                }
                .into(),
            )
        })?,
        attrs: attrs.unwrap_or_default(),
        pick,
        omit,
        csf,
        contained_by: contained_by.unwrap_or_default(),
        dataset_ids,
        expand_values: expand_values.unwrap_or_default(),
        entity_map: entity_map.unwrap_or_default(),
        entity_map_lifetime,
        json_keys: json_keys.unwrap_or_default(),
        join,
        join_level,
        lang,
        ordering,
        split_entities: split_entities.unwrap_or_default(),
    };
    Ok(query)
}

/// Reads a list of queries of kind `K`, placing an error under the index of the query at fault. The list may be
/// empty: Table 5.2.41-1 sets no minimum length for `snapshotQueries` or `snapshotTemporalQueries`.
pub(crate) fn read_queries<K: QueryKind>(json: Json) -> Result<Vec<QueryDocument<K>>, DocumentError<ReadErrorKind<Problem>>> {
    list(json, Emptiness::Allowed)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| read_query(item).map_err(|error| error.at_index(index)))
        .collect()
}
