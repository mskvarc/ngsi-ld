//! Replace Entity (NGSI-LD clauses 5.6.18 and 6.5.3.3).

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
    model::{Entity, Input, Lossless, representation::write::write_entity},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Replaces an entity with a complete new representation.
///
/// Its request is `PUT /entities/{entityId}` (NGSI-LD clause 6.5.3.3). The entity is addressed by the id of the new
/// representation.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, ReplaceEntity},
///     },
///     model::Entity,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicle = Entity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///     .entity_type("Vehicle")
///     .property("speed", 30)
///     .build()?;
/// let request = ReplaceEntity::builder()
///     .entity(&vehicle)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "PUT");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct ReplaceEntity<'a> {
    /// The new representation.
    pub entity: &'a Entity<Input>,
    /// Replaces the entity only if its type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for ReplaceEntity<'_> {}

impl Operation for ReplaceEntity<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for ReplaceEntity<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::PUT, ResourcePath::entity(&self.entity.id), query)
            .with_body(write_entity(self.entity, self.form))
            .into_request(endpoint, scope)
    }
}
