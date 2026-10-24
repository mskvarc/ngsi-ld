//! Delete Entity (NGSI-LD clauses 5.6.6 and 6.5.3.2).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::EntityId,
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Deletes an entity.
///
/// Its request is `DELETE /entities/{entityId}` (NGSI-LD clause 6.5.3.2).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteEntity, Operation},
///     },
///     model::EntityId,
///     query::type_selection::TypeSelection,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let vehicles: TypeSelection = "Vehicle".parse()?;
/// let request = DeleteEntity::builder()
///     .id(&id)
///     .entity_type(&vehicles)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567?type=Vehicle"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct DeleteEntity<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// Deletes the entity only if its type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
}

impl Sealed for DeleteEntity<'_> {}

impl Operation for DeleteEntity<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for DeleteEntity<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::DELETE, ResourcePath::entity(self.id), query).into_request(endpoint, scope)
    }
}
