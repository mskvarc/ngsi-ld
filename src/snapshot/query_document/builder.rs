//! The builder of a query a snapshot holds (NGSI-LD clause 5.2.23): one builder for both kinds, with the members
//! only a temporal query has set through setters that exist on its builder alone.

use crate::{
    model::{
        AttributeNameDraft,
        BuildError,
        DatasetId,
        Duration,
        EntityId,
        MemberProblem,
        NotPositive,
        Number,
        draft::resolve_required_drafts,
        name::draft::{resolve_names, resolve_required_names},
    },
    query::{
        EntityMapReturn,
        EntitySelectorDraft,
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
    snapshot::{
        AggregationParams,
        EntityQuery,
        Problem,
        QueryDocument,
        QueryKind,
        QueryMember,
        SnapshotField,
        TemporalEvolutionQuery,
        query_document::kind::sealed::KindMembers,
    },
};
use bon::bon;
use std::num::NonZeroU32;

#[bon]
impl<K: QueryKind> QueryDocument<K> {
    /// Starts a builder of kind `K`; [`QueryDocument::builder`] and [`QueryDocument::temporal_builder`] name the
    /// kind.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid attribute name, an empty `entities` or `attrs`, a
    /// `joinLevel` of 0, and every rule of Table 5.2.23-1 the members break together.
    #[builder(start_fn(name = kind_builder, vis = ""))]
    pub fn new(
        #[builder(field)] kind_members: <K::Members as KindMembers>::Pending,
        #[builder(with = |selectors: impl IntoIterator<Item = impl Into<EntitySelectorDraft>>| selectors.into_iter().map(Into::into).collect())]
        entities: Option<Vec<EntitySelectorDraft>>,
        q: Option<Query>,
        geo_q: Option<GeoQuery>,
        scope_q: Option<ScopeQuery>,
        #[builder(with = |names: impl IntoIterator<Item = impl Into<K::AttrsDraft>>| names.into_iter().map(Into::into).collect())] attrs: Option<
            Vec<K::AttrsDraft>,
        >,
        pick: Option<Projection>,
        omit: Option<Projection>,
        csf: Option<Query>,
        #[builder(default, with = |ids: impl IntoIterator<Item = EntityId>| ids.into_iter().collect())] contained_by: Vec<EntityId>,
        #[builder(with = |ids: impl IntoIterator<Item = DatasetId>| ids.into_iter().collect())] dataset_ids: Option<Vec<DatasetId>>,
        #[builder(default, with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())]
        expand_values: Vec<AttributeNameDraft>,
        #[builder(default)] entity_map: EntityMapReturn,
        entity_map_lifetime: Option<Duration>,
        #[builder(default, with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())] json_keys: Vec<
            AttributeNameDraft,
        >,
        join: Option<Join>,
        join_level: Option<u32>,
        lang: Option<LanguageFilter>,
        ordering: Option<EntityOrdering>,
        #[builder(default)] split_entities: EntitySplitting,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let field = SnapshotField::Query;
        let member = field(QueryMember::Entities);
        let entities = resolve_required_drafts(member, entities, &mut problems, |index, source| {
            Problem::from(MemberProblem::InvalidSelectedType { member, index, source })
        });
        let join_level = join_level.and_then(|level| {
            let positive = NonZeroU32::new(level);
            if positive.is_none() {
                problems.push(Problem::from(MemberProblem::NotPositive {
                    member: field(QueryMember::JoinLevel),
                    source: NotPositive { number: Number::from(level) },
                }));
            }
            positive
        });
        let attrs = resolve_required_names(field(QueryMember::Attrs), attrs, &mut problems);
        let expand_values = resolve_names(field(QueryMember::ExpandValues), expand_values, &mut problems);
        let json_keys = resolve_names(field(QueryMember::JsonKeys), json_keys, &mut problems);
        let kind_members = K::Members::complete(kind_members);
        if kind_members.is_none() {
            problems.push(Problem::from(MemberProblem::Missing {
                member: field(QueryMember::TemporalQ),
            }));
        }
        let query = kind_members.map(|kind_members| Self {
            entities,
            q,
            geo_q,
            scope_q,
            kind_members,
            attrs,
            pick,
            omit,
            csf,
            contained_by,
            dataset_ids,
            expand_values,
            entity_map,
            entity_map_lifetime,
            json_keys,
            join,
            join_level,
            lang,
            ordering,
            split_entities,
        });
        BuildError::finish_partial(problems, query)
    }
}

impl QueryDocument<EntityQuery> {
    /// Starts the builder of a query of entities (clause 5.7.2), the kind of `snapshotQueries`. Every member is
    /// optional, and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid attribute name, an empty `entities` or `attrs`, a
    /// `joinLevel` of 0, and every rule of Table 5.2.23-1 the members break together.
    pub fn builder() -> QueryDocumentBuilder<EntityQuery> {
        Self::kind_builder()
    }
}

impl QueryDocument<TemporalEvolutionQuery> {
    /// Starts the builder of a query of the temporal evolution of entities (clause 5.7.4), the kind of
    /// `snapshotTemporalQueries`, with its mandatory `temporalQ`.
    ///
    /// ```
    /// use ngsi_ld::{query::temporal_query::TemporalQuery, snapshot::QueryDocument};
    ///
    /// let history =
    ///     QueryDocument::temporal_builder(TemporalQuery::after("2024-05-01T10:00:00Z".parse()?))
    ///         .entities(["Vehicle"])
    ///         .build()?;
    /// assert_eq!(
    ///     history.temporal_q().time_at().to_string(),
    ///     "2024-05-01T10:00:00Z"
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] as [`QueryDocument::builder`] does.
    pub fn temporal_builder(temporal_q: TemporalQuery) -> QueryDocumentBuilder<TemporalEvolutionQuery> {
        let mut builder = Self::kind_builder();
        builder.kind_members.temporal_q = Some(temporal_q);
        builder
    }
}

impl<S: query_document_builder::State> QueryDocumentBuilder<TemporalEvolutionQuery, S> {
    /// Sets the aggregation of the temporal query (`aggrParams`, clause 5.2.44), which only a query of the temporal
    /// evolution of entities has.
    pub fn aggregation(mut self, params: AggregationParams) -> Self {
        self.kind_members.aggr_params = Some(params);
        self
    }

    /// Sets `aggrParams` when there is one.
    pub fn maybe_aggregation(mut self, params: Option<AggregationParams>) -> Self {
        self.kind_members.aggr_params = params;
        self
    }
}
