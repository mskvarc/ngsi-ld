//! Partial Attribute Update (NGSI-LD clauses 5.6.4 and 6.7.3.1).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        UpdateOutcome,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{Attribute, AttributeName, Change, EntityId, InstanceDeletion, Lossless, Patch, representation::write::write_attribute_body},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Changes some members of one attribute.
///
/// Its request is `PATCH /entities/{entityId}/attrs/{attrId}` (NGSI-LD clause 6.7.3.1). The instance is the one the
/// fragment's `datasetId` names, or the default instance.
///
/// The body may also be an [`InstanceDeletion`]: clauses 4.5.2.2 and 4.5.3.2 let the NGSI-LD Null stand as the payload
/// of a partial update patch to delete the attribute, although clause 5.6.4.1 says a partial update does not delete
/// it. The request carries the Null as clauses 4.5.2.2 and 4.5.3.2 write it, and the broker's answer says which rule it
/// applied.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, PartialAttributeUpdate},
///     },
///     model::{Attribute, AttributeName, Change, EntityId, Property},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let speed: AttributeName = "speed".parse()?;
/// let faster = Change::Set(Attribute::from(Property::builder(55).build()?).into_patch());
/// let update = PartialAttributeUpdate::builder()
///     .id(&id)
///     .name(&speed)
///     .attribute(&faster)
///     .build();
/// let request = update.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct PartialAttributeUpdate<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The members to change, any of which, sub-attributes included, the fragment may delete with the NGSI-LD Null
    /// (clause 5.5.8); or the deletion of the instance, whose payload is the Null (clauses 4.5.2.2 and 4.5.3.2).
    pub attribute: &'a Change<Attribute<Patch>, InstanceDeletion>,
    /// Updates only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for PartialAttributeUpdate<'_> {}

impl Operation for PartialAttributeUpdate<'_> {
    type Outcome = UpdateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<UpdateOutcome, ResponseError> {
        UpdateOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for PartialAttributeUpdate<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::PATCH, ResourcePath::attribute(self.id, self.name), query)
            .with_body(write_attribute_body(self.attribute, self.form))
            .into_request(endpoint, scope)
    }
}
