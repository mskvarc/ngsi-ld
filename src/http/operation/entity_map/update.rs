//! Update `EntityMap` (NGSI-LD clauses 5.14.2 and 6.32.3.2).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            no_content::decode_no_content,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::DateTime,
    registration::{EntityMap, EntityMapId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Moves the expiry of an entity map.
///
/// Its request is `PATCH /entityMaps/{entityMapId}` (NGSI-LD clause 6.32.3.2).
///
/// Clause 5.14.2.3 sends "a JSON-LD document representing an `EntityMap`", and a broker ignores the output-only
/// members a client could add (clause 5.14.2.4), so the expiry is the one member a client can change. The body is
/// the members of Table 5.2.39-1: `{"id": ..., "type": "EntityMap", "expiresAt": ...}`.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, UpdateEntityMap},
///     },
///     model::DateTime,
///     registration::EntityMapId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityMapId::from(iri!("urn:ngsi-ld:EntityMap:1"));
/// let expiry: DateTime = "2030-01-01T00:00:00Z".parse()?;
/// let request = UpdateEntityMap {
///     id: &id,
///     expires_at: &expiry,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateEntityMap<'a> {
    /// The entity map.
    pub id: &'a EntityMapId,
    /// When the map is to expire.
    pub expires_at: &'a DateTime,
}

impl Sealed for UpdateEntityMap<'_> {}

impl Operation for UpdateEntityMap<'_> {
    type Outcome = ();

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for UpdateEntityMap<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::PATCH, ResourcePath::entity_map(self.id), QueryString::default())
            .with_body(EntityMap::update_object(self.id, self.expires_at))
            .into_request(endpoint, scope)
    }
}
