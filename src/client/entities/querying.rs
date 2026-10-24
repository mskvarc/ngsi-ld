//! Query Entities (clauses 6.4.3.2 and 6.23.3.1), one method per representation family and per completeness.

use crate::{
    client::{
        Answering,
        ClientError,
        Entities,
        Transport,
        arguments::{SelectionBuilders, built, selectors_built},
    },
    http::{
        Page,
        operation::{EntityMapRequest, GeoJson, QueryEntities, QueryMethod, RepresentationSpelling, ResultsCount, Simplified, SystemAttributes},
    },
    model::{AttributeName, Complete, DatasetId, Entity, EntityId, Feature, Lossless, Output, Projected, SimplifiedEntity, Tenant},
    query::{
        EntitySelectorDraft,
        EntitySplitting,
        IdPattern,
        Join,
        Locality,
        builders::SelectorTypeBuilder,
        geoquery::builders::GeoQueryBuilder,
        language_filter::builders::LanguageFilterBuilder,
        ordering::{EntityOrdering, builders::EntityOrderingBuilder},
        projection::builders::ProjectionBuilder,
        q::builders::QueryBuilder,
        scope_query::builders::ScopeQueryBuilder,
    },
};
use bon::bon;
use std::num::NonZeroU32;

/// Generates the Query Entities method for one representation and one completeness.
macro_rules! query_method {
    (
        $(#[doc = $doc:literal])*
        $name:ident, $item:ty,
        $format:ident => $format_value:expr, [$($format_parameter:tt)*],
        completeness => $completeness:expr
    ) => {
        #[bon]
        impl<T: Transport, M: Answering> Entities<'_, T, M> {
            $(#[doc = $doc])*
            #[builder(derive(Debug), finish_fn(name = send, doc {
                /// Sends the request and reads the response.
                ///
                /// # Errors
                ///
                /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, the
                /// broker answers with an error, or the transport fails.
            }))]
            pub async fn $name(
                &self,
                #[builder(into)] entity_type: Option<SelectorTypeBuilder>,
                #[builder(default)] ids: Vec<EntityId>,
                #[builder(into)] id_pattern: Option<IdPattern>,
                #[builder(default, with = |selectors: impl IntoIterator<Item = impl Into<EntitySelectorDraft>>| selectors.into_iter().map(Into::into).collect())]
                selectors: Vec<EntitySelectorDraft>,
                #[builder(into)] q: Option<QueryBuilder>,
                #[builder(into)] geo_q: Option<GeoQueryBuilder>,
                #[builder(into)] scope_q: Option<ScopeQueryBuilder>,
                #[builder(into)] csf: Option<QueryBuilder>,
                #[builder(default)] attrs: Vec<AttributeName>,
                #[builder(into)] pick: Option<ProjectionBuilder>,
                #[builder(into)] omit: Option<ProjectionBuilder>,
                #[builder(default)] expand_values: Vec<AttributeName>,
                #[builder(default)] json_keys: Vec<AttributeName>,
                #[builder(default)] dataset_ids: Vec<DatasetId>,
                #[builder(into)] lang: Option<LanguageFilterBuilder>,
                join: Option<Join>,
                join_level: Option<NonZeroU32>,
                #[builder(default)] contained_by: Vec<EntityId>,
                #[builder(into)] ordering: Option<EntityOrderingBuilder>,
                #[builder(default)] split_entities: EntitySplitting,
                entity_map: Option<&EntityMapRequest>,
                #[builder(default)] system_attributes: SystemAttributes,
                limit: Option<u32>,
                offset: Option<u32>,
                #[builder(default)] count: ResultsCount,
                #[builder(default)] locality: Locality,
                #[builder(default)] method: QueryMethod,
                #[builder(default)] spelling: RepresentationSpelling,
                $($format_parameter)*
                tenant: Option<&Tenant>,
            ) -> Result<M::Answer<Page<$item>>, ClientError<T::Error>> {
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
                let query = QueryEntities {
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
                    join,
                    join_level,
                    contained_by: &contained_by,
                    ordering: ordering.as_ref(),
                    split_entities,
                    entity_map,
                    system_attributes,
                    limit,
                    offset,
                    count,
                    locality,
                    method,
                    spelling,
                    format: $format,
                    completeness: $completeness,
                };
                self.route.send_selection(&query, tenant).await
            }
        }
    };
}

query_method!(
    /// Query Entities (clauses 6.4.3.2 and 6.23.3.1), in normalized or concise representation.
    ///
    /// At least an entity type (every type, `*`, makes the query local), attributes, a `q` or a geoquery is needed
    /// unless the query is local (clause 6.4.3.2), more than one entity selection needs the POST binding (Tables
    /// 6.4.3.2-1 and 5.2.23-1), `limit` 0 needs `count` (clause 6.3.13), and a `pick` or `omit` must keep `id` and
    /// `type` ([`query_projected`] reads an answer without them); `send()` refuses the request otherwise.
    ///
    /// [`query_projected`]: Entities::query_projected
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, an argument is invalid, or the call
    /// fails.
    query, Entity<Output>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Complete
);

query_method!(
    /// Query Entities (clauses 6.4.3.2 and 6.23.3.1), in simplified representation (clause 4.5.4).
    ///
    /// At least an entity type (every type, `*`, makes the query local), attributes, a `q` or a geoquery is needed
    /// unless the query is local (clause 6.4.3.2), more than one entity selection needs the POST binding (Tables
    /// 6.4.3.2-1 and 5.2.23-1), `limit` 0 needs `count` (clause 6.3.13), and a `pick` or `omit` must keep `id` and
    /// `type` ([`query_projected_simplified`] reads an answer without them); `send()` refuses the request otherwise.
    ///
    /// [`query_projected_simplified`]: Entities::query_projected_simplified
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, an argument is invalid, or the call
    /// fails.
    query_simplified, SimplifiedEntity,
    format => Simplified, [],
    completeness => Complete
);

query_method!(
    /// Query Entities (clauses 6.4.3.2 and 6.23.3.1) with a `pick` or `omit` that may remove `id` or `type`
    /// (clause 4.21), in normalized or concise representation.
    ///
    /// At least an entity type (every type, `*`, makes the query local), attributes, a `q` or a geoquery is needed
    /// unless the query is local (clause 6.4.3.2), more than one entity selection needs the POST binding (Tables
    /// 6.4.3.2-1 and 5.2.23-1), and `limit` 0 needs `count` (clause 6.3.13). The projection also applies to linked
    /// entities (clause 4.21), which are read as projected entities.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, an argument is invalid, or the call
    /// fails.
    query_projected, Entity<Output, Projected>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Projected
);

query_method!(
    /// Query Entities (clauses 6.4.3.2 and 6.23.3.1) with a `pick` or `omit` that may remove `id` or `type`
    /// (clause 4.21), in simplified representation (clause 4.5.4).
    ///
    /// At least an entity type (every type, `*`, makes the query local), attributes, a `q` or a geoquery is needed
    /// unless the query is local (clause 6.4.3.2), more than one entity selection needs the POST binding (Tables
    /// 6.4.3.2-1 and 5.2.23-1), and `limit` 0 needs `count` (clause 6.3.13). The projection also applies to linked
    /// entities (clause 4.21), which are read as projected entities.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, an argument is invalid, or the call
    /// fails.
    query_projected_simplified, SimplifiedEntity<Projected>,
    format => Simplified, [],
    completeness => Projected
);

query_method!(
    /// Query Entities (clauses 6.4.3.2 and 6.23.3.1) in `GeoJSON` representation (`Accept: application/geo+json`,
    /// clauses 4.5.16.3 and 6.3.15), the attributes in normalized or concise representation: pages of Features
    /// whose geometry is the value of `geometry_property`, or of `location` when it is not set.
    ///
    /// At least an entity type (every type, `*`, makes the query local), attributes, a `q` or a geoquery is needed
    /// unless the query is local (clause 6.4.3.2), more than one entity selection needs the POST binding (Tables
    /// 6.4.3.2-1 and 5.2.23-1), and `limit` 0 needs `count` (clause 6.3.13); `send()` refuses the request otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, an argument is invalid, or the call
    /// fails.
    query_geo_json, Feature<Entity<Output>>,
    format => GeoJson { form, geometry_property }, [#[builder(default)] form: Lossless, geometry_property: Option<&AttributeName>,],
    completeness => Complete
);

query_method!(
    /// Query Entities (clauses 6.4.3.2 and 6.23.3.1) in simplified `GeoJSON` representation (`Accept:
    /// application/geo+json`, clauses 4.5.17.2 and 6.3.15): pages of Features whose geometry is the value of
    /// `geometry_property`, or of `location` when it is not set.
    ///
    /// At least an entity type (every type, `*`, makes the query local), attributes, a `q` or a geoquery is needed
    /// unless the query is local (clause 6.4.3.2), more than one entity selection needs the POST binding (Tables
    /// 6.4.3.2-1 and 5.2.23-1), and `limit` 0 needs `count` (clause 6.3.13); `send()` refuses the request otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, an argument is invalid, or the call
    /// fails.
    query_geo_json_simplified, Feature<SimplifiedEntity>,
    format => GeoJson { form: Simplified, geometry_property }, [geometry_property: Option<&AttributeName>,],
    completeness => Complete
);
