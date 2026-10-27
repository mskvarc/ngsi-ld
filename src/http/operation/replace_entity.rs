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

/// Whether the body of Replace Entity carries the `id` member.
///
/// The entity is named by the request path (NGSI-LD clause 6.5.3.3), so a body that leaves the `id` out is a complete
/// representation of that entity all the same (Table 6.5.3.3-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EntityIdMember {
    /// The body holds the `id` of the new representation.
    #[default]
    Written,
    /// The body leaves the `id` out; the path still names the entity.
    Omitted,
}

/// Replaces an entity with a complete new representation.
///
/// Its request is `PUT /entities/{entityId}` (NGSI-LD clause 6.5.3.3). The entity is addressed by the id of the new
/// representation, which the body repeats unless [`EntityIdMember::Omitted`] leaves it out.
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
    /// Whether the body repeats the `id` the path names.
    #[builder(default)]
    pub id_member: EntityIdMember,
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
        let mut body = write_entity(self.entity, self.form);
        match self.id_member {
            EntityIdMember::Written => {}
            EntityIdMember::Omitted => {
                body.remove("id");
            }
        }
        RequestParts::resource(Method::PUT, ResourcePath::entity(&self.entity.id), query)
            .with_body(body)
            .into_request(endpoint, scope)
    }
}
