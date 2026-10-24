//! Replace Attribute (NGSI-LD clauses 5.6.19 and 6.7.3.3).

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
    model::{Attribute, AttributeName, EntityId, Input, Lossless, representation::write::write_attribute_object},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `PUT /entities/{entityId}/attrs/{attrId}`: replaces one attribute instance with a complete new one (NGSI-LD
/// clause 6.7.3.3).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, ReplaceAttribute},
///     },
///     model::{Attribute, AttributeName, EntityId, Property},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let speed: AttributeName = "speed".parse()?;
/// let slower = Attribute::from(Property::builder(30).build()?);
/// let replace = ReplaceAttribute::builder()
///     .id(&id)
///     .name(&speed)
///     .attribute(&slower)
///     .build();
/// let request = replace.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PUT");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct ReplaceAttribute<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The new attribute instance.
    pub attribute: &'a Attribute<Input>,
    /// Replaces only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for ReplaceAttribute<'_> {}

impl Operation for ReplaceAttribute<'_> {
    type Outcome = UpdateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<UpdateOutcome, ResponseError> {
        UpdateOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for ReplaceAttribute<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::PUT, ResourcePath::attribute(self.id, self.name), query)
            .with_body(write_attribute_object(self.attribute, self.form))
            .into_request(endpoint, scope)
    }
}
