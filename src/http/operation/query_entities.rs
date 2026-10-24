//! Query Entities (NGSI-LD clauses 5.7.2, 6.4.3.2 and 6.23.3.1).

use crate::{
    http::{
        Endpoint,
        Page,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            EntityMapRequest,
            GeoJson,
            Operation,
            QueryMethod,
            RepresentationSpelling,
            ResultsCount,
            Simplified,
            SnapshotOperation,
            SystemAttributes,
            entity_format::{EntityFormat, decode_page},
            entity_selection::{EntitySelection, Reach, ResultAssembly},
            linked_entities::LinkedEntities,
            parameters::{check_limit, push_paging},
            projection_rule::ProjectionRequest,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{AttributeName, Complete, Completeness, DatasetId, Entity, EntityId, Feature, Lossless, Output, Projected, SimplifiedEntity},
    query::{
        EntitySelector,
        EntitySplitting,
        IdPattern,
        Join,
        Locality,
        SelectorType,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        ordering::EntityOrdering,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
    },
};
use bytes::Bytes;
use http::{Method, Request, Response};
use std::num::NonZeroU32;

mod body;
pub(super) mod parameters;

/// Queries entities (NGSI-LD clause 5.7.2), through `GET /entities` (clause 6.4.3.2) or
/// `POST /entityOperations/query` (clause 6.23.3.1); both bindings are built from these same fields.
///
/// `F` is the representation asked for: a [`Lossless`] one answers with pages of [`Entity`] values, [`Simplified`]
/// with pages of [`SimplifiedEntity`] values, and [`GeoJson`] with pages of [`Feature`] values (clause 6.3.15). `C`
/// is what the entities hold of their `id` and `type`: [`Complete`] entities, so `send()` refuses a `pick` or
/// `omit` that removes either, or [`Projected`] ones, which may lack them (clause 4.21). A Feature always holds
/// both (Table 5.2.29-1), so a `GeoJSON` query reads complete entities whatever the projection.
///
/// The entities are named by `entity_type`, `ids` and `id_pattern`, which the GET binding sends as `type`, `id`
/// and `idPattern` and the POST binding as one `EntitySelector` (clause 5.2.33), and by `selectors`, further
/// selectors each matched on its own, which only the `Query` body of the POST binding can carry beside another
/// (clause 5.2.23): `send()` refuses a GET query with more than one selection.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, QueryEntities},
///     },
///     model::Lossless,
///     query::{SelectorType, q::Query},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles: SelectorType = "Vehicle".parse()?;
/// let fast: Query = "speed>50".parse()?;
/// let query = QueryEntities::builder()
///     .entity_type(&vehicles)
///     .q(&fast)
///     .limit(10)
///     .format(Lossless::Normalized)
///     .build();
/// let request = query.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&q=speed%3E50&limit=10"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
#[builder(start_fn(name = generic_builder, vis = ""))]
pub struct QueryEntities<'a, F, C = Complete> {
    /// What the entities hold of their `id` and `type`, which the start function names.
    #[builder(start_fn)]
    pub completeness: C,
    /// The entity types to match (clause 4.17), or every type (`*`), which makes the query local (Table
    /// 6.4.3.2-1).
    pub entity_type: Option<&'a SelectorType>,
    /// The entity ids to match.
    #[builder(default)]
    pub ids: &'a [EntityId],
    /// A regular expression the entity ids match.
    pub id_pattern: Option<&'a IdPattern>,
    /// Further entity selectors (clause 5.2.33), each with its own type, ids and id pattern; an entity is matched
    /// when it matches any selection. Only a POST query carries more than one selection.
    #[builder(default)]
    pub selectors: &'a [EntitySelector],
    /// The attributes to match and to keep (`attrs`): deprecated by the specification, a synonym of a `pick` of these
    /// attributes and a `q` asking for at least one of them (Table 6.4.3.2-1); prefer `pick` and `q`. Empty sends no
    /// `attrs`.
    #[builder(default)]
    pub attrs: &'a [AttributeName],
    /// The query the entities match (clause 4.9).
    pub q: Option<&'a Query>,
    /// The geoquery the entities match (`georel`, `geometry`, `coordinates`, `geoproperty`, clause 4.10).
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
    /// (`expandValues`, clause 4.9).
    #[builder(default)]
    pub expand_values: &'a [AttributeName],
    /// The attributes whose values the broker does not expand against the `@context`, as uninterpretable as
    /// JSON-LD (`jsonKeys`, clause 4.9).
    #[builder(default)]
    pub json_keys: &'a [AttributeName],
    /// The attribute instances to keep, by dataset id, `@none` naming the default instance (`datasetId`, clause
    /// 4.5.5); empty keeps every instance.
    #[builder(default)]
    pub dataset_ids: &'a [DatasetId],
    /// The languages `LanguageProperty` attributes are reduced to (`lang`, clause 4.15).
    pub lang: Option<&'a LanguageFilter>,
    /// The linked entity retrieval (`join`, clause 4.5.23). The answer is a list of entities whichever join is
    /// asked for: a flat join appends the linked entities to it (clause 4.5.23.3).
    pub join: Option<Join>,
    /// The depth of linked entity retrieval (`joinLevel`); only with a `flat` or `inline` join.
    pub join_level: Option<NonZeroU32>,
    /// The linked entities already encountered while traversing the entity graph (`containedBy`); a broker only
    /// applies it with a `joinLevel`.
    #[builder(default)]
    pub contained_by: &'a [EntityId],
    /// The order of the entities (`orderBy`, `collation`, `orderFrom`, `orderGeometry`, clause 4.23); a broker
    /// applies it to a local query only (Table 5.2.23-1).
    pub ordering: Option<&'a EntityOrdering>,
    /// Whether an entity may be split over several context sources (`splitEntities`).
    #[builder(default)]
    pub split_entities: EntitySplitting,
    /// The entity map to use or to return (`entityMap`, `entityMapLifetime`, `NGSILD-EntityMap`, clause 4.5.25).
    pub entity_map: Option<&'a EntityMapRequest>,
    /// Whether system attributes are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// The page size (clause 6.3.10).
    pub limit: Option<u32>,
    /// The index of the first result of the page (clause 6.3.10).
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
    /// How the representation is written (clause 6.3.7).
    #[builder(default)]
    pub spelling: RepresentationSpelling,
    /// The representation asked for.
    pub format: F,
}

impl<'a, F> QueryEntities<'a, F> {
    /// Starts the builder of a query of complete entities.
    pub fn builder() -> QueryEntitiesBuilder<'a, F> {
        Self::generic_builder(Complete)
    }
}

impl<'a, F> QueryEntities<'a, F, Projected> {
    /// Starts the builder of a query of entities a `pick` or `omit` may leave without their `id` or `type`.
    pub fn projected_builder() -> QueryEntitiesBuilder<'a, F, Projected> {
        Self::generic_builder(Projected)
    }
}

impl<F, C> Sealed for QueryEntities<'_, F, C> {}

/// Implements [`Operation`] for one representation and one completeness, answered with pages of `$item`.
macro_rules! query_operation {
    ($format:ty, $completeness:ty, $item:ty) => {
        impl Operation for QueryEntities<'_, $format, $completeness> {
            type Outcome = Page<$item>;

            fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Page<$item>, ResponseError> {
                decode_page::<$completeness, _>(context, response, self.format)
            }
        }

        impl BuildsRequest for QueryEntities<'_, $format, $completeness> {
            fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                parts(self, Reach::Stated)?.into_request(endpoint, scope)
            }

            fn scoped_snapshot_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                parts(self, Reach::Snapshot)?.into_request(endpoint, scope)
            }
        }

        impl SnapshotOperation for QueryEntities<'_, $format, $completeness> {}
    };
}

query_operation!(Lossless, Complete, Entity<Output>);
query_operation!(Simplified, Complete, SimplifiedEntity);
query_operation!(Lossless, Projected, Entity<Output, Projected>);
query_operation!(Simplified, Projected, SimplifiedEntity<Projected>);
query_operation!(GeoJson<'_, Lossless>, Complete, Feature<Entity<Output>>);
query_operation!(GeoJson<'_, Simplified>, Complete, Feature<SimplifiedEntity>);

impl<F, C> QueryEntities<'_, F, C> {
    /// The entities matched and what of them the answer holds.
    const fn selection(&self) -> EntitySelection<'_> {
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

    /// The linked entity retrieval asked for.
    const fn linked_entities(&self) -> LinkedEntities<'_> {
        LinkedEntities {
            join: self.join,
            level: self.join_level,
            contained_by: self.contained_by,
        }
    }
}

/// The request for `entities`, after the checks clauses 6.3.13 and 6.4.3.2 call for, and the check that the answer can
/// be read in the format asked for. A type, `attrs`, a `q` or a geoquery bounds a query that is not local (Table
/// 6.4.3.2-1); a scope query alone does not, and a query on a snapshot needs none (clause 5.5.15).
fn parts<C: Completeness, F: EntityFormat<C>>(entities: &QueryEntities<'_, F, C>, reach: Reach) -> Result<RequestParts, RequestError> {
    check_limit(entities.limit, entities.count)?;
    entities.selection().check_bounded(entities.locality, reach)?;
    entities.format.check_projection(ProjectionRequest {
        pick: entities.pick,
        omit: entities.omit,
        join: entities.join,
    })?;
    let parts = match entities.method {
        QueryMethod::Get => {
            entities.selection().targets.check_query_string()?;
            let mut query = QueryString::default();
            entities.selection().push_query(&mut query);
            entities.linked_entities().push_query(&mut query);
            entities.assembly().push_query(&mut query);
            push_shared(entities, &mut query);
            RequestParts::resource(Method::GET, ResourcePath::entities(), query)
        }
        QueryMethod::Post => {
            let mut query = QueryString::default();
            push_shared(entities, &mut query);
            RequestParts::resource(Method::POST, ResourcePath::query_entities(), query).with_body(entities.query_body()?)
        }
    };
    Ok(parts
        .with_entity_map(entities.entity_map.and_then(EntityMapRequest::header))
        .accepting(entities.format.media_type()))
}

/// The parameters both bindings carry in the query string: the format and the options, which the `Query` body of
/// clause 5.2.23 cannot carry, and the paging, count and locality of clauses 6.3.10, 6.3.13 and 6.3.18.
fn push_shared<C: Completeness, F: EntityFormat<C>>(entities: &QueryEntities<'_, F, C>, query: &mut QueryString) {
    entities.format.push_query(query, entities.spelling);
    entities
        .spelling
        .push_options(query, entities.format.representation(), entities.system_attributes);
    push_paging(query, entities.limit, entities.offset, entities.count);
    query.push_some(QueryParameter::Local, entities.locality.flag());
}
