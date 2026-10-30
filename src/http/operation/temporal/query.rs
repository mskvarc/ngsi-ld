//! Query Temporal Evolution of Entities (NGSI-LD clauses 5.7.4, 6.18.3.2 and 6.24.3.1), and the requests for its
//! following pages.

use crate::{
    http::{
        Endpoint,
        Page,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Aggregated,
            ArgumentError,
            EntityMapRequest,
            Operation,
            QueryMethod,
            RepresentationSpelling,
            ResultsCount,
            SimplifiedTemporal,
            SnapshotOperation,
            SystemAttributes,
            TemporalSelection,
            entity_selection::{EntitySelection, Reach, ResultAssembly},
            parameters::{check_limit, push_paging},
            projection_rule::{ProjectionRequest, ProjectionRule},
            sealed::{BuildsRequest, Sealed},
            temporal::{TemporalFormat, check_spelling, decode_page},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{
        AttributeName,
        Complete,
        Completeness,
        DatasetId,
        EntityId,
        Lossless,
        NgsiLdMediaType,
        Output,
        Projected,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalAttributeName, TemporalEntity},
    },
    query::{
        EntitySelector,
        EntitySplitting,
        IdPattern,
        Locality,
        SelectorType,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        ordering::{EntityOrdering, OrderingKey, SortDirection},
        projection::{EntityMember, Projection},
        q::Query,
        scope_query::ScopeQuery,
        temporal_query::TemporalQuery,
    },
    snapshot::query_document::writer::{QueryMembers, query_object},
};
use bytes::Bytes;
use http::{Method, Request, Response};
use jstrict::Object;

/// Queries the temporal evolution of entities.
///
/// NGSI-LD clause 5.7.4 defines the operation. Its request is `GET /temporal/entities` (clause 6.18.3.2) or `POST
/// /temporal/entityOperations/query` (clause 6.24.3.1); both bindings are built from these same fields, as for Query
/// Entities.
///
/// `F` is the representation asked for: a [`Lossless`] one answers with pages of [`TemporalEntity`] values,
/// [`SimplifiedTemporal`] with pages of [`SimplifiedTemporalEntity`] values and [`Aggregated`] with pages of
/// [`AggregatedTemporalEntity`] values (clause 6.3.12). `C` is what the entities hold of their `id` and `type`
/// (clause 4.21).
///
/// The entities are named by `entity_type`, `ids` and `id_pattern`, which the GET binding sends as `type`, `id` and
/// `idPattern` and the POST binding as one `EntitySelector` (clause 5.2.33), and by `selectors`, further selectors each
/// matched on its own, which only the `Query` body of the POST binding can carry beside another (clause 5.2.23):
/// `send()` refuses a GET query with more than one selection.
///
/// The temporal query is mandatory (Table 6.18.3.2-1 gives `timerel` and `timeAt` a cardinality of 1). The other
/// parameters are those of Table 6.18.3.2-1, the paging and count of clauses 4.12, 6.3.10 and 6.3.13, `options`
/// (clause 6.3.11), `format` (clause 6.3.12) and `local` (clause 6.3.18). The ordering may only order by `id`
/// (Table 6.18.3.2-1, clause 5.7.4.3). With POST the selection, the temporal query (as `temporalQ`, clause 5.2.21)
/// and the ordering go into the `Query` body (clause 5.2.23); the format, paging, count and locality stay in the
/// query string.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, QueryTemporalEvolution},
///     },
///     model::Lossless,
///     query::{SelectorType, temporal_query::TemporalQuery},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles: SelectorType = "Vehicle".parse()?;
/// let since = TemporalQuery::after("2018-08-01T12:00:00Z".parse()?);
/// let query = QueryTemporalEvolution::builder()
///     .entity_type(&vehicles)
///     .temporal_q(&since)
///     .format(Lossless::Normalized)
///     .build();
/// let request = query.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/temporal/entities",
///         "?type=Vehicle&timerel=after&",
///         "timeAt=2018-08-01T12%3A00%3A00Z"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
#[builder(start_fn(name = generic_builder, vis = ""))]
pub struct QueryTemporalEvolution<'a, F = Lossless, C = Complete> {
    /// What the entities hold of their `id` and `type`, which the start function names.
    #[builder(start_fn)]
    pub completeness: C,
    /// The entity types to match (clause 4.17), or every type (`*`), which makes the query local (Table
    /// 6.18.3.2-1).
    pub entity_type: Option<&'a SelectorType>,
    /// The entity ids to match.
    #[builder(default)]
    pub ids: &'a [EntityId],
    /// A regular expression the entity ids match (clause 5.2.33).
    pub id_pattern: Option<&'a IdPattern>,
    /// Further entity selectors (clause 5.2.33), each with its own type, ids and id pattern; an entity is matched
    /// when it matches any selection. Only a POST query carries more than one selection.
    #[builder(default)]
    pub selectors: &'a [EntitySelector],
    /// The attributes to match and to keep (`attrs`): deprecated by the specification, a synonym of a `pick` of these
    /// attributes and a `q` asking for at least one of them (Table 6.18.3.2-1); prefer `pick` and `q`. It may name
    /// `scope`, which the temporal representation reifies (clauses 4.18 and C.5.16.2). Empty sends no `attrs`.
    #[builder(default)]
    pub attrs: &'a [TemporalAttributeName],
    /// The query the entities match (clause 4.9).
    pub q: Option<&'a Query>,
    /// The geoquery the entities match (clause 4.10).
    pub geo_q: Option<&'a GeoQuery>,
    /// The scopes the entities are within (`scopeQ`, clause 4.19).
    pub scope_q: Option<&'a ScopeQuery>,
    /// The context source filter (clause 4.9), applied to registrations.
    pub csf: Option<&'a Query>,
    /// The members to keep (`pick`, clause 4.21).
    pub pick: Option<&'a Projection>,
    /// The members to remove (`omit`, clause 4.21).
    pub omit: Option<&'a Projection>,
    /// The attributes whose values the broker expands against the `@context` before evaluating the query
    /// (`expandValues`).
    #[builder(default)]
    pub expand_values: &'a [AttributeName],
    /// The attributes whose values the broker does not expand (`jsonKeys`).
    #[builder(default)]
    pub json_keys: &'a [AttributeName],
    /// The attribute instances to keep, by dataset id (`datasetId`, clause 4.5.5); empty keeps every instance.
    #[builder(default)]
    pub dataset_ids: &'a [DatasetId],
    /// The languages `LanguageProperty` attributes are reduced to (`lang`, clause 4.15).
    pub lang: Option<&'a LanguageFilter>,
    /// The instances of each attribute, by time, with `lastN` and the aggregation (clause 4.11).
    pub temporal_q: &'a TemporalQuery,
    /// The order of the entities (`orderBy` and `collation`, clause 4.23), by `id` only.
    pub ordering: Option<&'a EntityOrdering>,
    /// Whether an entity may be split over several context sources (`splitEntities`).
    #[builder(default)]
    pub split_entities: EntitySplitting,
    /// The entity map to use or to return (`entityMap`, `entityMapLifetime`, `NGSILD-EntityMap`).
    pub entity_map: Option<&'a EntityMapRequest>,
    /// Whether system attributes, and `deletedAt` of deleted instances, are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// The page size (clause 6.3.10).
    pub limit: Option<u32>,
    /// The index of the first entity of the page (clause 6.3.10).
    pub offset: Option<u32>,
    /// Whether the total is reported (clause 6.3.13).
    #[builder(default)]
    pub count: ResultsCount,
    /// Whether the query is limited to the broker's own data (clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
    /// The binding used.
    #[builder(default)]
    pub method: QueryMethod,
    /// The representation asked for.
    pub format: F,
    /// How the representation is written: in `format`, `options` or both (clause 6.3.12).
    #[builder(default)]
    pub spelling: RepresentationSpelling,
}

impl<'a, F> QueryTemporalEvolution<'a, F> {
    /// Starts the builder of a query of the temporal evolution of complete entities.
    pub fn builder() -> QueryTemporalEvolutionBuilder<'a, F> {
        Self::generic_builder(Complete)
    }
}

impl<'a, F> QueryTemporalEvolution<'a, F, Projected> {
    /// Starts the builder of a query of the temporal evolution of entities a `pick` or `omit` may leave without their
    /// `id` or `type`.
    pub fn projected_builder() -> QueryTemporalEvolutionBuilder<'a, F, Projected> {
        Self::generic_builder(Projected)
    }
}

impl<F, C> Sealed for QueryTemporalEvolution<'_, F, C> {}

/// Implements [`Operation`] for one representation and one completeness, answered with pages of `$item`, and for
/// the requests of the following pages, read in `$page_format`.
macro_rules! query_operation {
    ($format:ty, $completeness:ty, $item:ty, $page_format:expr) => {
        impl Operation for QueryTemporalEvolution<'_, $format, $completeness> {
            type Outcome = Page<$item>;

            fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Page<$item>, ResponseError> {
                decode_page::<$completeness, _>(context, response, self.format)
            }
        }

        impl BuildsRequest for QueryTemporalEvolution<'_, $format, $completeness> {
            fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                parts(self, Reach::Stated)?.into_request(endpoint, scope)
            }

            fn scoped_snapshot_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                parts(self, Reach::Snapshot)?.into_request(endpoint, scope)
            }
        }

        impl SnapshotOperation for QueryTemporalEvolution<'_, $format, $completeness> {}

        page_operation!(
            impl for $item => |context, response| decode_page::<$completeness, _>(context, response, $page_format),
            NgsiLdMediaType::Json
        );
    };
}

// A page link carries the `format` of the query; the concise reader also reads normalized instances (clause
// 4.5.2.3), so it reads the pages of either lossless representation.
query_operation!(Lossless, Complete, TemporalEntity<Output>, Lossless::Concise);
query_operation!(SimplifiedTemporal, Complete, SimplifiedTemporalEntity, SimplifiedTemporal);
query_operation!(Aggregated, Complete, AggregatedTemporalEntity, Aggregated);
query_operation!(Lossless, Projected, TemporalEntity<Output, Projected>, Lossless::Concise);
query_operation!(SimplifiedTemporal, Projected, SimplifiedTemporalEntity<Projected>, SimplifiedTemporal);
query_operation!(Aggregated, Projected, AggregatedTemporalEntity<Projected>, Aggregated);

impl<F, C> QueryTemporalEvolution<'_, F, C> {
    /// The entities matched and what of them the answer holds.
    const fn selection(&self) -> EntitySelection<'_, TemporalAttributeName> {
        let selection = entity_selection!(self);
        EntitySelection {
            targets: selection.targets.with_selectors(self.selectors),
            ..selection
        }
    }

    /// How the answer is assembled.
    const fn assembly(&self) -> ResultAssembly<'_> {
        ResultAssembly {
            ordering: self.ordering,
            split_entities: self.split_entities,
            entity_map: self.entity_map,
        }
    }

    /// The `Query` body (Table 5.2.23-1), written by the writer of the `Query` document: the selection, `temporalQ`
    /// (clause 5.2.21) and the assembly.
    fn query_body(&self) -> Result<Object, RequestError> {
        let members = QueryMembers {
            temporal_q: Some(self.temporal_q),
            ..self.selection().query_members()?
        };
        Ok(query_object(self.assembly().query_members(members)))
    }
}

/// The request, after the checks of clauses 6.3.13 and 5.7.4.3 and the checks the format and the completeness call
/// for.
fn parts<C: Completeness + ProjectionRule, F: TemporalFormat<C>>(query: &QueryTemporalEvolution<'_, F, C>, reach: Reach) -> Result<RequestParts, RequestError> {
    check_limit(query.limit, query.count)?;
    query.selection().check_bounded(query.locality, reach)?;
    check_ordering(query.ordering)?;
    let selection = TemporalSelection::Query(query.temporal_q);
    query.format.check(selection)?;
    check_spelling(query.spelling, query.format.representation(), selection)?;
    C::check(ProjectionRequest {
        pick: query.pick,
        omit: query.omit,
        join: None,
    })?;
    let parts = match query.method {
        QueryMethod::Get => {
            query.selection().targets.check_query_string()?;
            let mut parameters = QueryString::default();
            query.selection().push_query(&mut parameters);
            selection.push_query(&mut parameters);
            query.assembly().push_query(&mut parameters);
            push_shared(query, &mut parameters);
            RequestParts::resource(Method::GET, ResourcePath::temporal_entities(), parameters)
        }
        QueryMethod::Post => {
            let mut parameters = QueryString::default();
            push_shared(query, &mut parameters);
            RequestParts::resource(Method::POST, ResourcePath::query_temporal(), parameters).with_body(query.query_body()?)
        }
    };
    Ok(parts
        .with_entity_map(query.entity_map.and_then(EntityMapRequest::header))
        .accepting(NgsiLdMediaType::Json))
}

/// Refuses an ordering by anything but `id` (Table 6.18.3.2-1, clause 5.7.4.3), or by distance.
fn check_ordering(ordering: Option<&EntityOrdering>) -> Result<(), ArgumentError> {
    let Some(ordering) = ordering else {
        return Ok(());
    };
    let by_id = ordering.order_by().terms().iter().all(|term| {
        matches!(term.key(), OrderingKey::Member(EntityMember::Id))
            && matches!(term.direction(), None | Some(SortDirection::Ascending | SortDirection::Descending))
    });
    if by_id && ordering.origin().is_none() {
        Ok(())
    } else {
        Err(ArgumentError::TemporalOrdering)
    }
}

/// The parameters both bindings carry in the query string: the format, which the `Query` body cannot carry, written
/// as `format`, `options` or both (clause 6.3.12), and the system attributes, paging, count and locality of clauses
/// 6.3.10, 6.3.11, 6.3.13 and 6.3.18.
fn push_shared<C: Completeness, F: TemporalFormat<C>>(query: &QueryTemporalEvolution<'_, F, C>, parameters: &mut QueryString) {
    query.spelling.push_format(parameters, query.format.representation());
    query.spelling.push_options(parameters, query.format.representation(), query.system_attributes);
    push_paging(parameters, query.limit, query.offset, query.count);
    parameters.push_some(QueryParameter::Local, query.locality.flag());
}
