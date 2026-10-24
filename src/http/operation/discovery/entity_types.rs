//! Retrieve Available Entity Types (NGSI-LD clauses 5.7.5 and 6.25.3.1).

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
    model::discovery::EntityTypeList,
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// Lists the entity types the broker holds.
///
/// Its request is `GET /types` (NGSI-LD clause 6.25.3.1), answered with an [`EntityTypeList`] of the entity types the
/// broker holds instances of.
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{Operation, RetrieveEntityTypes, SummaryDetails},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let request = RetrieveEntityTypes::builder()
///     .details(SummaryDetails::Explicit)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/types?details=false"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct RetrieveEntityTypes {
    /// Whether `details=false` is stated or left out (Table 6.25.3.1-1).
    #[builder(default)]
    pub details: SummaryDetails,
    /// Whether only the broker's own entities are considered (`local`, clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
}

impl Sealed for RetrieveEntityTypes {}

impl Operation for RetrieveEntityTypes {
    type Outcome = EntityTypeList;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<EntityTypeList, ResponseError> {
        DocumentReader::document(EntityTypeList::from_json).decode_one(response)
    }
}

impl BuildsRequest for RetrieveEntityTypes {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(ResourcePath::types(), Details::Summary(self.details), self.locality, endpoint, scope)
    }
}
