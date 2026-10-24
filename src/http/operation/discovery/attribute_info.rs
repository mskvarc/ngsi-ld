//! Retrieve Available Attribute Information (NGSI-LD clauses 5.7.10 and 6.28.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            SummaryDetails,
            discovery::request,
            parameters::Details,
            sealed::{BuildsRequest, Sealed},
            stored_document::DocumentReader,
        },
        path::ResourcePath,
        request::scope::RequestScope,
    },
    model::{AttributeName, discovery::AttributeInfo},
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// Reads what the broker holds of one attribute.
///
/// Its request is `GET /attributes/{attrId}` (NGSI-LD clause 6.28.3.1), answered with an [`AttributeInfo`]. An unknown
/// attribute answers 404 `ResourceNotFound`.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveAttributeInfo},
///     },
///     model::AttributeName,
///     query::Locality,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let speed: AttributeName = "speed".parse()?;
/// let request = RetrieveAttributeInfo::builder()
///     .attribute(&speed)
///     .locality(Locality::Local)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/attributes/speed?local=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bon::Builder)]
pub struct RetrieveAttributeInfo<'a> {
    /// The attribute: its short name, which the client's context expands, or its Fully Qualified Name (Table
    /// 6.28.2-1).
    pub attribute: &'a AttributeName,
    /// Whether only the broker's own entities are considered (`local`, clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
}

impl Sealed for RetrieveAttributeInfo<'_> {}

impl Operation for RetrieveAttributeInfo<'_> {
    type Outcome = AttributeInfo;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<AttributeInfo, ResponseError> {
        DocumentReader::document(AttributeInfo::from_json).decode_one(response)
    }
}

impl BuildsRequest for RetrieveAttributeInfo<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(
            ResourcePath::available_attribute(self.attribute),
            Details::Summary(SummaryDetails::Implied),
            self.locality,
            endpoint,
            scope,
        )
    }
}
