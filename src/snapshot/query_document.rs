//! The `Query` data type of NGSI-LD clause 5.2.23 as a document of its own, as a snapshot holds its queries (Table
//! 5.2.41-1).

use crate::{
    model::{AttributeName, DatasetId, DocumentError, Duration, EntityId, RawJson, reader::parse_json},
    query::{
        EntityMapReturn,
        EntitySelector,
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
    snapshot::{AggregationParams, EntityQuery, Problem, QueryKind, TemporalEvolutionQuery},
    subscription::ReadErrorKind,
};
use jstrict::Value as Json;
use std::num::NonZeroU32;

pub(super) mod builder;
pub(super) mod kind;
pub(super) mod member;
pub(crate) mod reader;
pub(crate) mod writer;

/// The `Query` data type (NGSI-LD clause 5.2.23), of the kind `K`: a query of entities ([`EntityQuery`], the
/// default) or of the temporal evolution of entities ([`TemporalEvolutionQuery`]), as a snapshot holds them in
/// `snapshotQueries` and `snapshotTemporalQueries` (Table 5.2.41-1).
///
/// It is named [`QueryDocument`] because [`Query`](crate::query::q::Query) is the query language of clause 4.9. The `POST` bindings of
/// Query Entities and Query Temporal Evolution write the same members from the fields of their operations; a
/// snapshot keeps its queries, so they are documents a client builds and a broker gives back.
///
/// Every member of Table 5.2.23-1 is typed with the query languages and written in the JSON type the table gives
/// it. `temporalQ` and `aggrParams` exist on temporal queries only, where `temporalQ` is mandatory.
///
/// ```
/// use ngsi_ld::{query::q, snapshot::QueryDocument};
///
/// let fast = QueryDocument::builder()
///     .entities(["Vehicle"])
///     .q(q::attr("speed").gt(50).build()?)
///     .build()?;
/// assert_eq!(
///     fast.render().to_string(),
///     r#"{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDocument<K: QueryKind = EntityQuery> {
    /// `entities`, never empty when present.
    pub(crate) entities: Vec<EntitySelector>,
    /// `q`.
    pub(crate) q: Option<Query>,
    /// `geoQ`.
    pub(crate) geo_q: Option<GeoQuery>,
    /// `scopeQ`.
    pub(crate) scope_q: Option<ScopeQuery>,
    /// The members of the kind: `temporalQ` and `aggrParams` of a temporal query.
    pub(crate) kind_members: K::Members,
    /// `attrs`, deprecated (Table 5.2.23-1), never empty when present; in a temporal query it may name `scope`.
    pub(crate) attrs: Vec<K::AttrsName>,
    /// `pick`.
    pub(crate) pick: Option<Projection>,
    /// `omit`.
    pub(crate) omit: Option<Projection>,
    /// `csf`.
    pub(crate) csf: Option<Query>,
    /// `containedBy`.
    pub(crate) contained_by: Vec<EntityId>,
    /// `datasetId`.
    pub(crate) dataset_ids: Option<Vec<DatasetId>>,
    /// `expandValues`.
    pub(crate) expand_values: Vec<AttributeName>,
    /// `entityMap`.
    pub(crate) entity_map: EntityMapReturn,
    /// `entityMapLifetime`.
    pub(crate) entity_map_lifetime: Option<Duration>,
    /// `jsonKeys`.
    pub(crate) json_keys: Vec<AttributeName>,
    /// `join`.
    pub(crate) join: Option<Join>,
    /// `joinLevel`.
    pub(crate) join_level: Option<NonZeroU32>,
    /// `lang`.
    pub(crate) lang: Option<LanguageFilter>,
    /// `ordering`.
    pub(crate) ordering: Option<EntityOrdering>,
    /// `splitEntities`.
    pub(crate) split_entities: EntitySplitting,
}

impl<K: QueryKind> QueryDocument<K> {
    /// Reads the text of a query of kind `K` (Table 5.2.23-1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault: a member the table does not define (or that only the other
    /// kind has), a member written twice or out of its type, a missing `type` (or `temporalQ` of a temporal
    /// query), or a rule of the table the members break together.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        reader::read_query(parse_json(text)?)
    }

    /// Writes the query in canonical form: `type`, then the members in the order of Table 5.2.23-1.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(writer::query_object(self.members())))
    }

    /// Returns the entities to match (`entities`).
    #[must_use]
    pub fn entities(&self) -> &[EntitySelector] {
        &self.entities
    }

    /// Returns the query the entities match (`q`, clause 4.9).
    #[must_use]
    pub const fn q(&self) -> Option<&Query> {
        self.q.as_ref()
    }

    /// Returns the geoquery the entities match (`geoQ`, clause 5.2.13).
    #[must_use]
    pub const fn geo_q(&self) -> Option<&GeoQuery> {
        self.geo_q.as_ref()
    }

    /// Returns the scopes the entities are within (`scopeQ`, clause 4.19).
    #[must_use]
    pub const fn scope_q(&self) -> Option<&ScopeQuery> {
        self.scope_q.as_ref()
    }

    /// Returns the deprecated `attrs`, a synonym of a `pick` and a `q` (Table 5.2.23-1).
    #[must_use]
    pub fn attrs(&self) -> &[K::AttrsName] {
        &self.attrs
    }

    /// Returns the members to keep (`pick`, clause 4.21).
    #[must_use]
    pub const fn pick(&self) -> Option<&Projection> {
        self.pick.as_ref()
    }

    /// Returns the members to remove (`omit`, clause 4.21).
    #[must_use]
    pub const fn omit(&self) -> Option<&Projection> {
        self.omit.as_ref()
    }

    /// Returns the context source filter (`csf`, clause 4.9).
    #[must_use]
    pub const fn csf(&self) -> Option<&Query> {
        self.csf.as_ref()
    }

    /// Returns the linked entities already encountered (`containedBy`, clause 4.5.23).
    #[must_use]
    pub fn contained_by(&self) -> &[EntityId] {
        &self.contained_by
    }

    /// Returns the attribute instances to keep, by dataset id (`datasetId`, clause 4.5.5).
    #[must_use]
    pub fn dataset_ids(&self) -> Option<&[DatasetId]> {
        self.dataset_ids.as_deref()
    }

    /// Returns the attributes whose values are expanded before the query is evaluated (`expandValues`).
    #[must_use]
    pub fn expand_values(&self) -> &[AttributeName] {
        &self.expand_values
    }

    /// Returns whether the location of the entity map is asked for (`entityMap`).
    #[must_use]
    pub const fn entity_map(&self) -> EntityMapReturn {
        self.entity_map
    }

    /// Returns the lifetime asked for the entity map (`entityMapLifetime`).
    #[must_use]
    pub const fn entity_map_lifetime(&self) -> Option<&Duration> {
        self.entity_map_lifetime.as_ref()
    }

    /// Returns the attributes whose values are not interpreted as JSON-LD (`jsonKeys`).
    #[must_use]
    pub fn json_keys(&self) -> &[AttributeName] {
        &self.json_keys
    }

    /// Returns the linked entity retrieval (`join`, clause 4.5.23).
    #[must_use]
    pub const fn join(&self) -> Option<Join> {
        self.join
    }

    /// Returns the depth of linked entity retrieval (`joinLevel`).
    #[must_use]
    pub const fn join_level(&self) -> Option<NonZeroU32> {
        self.join_level
    }

    /// Returns the language filter (`lang`, clause 4.15).
    #[must_use]
    pub const fn lang(&self) -> Option<&LanguageFilter> {
        self.lang.as_ref()
    }

    /// Returns the order of the entities (`ordering`, clause 5.2.43).
    #[must_use]
    pub const fn ordering(&self) -> Option<&EntityOrdering> {
        self.ordering.as_ref()
    }

    /// Returns whether an entity may be split over several context sources (`splitEntities`).
    #[must_use]
    pub const fn split_entities(&self) -> EntitySplitting {
        self.split_entities
    }
}

impl QueryDocument<TemporalEvolutionQuery> {
    /// Returns the temporal query (`temporalQ`, clause 5.2.21).
    #[must_use]
    pub const fn temporal_q(&self) -> &TemporalQuery {
        &self.kind_members.temporal_q
    }

    /// Returns the aggregation (`aggrParams`, clause 5.2.44).
    #[must_use]
    pub const fn aggr_params(&self) -> Option<&AggregationParams> {
        self.kind_members.aggr_params.as_ref()
    }
}

#[cfg(test)]
mod tests;
