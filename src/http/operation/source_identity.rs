//! Retrieve Context Source Identity Information (NGSI-LD clauses 5.15.1 and 6.33.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
            stored_document::DocumentReader,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{ContextSourceIdentity, NgsiLdMediaType},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `GET /info/sourceIdentity`: the identity of the broker, or of the tenant the request names (NGSI-LD clauses
/// 5.15.1.1 and 6.33.3.1).
///
/// A broker that cannot supply it answers 501 with the error type `NotImplemented` (clause 5.15.1.4), which Table
/// 5.5.2-1 does not list, so the problem's type is read as
/// [`ErrorType::Other`](crate::model::ErrorType::Other).
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{Operation, RetrieveSourceIdentity},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let request = RetrieveSourceIdentity.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/info/sourceIdentity"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RetrieveSourceIdentity;

impl Sealed for RetrieveSourceIdentity {}

impl Operation for RetrieveSourceIdentity {
    type Outcome = ContextSourceIdentity;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ContextSourceIdentity, ResponseError> {
        DocumentReader::document(ContextSourceIdentity::from_json).decode_one(response)
    }
}

impl BuildsRequest for RetrieveSourceIdentity {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::GET, ResourcePath::source_identity(), QueryString::default())
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
