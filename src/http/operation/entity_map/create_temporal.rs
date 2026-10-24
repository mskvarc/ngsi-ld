//! Create `EntityMap` for Query Temporal Evolution of Entities (NGSI-LD clauses 5.14.5, 6.35.3.1 and 6.35.3.2).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        Retrieved,
        operation::{
            EntityMapLifetime,
            Operation,
            QueryMethod,
            TemporalSelection,
            entity_map::create::{creation_parts, decode_created, push_lifetime_query},
            entity_selection::{EntitySelection, Reach, ResultAssembly},
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{AttributeName, DatasetId, EntityId, evolution::TemporalAttributeName},
    query::{
        EntitySplitting,
        IdPattern,
        Locality,
        SelectorType,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
        temporal_query::TemporalQuery,
    },
    registration::EntityMap,
    snapshot::query_document::writer::{QueryMembers, query_object},
};
use bytes::Bytes;
use http::{Request, Response, StatusCode};

/// Creates an entity map of the entities whose temporal evolution a query matches.
///
/// NGSI-LD clause 5.14.5 defines the operation, whose query is a temporal query. Its request is `GET
/// /temporal/entityMaps` (clause 6.35.3.1) or `POST /temporal/entityMaps` (clause 6.35.3.2); both bindings are built
/// from these same fields, as for Query Temporal Evolution.
///
/// The temporal query is mandatory (clause 5.14.5.4), and carries `lastN` with it. Clause 5.14.5.3 allows the
/// parameters of Query Temporal Evolution and marks some ignored; these fields are the ones that count, and the ignored
/// ones are never sent. Clause 5.14.5.4 refuses linked entity retrieval, so there is no `join`. A type, `attrs`, a `q`
/// or a geoquery bounds a creation that is not local. With POST the selection, `temporalQ`, `splitEntities` and
/// `entityMapLifetime` go into the `Query` body (clause 5.2.23), and `local` stays in the query string.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateTemporalEntityMap, Operation},
///     },
///     query::{SelectorType, temporal_query::TemporalQuery},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles: SelectorType = "Vehicle".parse()?;
/// let since = TemporalQuery::after("2024-05-01T10:00:00Z".parse()?);
/// let creation = CreateTemporalEntityMap::builder()
///     .entity_type(&vehicles)
///     .temporal_q(&since)
///     .build();
/// let request = creation.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/temporal/entityMaps",
///         "?type=Vehicle&timerel=after&",
///         "timeAt=2024-05-01T10%3A00%3A00Z"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct CreateTemporalEntityMap<'a> {
    /// The entity types to match (clause 4.17), or every type (`*`), which makes the creation local (Table
    /// 6.18.3.2-1, which clause 5.14.5.3 refers to).
    pub entity_type: Option<&'a SelectorType>,
    /// The entity ids to match.
    #[builder(default)]
    pub ids: &'a [EntityId],
    /// A regular expression the entity ids match.
    pub id_pattern: Option<&'a IdPattern>,
    /// The attributes of which at least one must exist for an entity to be selected, and which are kept (`attrs`,
    /// clause 5.14.5.3): deprecated, a synonym of a `pick` and a `q`. As in Query Temporal Evolution, it may name
    /// `scope`, which the temporal representation reifies (clause 4.18). Empty sends no `attrs`.
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
    /// The attributes whose values the broker expands before evaluating the query (`expandValues`).
    #[builder(default)]
    pub expand_values: &'a [AttributeName],
    /// The attributes whose values the broker does not expand (`jsonKeys`).
    #[builder(default)]
    pub json_keys: &'a [AttributeName],
    /// The attribute instances to keep, by dataset id (`datasetId`, clause 4.5.5).
    #[builder(default)]
    pub dataset_ids: &'a [DatasetId],
    /// The languages `LanguageProperty` attributes are reduced to (`lang`, clause 4.15).
    pub lang: Option<&'a LanguageFilter>,
    /// The instances of each attribute, by time, with `lastN` (clause 4.11).
    pub temporal_q: &'a TemporalQuery,
    /// The suggested lifetime of the map (`entityMapLifetime`); the broker sets the actual expiry.
    pub lifetime: Option<EntityMapLifetime>,
    /// Whether an entity may be split over several context sources (`splitEntities`).
    #[builder(default)]
    pub split_entities: EntitySplitting,
    /// Whether the map is limited to the broker's own entities (clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
    /// The binding used.
    #[builder(default)]
    pub method: QueryMethod,
}

impl Sealed for CreateTemporalEntityMap<'_> {}

impl Operation for CreateTemporalEntityMap<'_> {
    type Outcome = Retrieved<EntityMap>;

    /// Reads 201, the one success Tables 6.35.3.1-1 and 6.35.3.2-1 list.
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<EntityMap>, ResponseError> {
        decode_created(response, &[StatusCode::CREATED])
    }
}

impl BuildsRequest for CreateTemporalEntityMap<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.parts()?.into_request(endpoint, scope)
    }
}

impl CreateTemporalEntityMap<'_> {
    /// The entities matched.
    const fn selection(&self) -> EntitySelection<'_, TemporalAttributeName> {
        entity_selection!(self)
    }

    /// The request, after the check of clause 5.14.5.4 that it is bounded.
    fn parts(&self) -> Result<RequestParts, RequestError> {
        self.selection().check_bounded(self.locality, Reach::Stated)?;
        let assembly = ResultAssembly::splitting(self.split_entities);
        let mut query = QueryString::default();
        let body = match self.method {
            QueryMethod::Get => {
                self.selection().push_query(&mut query);
                TemporalSelection::Query(self.temporal_q).push_query(&mut query);
                assembly.push_query(&mut query);
                push_lifetime_query(&mut query, self.lifetime);
                None
            }
            QueryMethod::Post => {
                let members = QueryMembers {
                    temporal_q: Some(self.temporal_q),
                    ..self.selection().query_members()?
                };
                Some(query_object(QueryMembers {
                    entity_map_lifetime: self.lifetime.map(EntityMapLifetime::duration),
                    ..assembly.query_members(members)
                }))
            }
        };
        query.push_some(QueryParameter::Local, self.locality.flag());
        Ok(creation_parts(ResourcePath::temporal_entity_maps(), query, body))
    }
}
