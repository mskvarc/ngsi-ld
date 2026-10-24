//! Delete Attribute Instance from Temporal Evolution of an Entity (NGSI-LD clauses 5.6.15 and 6.22.3.2).

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
    model::{AttributeName, EntityId, InstanceId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `DELETE /temporal/entities/{entityId}/attrs/{attrId}/{instanceId}`: deletes one instance of an attribute from
/// the temporal evolution of an entity (NGSI-LD clause 6.22.3.2).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteAttributeInstance, Operation},
///     },
///     model::{AttributeName, EntityId, InstanceId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let speed: AttributeName = "speed".parse()?;
/// let instance = InstanceId::from(iri!("urn:ngsi-ld:Instance:1"));
/// let request = DeleteAttributeInstance {
///     id: &id,
///     name: &speed,
///     instance_id: &instance,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/temporal/entities",
///         "/urn:ngsi-ld:Vehicle:B9211/attrs/speed",
///         "/urn:ngsi-ld:Instance:1"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAttributeInstance<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The instance deleted (`instanceId`, clause 4.5.7).
    pub instance_id: &'a InstanceId,
}

impl Sealed for DeleteAttributeInstance<'_> {}

impl Operation for DeleteAttributeInstance<'_> {
    type Outcome = ();

    /// Reads 204, the only success of Table 6.22.3.2-1.
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for DeleteAttributeInstance<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(
            Method::DELETE,
            ResourcePath::temporal_instance(self.id, self.name, self.instance_id),
            QueryString::default(),
        )
        .into_request(endpoint, scope)
    }
}
