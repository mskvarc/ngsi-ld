//! Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1), once per representation and once per
//! completeness.

use crate::{
    client::{
        Answering,
        ClientError,
        Temporal,
        Transport,
        arguments::{SelectionBuilders, build, built, selectors_built},
    },
    http::{
        Page,
        operation::{
            Aggregated,
            EntityMapRequest,
            QueryMethod,
            QueryTemporalEvolution,
            RepresentationSpelling,
            ResultsCount,
            SimplifiedTemporal,
            SystemAttributes,
        },
    },
    model::{
        AttributeName,
        Complete,
        DatasetId,
        EntityId,
        Lossless,
        Output,
        Projected,
        Tenant,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalAttributeName, TemporalEntity},
    },
    query::{
        EntitySelectorDraft,
        EntitySplitting,
        IdPattern,
        Locality,
        builders::SelectorTypeBuilder,
        geoquery::builders::GeoQueryBuilder,
        language_filter::builders::LanguageFilterBuilder,
        ordering::{EntityOrdering, builders::EntityOrderingBuilder},
        projection::builders::ProjectionBuilder,
        q::builders::QueryBuilder,
        scope_query::builders::ScopeQueryBuilder,
        temporal_query::{TemporalQuery, builders},
    },
};
use bon::bon;

/// Generates the Query Temporal Evolution method for one representation and one completeness.
///
/// `$name => $builder` names the method's builder where bon's default, `Temporal` and the method name, would
/// collide with a builder of another namespace.
macro_rules! query_method {
    (
        $(#[doc = $doc:literal])*
        $name:ident $(=> $builder:ident)?, $item:ty,
        $format:ident => $format_value:expr, [$($format_parameter:tt)*],
        completeness => $completeness:expr
    ) => {
        #[bon]
        impl<T: Transport, M: Answering> Temporal<'_, T, M> {
            $(#[doc = $doc])*
            #[builder($(builder_type = $builder,)? derive(Debug), finish_fn(name = send, doc {
                /// Sends the request and reads the response.
                ///
                /// # Errors
                ///
                /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, the
                /// broker answers with an error, or the transport fails.
            }))]
            pub async fn $name(
                &self,
                #[builder(start_fn, into)] temporal_q: builders::TemporalQueryBuilder,
                #[builder(into)] entity_type: Option<SelectorTypeBuilder>,
                #[builder(default)] ids: Vec<EntityId>,
                #[builder(into)] id_pattern: Option<IdPattern>,
                #[builder(default, with = |selectors: impl IntoIterator<Item = impl Into<EntitySelectorDraft>>| selectors.into_iter().map(Into::into).collect())]
                selectors: Vec<EntitySelectorDraft>,
                #[builder(into)] q: Option<QueryBuilder>,
                #[builder(into)] geo_q: Option<GeoQueryBuilder>,
                #[builder(into)] scope_q: Option<ScopeQueryBuilder>,
                #[builder(into)] csf: Option<QueryBuilder>,
                #[builder(default)] attrs: Vec<TemporalAttributeName>,
                #[builder(into)] pick: Option<ProjectionBuilder>,
                #[builder(into)] omit: Option<ProjectionBuilder>,
                #[builder(default)] expand_values: Vec<AttributeName>,
                #[builder(default)] json_keys: Vec<AttributeName>,
                #[builder(default)] dataset_ids: Vec<DatasetId>,
                #[builder(into)] lang: Option<LanguageFilterBuilder>,
                #[builder(into)] ordering: Option<EntityOrderingBuilder>,
                #[builder(default)] split_entities: EntitySplitting,
                entity_map: Option<&EntityMapRequest>,
                #[builder(default)] system_attributes: SystemAttributes,
                limit: Option<u32>,
                offset: Option<u32>,
                #[builder(default)] count: ResultsCount,
                #[builder(default)] locality: Locality,
                #[builder(default)] method: QueryMethod,
                $($format_parameter)*
                #[builder(default)] spelling: RepresentationSpelling,
                tenant: Option<&Tenant>,
            ) -> Result<M::Answer<Page<$item>>, ClientError<T::Error>> {
                let temporal_q: TemporalQuery = build(temporal_q)?;
                let selection = SelectionBuilders {
                    entity_type,
                    q,
                    geo_q,
                    scope_q,
                    csf,
                    pick,
                    omit,
                    lang,
                }
                .build()?;
                let ordering: Option<EntityOrdering> = built(ordering)?;
                let selectors = selectors_built(selectors)?;
                let $format = $format_value;
                let query = QueryTemporalEvolution {
                    entity_type: selection.entity_type.as_ref(),
                    ids: &ids,
                    id_pattern: id_pattern.as_ref(),
                    selectors: &selectors,
                    attrs: &attrs,
                    q: selection.q.as_ref(),
                    geo_q: selection.geo_q.as_ref(),
                    scope_q: selection.scope_q.as_ref(),
                    csf: selection.csf.as_ref(),
                    pick: selection.pick.as_ref(),
                    omit: selection.omit.as_ref(),
                    expand_values: &expand_values,
                    json_keys: &json_keys,
                    dataset_ids: &dataset_ids,
                    lang: selection.lang.as_ref(),
                    temporal_q: &temporal_q,
                    ordering: ordering.as_ref(),
                    split_entities,
                    entity_map,
                    system_attributes,
                    limit,
                    offset,
                    count,
                    locality,
                    method,
                    format: $format,
                    spelling,
                    completeness: $completeness,
                };
                self.route.send_selection(&query, tenant).await
            }
        }
    };
}

query_method!(
    /// Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1), in normalized or concise
    /// representation; the first page, which [`Client::pages`](crate::client::Client::pages) walks on.
    ///
    /// At least an entity type, attributes, a `q` or a geoquery is needed unless the query is local, `limit` 0 needs
    /// `count` (clause 6.3.13), the ordering may only order by `id` (Table 6.18.3.2-1), and a `pick` or `omit` must
    /// keep `id` and `type` ([`query_projected`] reads an answer without them); `send()` refuses the request otherwise.
    ///
    /// [`query_projected`]: Temporal::query_projected
    query => QueryTemporalBuilder, TemporalEntity<Output>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Complete
);

query_method!(
    /// Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1), in simplified temporal
    /// representation (`format=temporalValues`, clause 4.5.9).
    ///
    /// The same checks as [`query`](Temporal::query) apply.
    query_values, SimplifiedTemporalEntity,
    format => SimplifiedTemporal, [],
    completeness => Complete
);

query_method!(
    /// Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1), in aggregated temporal
    /// representation (`format=aggregatedValues`, clause 4.5.19).
    ///
    /// The same checks as [`query`](Temporal::query) apply, and the temporal query must name the aggregation methods,
    /// or `send()` reports
    /// [`ArgumentError::AggregationWithoutMethods`](crate::http::operation::ArgumentError::AggregationWithoutMethods).
    query_aggregated, AggregatedTemporalEntity,
    format => Aggregated, [],
    completeness => Complete
);

query_method!(
    /// Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1) with a `pick` or `omit` that may
    /// remove `id` or `type` (clause 4.21), in normalized or concise representation.
    query_projected, TemporalEntity<Output, Projected>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Projected
);

query_method!(
    /// Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1) with a `pick` or `omit` that may
    /// remove `id` or `type` (clause 4.21), in simplified temporal representation.
    query_projected_values, SimplifiedTemporalEntity<Projected>,
    format => SimplifiedTemporal, [],
    completeness => Projected
);

query_method!(
    /// Query Temporal Evolution of Entities (clauses 6.18.3.2 and 6.24.3.1) with a `pick` or `omit` that may
    /// remove `id` or `type` (clause 4.21), in aggregated temporal representation.
    query_projected_aggregated, AggregatedTemporalEntity<Projected>,
    format => Aggregated, [],
    completeness => Projected
);
