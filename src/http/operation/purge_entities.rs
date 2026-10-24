//! Purge Entities (NGSI-LD clauses 5.6.21 and 6.4.3.3).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        headers::NgsiLdEntityMap,
        operation::{
            Operation,
            SnapshotOperation,
            entity_selection::{EntitySelection, Reach, targets::EntityTargets},
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{AttributeName, EntityId},
    query::{IdPattern, Locality, SelectorType, geoquery::GeoQuery, q::Query, scope_query::ScopeQuery},
};
use bytes::Bytes;
use http::{Method, Request, Response};

pub(super) mod parameters;

/// `DELETE /entities`: deletes every matching entity, or, with `drop` or `keep`, only some of their attributes
/// (NGSI-LD clause 6.4.3.3).
///
/// Clause 6.4.3.3 requires a type, a `q` or a geoquery unless the operation is local, as Query Entities does; a
/// scope query alone does not count (Table 6.4.3.3-1). The request is refused before it is sent otherwise. Every
/// type (`*`) makes the purge local, and is refused with `local=false` (Table 6.4.3.3-1).
///
/// Table 6.4.3.3-2 lets the request name an existing entity map in the `NGSILD-EntityMap` header, which the broker
/// uses to find the entities to purge; unlike a query, Purge Entities takes no `entityMap` or `entityMapLifetime`
/// parameter (Table 6.4.3.3-1), so the header is all it carries of an entity map.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, PurgeEntities},
///     },
///     model::AttributeName,
///     query::SelectorType,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles: SelectorType = "Vehicle".parse()?;
/// let speed: AttributeName = "speed".parse()?;
/// let drop = [speed];
/// let purge = PurgeEntities::builder()
///     .entity_type(&vehicles)
///     .drop(&drop)
///     .build();
/// let request = purge.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&drop=speed"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct PurgeEntities<'a> {
    /// The entity types to match (clause 4.17), or every type (`*`).
    pub entity_type: Option<&'a SelectorType>,
    /// The entity ids to match.
    #[builder(default)]
    pub ids: &'a [EntityId],
    /// A regular expression the entity ids match (clause 5.2.33).
    pub id_pattern: Option<&'a IdPattern>,
    /// The query the entities match (clause 4.9).
    pub q: Option<&'a Query>,
    /// The geoquery the entities match (`georel`, `geometry`, `coordinates`, `geoproperty`, clause 4.10).
    pub geo_q: Option<&'a GeoQuery>,
    /// The scopes the entities are within (`scopeQ`, clause 4.19).
    pub scope_q: Option<&'a ScopeQuery>,
    /// The context source filter (clause 4.9).
    pub csf: Option<&'a Query>,
    /// Attributes removed from each matching entity, instead of deleting the entity.
    #[builder(default)]
    pub drop: &'a [AttributeName],
    /// Attributes kept on each matching entity; every other attribute is removed.
    #[builder(default)]
    pub keep: &'a [AttributeName],
    /// Whether the purge is limited to the broker's own data (clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
    /// The entity map that determines the entities purged (the `NGSILD-EntityMap` header, Table 6.4.3.3-2).
    pub entity_map: Option<&'a NgsiLdEntityMap>,
}

impl Sealed for PurgeEntities<'_> {}

impl Operation for PurgeEntities<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for PurgeEntities<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.request(endpoint, scope, Reach::Stated)
    }

    fn scoped_snapshot_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.request(endpoint, scope, Reach::Snapshot)
    }
}

impl SnapshotOperation for PurgeEntities<'_> {}

impl PurgeEntities<'_> {
    /// The request, refused when the purge names no type, `q` or geoquery and is neither local nor on a snapshot,
    /// which is local by itself (clause 5.5.15), and when it names every type with `local=false`.
    fn request(&self, endpoint: &Endpoint, scope: RequestScope<'_>, reach: Reach) -> Result<Request<Bytes>, RequestError> {
        let selection = EntitySelection::<AttributeName> {
            q: self.q,
            geo_q: self.geo_q,
            scope_q: self.scope_q,
            csf: self.csf,
            ..EntitySelection::of(EntityTargets::parameters(self.entity_type, self.ids, self.id_pattern))
        };
        selection.check_bounded(self.locality, reach)?;
        let mut query = QueryString::default();
        selection.push_query(&mut query);
        query.push_list(QueryParameter::Drop, self.drop);
        query.push_list(QueryParameter::Keep, self.keep);
        query.push_some(QueryParameter::Local, self.locality.flag());
        RequestParts::resource(Method::DELETE, ResourcePath::entities(), query)
            .with_entity_map(self.entity_map)
            .into_request(endpoint, scope)
    }
}
