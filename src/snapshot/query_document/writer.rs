//! Writing the `Query` data type (NGSI-LD Table 5.2.23-1) in canonical member order: `type`, then the members in
//! the order of the table.
//!
//! One writer serves every `Query` the crate writes: a [`QueryDocument`] a snapshot holds, and the body of the POST
//! binding of Query Entities, Query Temporal Evolution and the entity map creations (clauses 6.23.3.1, 6.24.3.1,
//! 6.34.3.2 and 6.35.3.2). Each fills a borrowed [`QueryMembers`] with what it holds, so a body sent and a document
//! rendered are the same JSON for the same members.

use crate::{
    model::{AttributeName, DatasetId, Duration, EntityId},
    query::{
        EntityMapReturn,
        EntitySplitting,
        Join,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        ordering::EntityOrdering,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
        temporal_query::TemporalQuery,
    },
    snapshot::{AggregationParams, QueryDocument, QueryKind, query_document::kind::sealed::KindMembers},
    subscription::{
        DocumentType,
        attribute_list::array,
        writer::{dataset_id_array, geo_query, name_string, printed, projection, push, selectors},
    },
};
use jstrict::{Object, Value as Json};
use std::{fmt::Display, num::NonZeroU32};

/// The members of a `Query` (Table 5.2.23-1), borrowed from whatever holds them; an absent or empty member is not
/// written.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct QueryMembers<'a> {
    /// `entities`, the entity selectors already written (clause 5.2.33).
    pub(crate) entities: Option<Json>,
    /// `q`.
    pub(crate) q: Option<&'a Query>,
    /// `geoQ`.
    pub(crate) geo_q: Option<&'a GeoQuery>,
    /// `scopeQ`.
    pub(crate) scope_q: Option<&'a ScopeQuery>,
    /// `temporalQ`.
    pub(crate) temporal_q: Option<&'a TemporalQuery>,
    /// `attrs`, already written: attribute names, or the names of a temporal query, which may include `scope`.
    pub(crate) attrs: Option<Json>,
    /// `pick`.
    pub(crate) pick: Option<&'a Projection>,
    /// `omit`.
    pub(crate) omit: Option<&'a Projection>,
    /// `aggrParams`.
    pub(crate) aggregation: Option<&'a AggregationParams>,
    /// `csf`.
    pub(crate) csf: Option<&'a Query>,
    /// `containedBy`.
    pub(crate) contained_by: &'a [EntityId],
    /// `datasetId`.
    pub(crate) dataset_ids: Option<&'a [DatasetId]>,
    /// `expandValues`.
    pub(crate) expand_values: &'a [AttributeName],
    /// `entityMap`.
    pub(crate) entity_map: EntityMapReturn,
    /// `entityMapLifetime`.
    pub(crate) entity_map_lifetime: Option<Duration>,
    /// `jsonKeys`.
    pub(crate) json_keys: &'a [AttributeName],
    /// `join`.
    pub(crate) join: Option<Join>,
    /// `joinLevel`.
    pub(crate) join_level: Option<NonZeroU32>,
    /// `lang`.
    pub(crate) lang: Option<&'a LanguageFilter>,
    /// `ordering`.
    pub(crate) ordering: Option<&'a EntityOrdering>,
    /// `splitEntities`.
    pub(crate) split_entities: EntitySplitting,
}

/// `attrs`: an array of names, when there is one.
pub(crate) fn attrs<T: Display>(names: &[T]) -> Option<Json> {
    (!names.is_empty()).then(|| array(names))
}

/// `containedBy`: an array of entity ids, when there is one.
fn entity_ids(ids: &[EntityId]) -> Option<Json> {
    (!ids.is_empty()).then(|| Json::Array(ids.iter().map(|id| Json::from(id.as_str())).collect()))
}

/// Writes a `Query` object: `type`, then each member present, in the order of Table 5.2.23-1, in the JSON type the
/// table gives it.
pub(crate) fn query_object(members: QueryMembers<'_>) -> Object {
    let mut object = Object::new();
    object.push("type".into(), Json::from(<&str>::from(DocumentType::Query)));
    push(&mut object, "entities", members.entities);
    push(&mut object, "q", members.q.map(printed));
    push(&mut object, "geoQ", members.geo_q.map(geo_query));
    push(&mut object, "scopeQ", members.scope_q.map(printed));
    push(&mut object, "temporalQ", members.temporal_q.map(TemporalQuery::to_json));
    push(&mut object, "attrs", members.attrs);
    push(&mut object, "pick", members.pick.map(projection));
    push(&mut object, "omit", members.omit.map(projection));
    push(&mut object, "aggrParams", members.aggregation.map(AggregationParams::to_json));
    push(&mut object, "csf", members.csf.map(printed));
    push(&mut object, "containedBy", entity_ids(members.contained_by));
    push(&mut object, "datasetId", members.dataset_ids.map(dataset_id_array));
    push(&mut object, "expandValues", name_string(members.expand_values));
    push(&mut object, "entityMap", members.entity_map.flag().map(Json::from));
    push(&mut object, "entityMapLifetime", members.entity_map_lifetime.as_ref().map(printed));
    push(&mut object, "jsonKeys", name_string(members.json_keys));
    push(&mut object, "join", members.join.map(|join| Json::from(<&str>::from(join))));
    push(&mut object, "joinLevel", members.join_level.map(|level| Json::from(level.get())));
    push(&mut object, "lang", members.lang.map(printed));
    push(&mut object, "ordering", members.ordering.map(EntityOrdering::to_json));
    push(&mut object, "splitEntities", members.split_entities.flag().map(Json::from));
    object
}

impl<K: QueryKind> QueryDocument<K> {
    /// The members of the document, as the writer takes them.
    pub(crate) fn members(&self) -> QueryMembers<'_> {
        QueryMembers {
            entities: selectors(&self.entities),
            q: self.q.as_ref(),
            geo_q: self.geo_q.as_ref(),
            scope_q: self.scope_q.as_ref(),
            temporal_q: self.kind_members.temporal_q(),
            attrs: attrs(&self.attrs),
            pick: self.pick.as_ref(),
            omit: self.omit.as_ref(),
            aggregation: self.kind_members.aggregation(),
            csf: self.csf.as_ref(),
            contained_by: &self.contained_by,
            dataset_ids: self.dataset_ids.as_deref(),
            expand_values: &self.expand_values,
            entity_map: self.entity_map,
            entity_map_lifetime: self.entity_map_lifetime,
            json_keys: &self.json_keys,
            join: self.join,
            join_level: self.join_level,
            lang: self.lang.as_ref(),
            ordering: self.ordering.as_ref(),
            split_entities: self.split_entities,
        }
    }
}
