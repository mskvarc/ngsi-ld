//! Create `EntityMap` for Query Entities (NGSI-LD clauses 5.14.4, 6.34.3.1 and 6.34.3.2), and how the answer of
//! either creation is read.

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
            entity_selection::{EntitySelection, Reach, ResultAssembly},
            linked_entities::LinkedEntities,
            sealed::{BuildsRequest, Sealed},
            stored_document::DocumentReader,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{AttributeName, DatasetId, EntityId, NgsiLdMediaType},
    query::{
        EntitySelector,
        EntitySplitting,
        IdPattern,
        Join,
        Locality,
        SelectorType,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
    },
    registration::EntityMap,
    snapshot::query_document::writer::{QueryMembers, query_object},
};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use jstrict::Object;
use std::num::NonZeroU32;

/// Creates an entity map of the entities a query matches.
///
/// NGSI-LD clause 5.14.4 defines the operation. Its request is `GET /entityMaps` (clause 6.34.3.1) or `POST
/// /entityMaps` (clause 6.34.3.2); both bindings are built from these same fields, as for Query Entities.
///
/// Clause 5.14.4.3 allows the parameters of Query Entities and marks some ignored; these fields are the ones that
/// count. `geometryProperty`, `limit`, the `entityMap` flag ("`EntityMap` is always returned") and the
/// `NGSILD-EntityMap` header are ignored and never sent, and neither are the parameters the clause does not list. A
/// type, `attrs`, a `q` or a geoquery bounds a creation that is not local (clause 5.14.4.4). With POST the selection,
/// the linked entity retrieval, `splitEntities` and `entityMapLifetime` go into the `Query` body (clause 5.2.23), and
/// `local` stays in the query string.
///
/// The answer is the map, with its location in the `NGSILD-EntityMap` header: a [`Retrieved`] whose
/// [`entity_map`](Retrieved::entity_map) is the location.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateEntityMap, Operation},
///     },
///     query::SelectorType,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let every = SelectorType::All;
/// let creation = CreateEntityMap::builder()
///     .entity_type(&every)
///     .lifetime("PT1H".parse()?)
///     .build();
/// let request = creation.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityMaps?type=*&entityMapLifetime=PT1H"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default, bon::Builder)]
pub struct CreateEntityMap<'a> {
    /// The entity types to match (clause 4.17), or every type (`*`), which makes the creation local (Table
    /// 6.4.3.2-1, which clause 5.14.4.3 refers to).
    pub entity_type: Option<&'a SelectorType>,
    /// The entity ids to match.
    #[builder(default)]
    pub ids: &'a [EntityId],
    /// A regular expression the entity ids match.
    pub id_pattern: Option<&'a IdPattern>,
    /// Further entity selectors (clause 5.2.33), each with its own type, ids and id pattern; an entity is matched
    /// when it matches any selection. Only a POST creation carries more than one selection.
    #[builder(default)]
    pub selectors: &'a [EntitySelector],
    /// The attributes of which at least one must exist for an entity to be selected, and which are kept (`attrs`,
    /// clause 5.14.4.3): deprecated, a synonym of a `pick` and a `q`. Empty sends no `attrs`.
    #[builder(default)]
    pub attrs: &'a [AttributeName],
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
    /// The linked entity retrieval (`join`, clause 4.5.23).
    pub join: Option<Join>,
    /// The depth of linked entity retrieval (`joinLevel`); only with a `flat` or `inline` join.
    pub join_level: Option<NonZeroU32>,
    /// The linked entities already encountered (`containedBy`); only with a `joinLevel`.
    #[builder(default)]
    pub contained_by: &'a [EntityId],
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

impl Sealed for CreateEntityMap<'_> {}

impl Operation for CreateEntityMap<'_> {
    type Outcome = Retrieved<EntityMap>;

    /// Reads 201, or for the GET binding 203, the one Table 6.34.3.1-1 adds (clause 4.3.6.8).
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<EntityMap>, ResponseError> {
        match self.method {
            QueryMethod::Get => decode_created(response, &[StatusCode::CREATED, StatusCode::NON_AUTHORITATIVE_INFORMATION]),
            QueryMethod::Post => decode_created(response, &[StatusCode::CREATED]),
        }
    }
}

impl BuildsRequest for CreateEntityMap<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.parts()?.into_request(endpoint, scope)
    }
}

impl CreateEntityMap<'_> {
    /// The entities matched.
    const fn selection(&self) -> EntitySelection<'_> {
        let selection = entity_selection!(self);
        EntitySelection {
            targets: selection.targets.with_selectors(self.selectors),
            ..selection
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

    /// The request, after the check of clause 5.14.4.4 that it is bounded.
    fn parts(&self) -> Result<RequestParts, RequestError> {
        self.selection().check_bounded(self.locality, Reach::Stated)?;
        let assembly = ResultAssembly::splitting(self.split_entities);
        let mut query = QueryString::default();
        let body = match self.method {
            QueryMethod::Get => {
                self.selection().targets.check_query_string()?;
                self.selection().push_query(&mut query);
                self.linked_entities().push_query(&mut query);
                assembly.push_query(&mut query);
                push_lifetime_query(&mut query, self.lifetime);
                None
            }
            QueryMethod::Post => {
                let members = assembly.query_members(self.linked_entities().query_members(self.selection().query_members()?));
                Some(query_object(QueryMembers {
                    entity_map_lifetime: self.lifetime.map(EntityMapLifetime::duration),
                    ..members
                }))
            }
        };
        query.push_some(QueryParameter::Local, self.locality.flag());
        Ok(creation_parts(ResourcePath::entity_maps(), query, body))
    }
}

/// The request of a creation at `path`, answered in JSON: a `GET` with the parameters in `query`, or a `POST` with
/// `body`, the `Query` body (clause 5.2.23), and the parameters the body cannot carry in `query`.
pub(super) fn creation_parts(path: ResourcePath, query: QueryString, body: Option<Object>) -> RequestParts {
    let parts = match body {
        None => RequestParts::resource(Method::GET, path, query),
        Some(body) => RequestParts::resource(Method::POST, path, query).with_body(body),
    };
    parts.accepting(NgsiLdMediaType::Json)
}

/// Adds `entityMapLifetime` to a query string.
pub(super) fn push_lifetime_query(query: &mut QueryString, lifetime: Option<EntityMapLifetime>) {
    query.push_some(QueryParameter::EntityMapLifetime, lifetime);
}

/// Reads the answer of a creation: the map, read strictly, with its location in `NGSILD-EntityMap` and the version
/// of a 203 in `Preference-Applied`, when the status is one of `statuses`.
pub(super) fn decode_created(response: Response<Bytes>, statuses: &[StatusCode]) -> Result<Retrieved<EntityMap>, ResponseError> {
    Retrieved::decode(response, statuses, |response| DocumentReader::document(EntityMap::from_json).one(response))
}
