//! Delete Attribute from Temporal Evolution of an Entity (NGSI-LD clauses 5.6.13 and 6.21.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            InstanceSelection,
            Operation,
            no_content::decode_no_content,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{AttributeName, EntityId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Deletes an attribute from the temporal evolution of an entity.
///
/// Its request is `DELETE /temporal/entities/{entityId}/attrs/{attrId}` (NGSI-LD clause 6.21.3.1). It deletes every
/// instance over time of the default instance, of one dataset, or of all of them; the clause's `datasetId` and
/// `deleteAll` select the instances as for Delete Attribute.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteTemporalAttribute, Operation},
///     },
///     model::{AttributeName, EntityId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let speed: AttributeName = "speed".parse()?;
/// let request = DeleteTemporalAttribute::builder()
///     .id(&id)
///     .name(&speed)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211/attrs/speed"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, bon::Builder)]
pub struct DeleteTemporalAttribute<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The instances to delete.
    #[builder(default)]
    pub instances: InstanceSelection<'a>,
}

impl Sealed for DeleteTemporalAttribute<'_> {}

impl Operation for DeleteTemporalAttribute<'_> {
    type Outcome = ();

    /// Reads 204, the only success of Table 6.21.3.1-2.
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for DeleteTemporalAttribute<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.instances.push_query(&mut query);
        RequestParts::resource(Method::DELETE, ResourcePath::temporal_attribute(self.id, self.name), query).into_request(endpoint, scope)
    }
}
