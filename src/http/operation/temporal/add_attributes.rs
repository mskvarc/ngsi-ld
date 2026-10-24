//! Add Attributes to Temporal Evolution of an Entity (NGSI-LD clauses 5.6.12 and 6.20.3.1).

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
    model::{
        EntityId,
        Input,
        Lossless,
        evolution::{TemporalAttributes, write::temporal_attributes_object},
    },
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `POST /temporal/entities/{entityId}/attrs`: adds attribute instances to the temporal evolution of an entity
/// (NGSI-LD clause 6.20.3.1). Each evolution is written as an array, as clause 5.6.12.3 requires.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{AddTemporalAttributes, Operation},
///     },
///     model::{AttributeName, EntityId, Input, Property, evolution::TemporalAttributes},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let speed: AttributeName = "speed".parse()?;
/// let mut instances = TemporalAttributes::<Input>::new();
/// instances.insert(
///     speed,
///     Property::builder(80)
///         .observed_at("2018-08-01T12:05:00Z".parse()?)
///         .build()?
///         .into(),
/// )?;
/// let request = AddTemporalAttributes::builder()
///     .id(&id)
///     .attributes(&instances)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211/attrs"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct AddTemporalAttributes<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The instances to add.
    pub attributes: &'a TemporalAttributes<Input>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for AddTemporalAttributes<'_> {}

impl Operation for AddTemporalAttributes<'_> {
    type Outcome = ();

    /// Reads 204, the only success of Table 6.20.3.1-1.
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for AddTemporalAttributes<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::temporal_attributes(self.id), QueryString::default())
            .with_body(temporal_attributes_object(self.attributes, self.form))
            .into_request(endpoint, scope)
    }
}
