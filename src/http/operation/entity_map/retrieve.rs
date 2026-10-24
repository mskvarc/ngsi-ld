//! Retrieve `EntityMap` (NGSI-LD clauses 5.14.1 and 6.32.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
            stored_document::DocumentReader,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::NgsiLdMediaType,
    registration::{EntityMap, EntityMapId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Reads an entity map the broker stores.
///
/// Its request is `GET /entityMaps/{entityMapId}` (NGSI-LD clause 6.32.3.1).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveEntityMap},
///     },
///     registration::EntityMapId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityMapId::from(iri!("urn:ngsi-ld:EntityMap:1"));
/// let request = RetrieveEntityMap { id: &id }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrieveEntityMap<'a> {
    /// The entity map.
    pub id: &'a EntityMapId,
}

impl Sealed for RetrieveEntityMap<'_> {}

impl Operation for RetrieveEntityMap<'_> {
    type Outcome = EntityMap;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<EntityMap, ResponseError> {
        DocumentReader::document(EntityMap::from_json).decode_one(response)
    }
}

impl BuildsRequest for RetrieveEntityMap<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::GET, ResourcePath::entity_map(self.id), QueryString::default())
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
