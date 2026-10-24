//! Retrieve Details of Available Entity Types (NGSI-LD clauses 5.7.6 and 6.25.3.1).

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
    model::discovery::EntityTypeDetails,
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// `GET /types?details=true`: every entity type the broker holds instances of, with the names of their
/// attributes (NGSI-LD clause 6.25.3.1), as a list of [`EntityTypeDetails`].
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{Operation, RetrieveEntityTypeDetails},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let request = RetrieveEntityTypeDetails::builder()
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/types?details=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct RetrieveEntityTypeDetails {
    /// Whether only the broker's own entities are considered (`local`, clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
}

impl Sealed for RetrieveEntityTypeDetails {}

impl Operation for RetrieveEntityTypeDetails {
    type Outcome = Vec<EntityTypeDetails>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Vec<EntityTypeDetails>, ResponseError> {
        DocumentReader::document(EntityTypeDetails::from_json).decode_list(response)
    }
}

impl BuildsRequest for RetrieveEntityTypeDetails {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(ResourcePath::types(), Details::Detailed, self.locality, endpoint, scope)
    }
}
