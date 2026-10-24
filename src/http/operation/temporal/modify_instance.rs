//! Modify Attribute Instance in Temporal Evolution of an Entity (NGSI-LD clauses 5.6.14 and 6.22.3.1).

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
    model::{Attribute, AttributeName, EntityId, Input, InstanceId, Lossless, representation::write::write_attribute_object},
};
use bytes::Bytes;
use http::{Method, Request, Response};
use jstrict::{Object, Value as Json};

/// `PATCH /temporal/entities/{entityId}/attrs/{attrId}/{instanceId}`: replaces one instance of an attribute in the
/// temporal evolution of an entity (NGSI-LD clause 6.22.3.1).
///
/// Clause 5.6.14.3 makes the body a fragment holding the new instance "contained by an Array of exactly one
/// item", so it is written `{"<attrId>": [<instance>]}`, the attribute named by the path; taking one attribute
/// makes any other number of items impossible.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{http::{Endpoint, operation::{Operation, ModifyAttributeInstance}}, model::{Attribute, Property, EntityId, InstanceId, AttributeName}};
/// use url::Url;
///
/// let endpoint = Endpoint::builder().base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?).build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let speed: AttributeName = "speed".parse()?;
/// let instance = InstanceId::from(iri!("urn:ngsi-ld:Instance:1"));
/// let corrected = Attribute::from(Property::builder(110).observed_at("2018-08-01T12:03:00Z".parse()?).build()?);
/// let modify = ModifyAttributeInstance::builder().id(&id).name(&speed).instance_id(&instance).instance(&corrected).build();
/// let request = modify.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211/attrs/speed/urn:ngsi-ld:Instance:1");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct ModifyAttributeInstance<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The instance replaced (`instanceId`, clause 4.5.7).
    pub instance_id: &'a InstanceId,
    /// The new instance.
    pub instance: &'a Attribute<Input>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for ModifyAttributeInstance<'_> {}

impl Operation for ModifyAttributeInstance<'_> {
    type Outcome = ();

    /// Reads 204, the only success of Table 6.22.3.1-1.
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for ModifyAttributeInstance<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut body = Object::new();
        let instance = write_attribute_object(self.instance, self.form);
        body.push(self.name.as_ref().into(), Json::Array(vec![Json::Object(instance)].into()));
        RequestParts::resource(
            Method::PATCH,
            ResourcePath::temporal_instance(self.id, self.name, self.instance_id),
            QueryString::default(),
        )
        .with_body(body)
        .into_request(endpoint, scope)
    }
}
