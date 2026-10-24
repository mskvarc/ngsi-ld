//! Retrieve Details of Available Attributes (NGSI-LD clauses 5.7.9 and 6.27.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            discovery::request,
            parameters::Details,
            sealed::{BuildsRequest, Sealed},
            stored_document::DocumentReader,
        },
        path::ResourcePath,
        request::scope::RequestScope,
    },
    model::discovery::AttributeDetails,
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// Lists every attribute the broker holds, with the entity types that have it.
///
/// Its request is `GET /attributes?details=true` (NGSI-LD clause 6.27.3.1), answered with a list of
/// [`AttributeDetails`], one for each attribute the broker holds instances of.
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{Operation, RetrieveAttributeDetails},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let request = RetrieveAttributeDetails::builder()
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/attributes?details=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct RetrieveAttributeDetails {
    /// Whether only the broker's own entities are considered (`local`, clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
}

impl Sealed for RetrieveAttributeDetails {}

impl Operation for RetrieveAttributeDetails {
    type Outcome = Vec<AttributeDetails>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Vec<AttributeDetails>, ResponseError> {
        DocumentReader::document(AttributeDetails::from_json).decode_list(response)
    }
}

impl BuildsRequest for RetrieveAttributeDetails {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(ResourcePath::available_attributes(), Details::Detailed, self.locality, endpoint, scope)
    }
}
