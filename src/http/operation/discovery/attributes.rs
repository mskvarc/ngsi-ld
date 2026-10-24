//! Retrieve Available Attributes (NGSI-LD clauses 5.7.8 and 6.27.3.1).

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
    model::discovery::AttributeList,
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// `GET /attributes`: the attributes the broker holds instances of (NGSI-LD clause 6.27.3.1), as an
/// [`AttributeList`].
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{Operation, RetrieveAttributes},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let request = RetrieveAttributes::builder()
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/attributes");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct RetrieveAttributes {
    /// Whether `details=false` is stated or left out (Table 6.27.3.1-1).
    #[builder(default)]
    pub details: SummaryDetails,
    /// Whether only the broker's own entities are considered (`local`, clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
}

impl Sealed for RetrieveAttributes {}

impl Operation for RetrieveAttributes {
    type Outcome = AttributeList;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<AttributeList, ResponseError> {
        DocumentReader::document(AttributeList::from_json).decode_one(response)
    }
}

impl BuildsRequest for RetrieveAttributes {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(
            ResourcePath::available_attributes(),
            Details::Summary(self.details),
            self.locality,
            endpoint,
            scope,
        )
    }
}
