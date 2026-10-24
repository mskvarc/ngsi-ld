//! Retrieve Available Entity Type Information (NGSI-LD clauses 5.7.7 and 6.26.3.1).

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
    model::{EntityTypeName, discovery::EntityTypeInfo},
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// `GET /types/{type}`: what the broker holds of one entity type (NGSI-LD clause 6.26.3.1), as an
/// [`EntityTypeInfo`]. An unknown type answers 404 `ResourceNotFound`.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveEntityTypeInfo},
///     },
///     model::EntityTypeName,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicle: EntityTypeName = "Vehicle".parse()?;
/// let request = RetrieveEntityTypeInfo::builder()
///     .entity_type(&vehicle)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/types/Vehicle");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bon::Builder)]
pub struct RetrieveEntityTypeInfo<'a> {
    /// The entity type: its short name, which the client's context expands, or its Fully Qualified Name (Table
    /// 6.26.2-1).
    pub entity_type: &'a EntityTypeName,
    /// Whether only the broker's own entities are considered (`local`, clause 6.3.18).
    #[builder(default)]
    pub locality: Locality,
}

impl Sealed for RetrieveEntityTypeInfo<'_> {}

impl Operation for RetrieveEntityTypeInfo<'_> {
    type Outcome = EntityTypeInfo;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<EntityTypeInfo, ResponseError> {
        DocumentReader::document(EntityTypeInfo::from_json).decode_one(response)
    }
}

impl BuildsRequest for RetrieveEntityTypeInfo<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(
            ResourcePath::entity_type(self.entity_type),
            Details::Summary(SummaryDetails::Implied),
            self.locality,
            endpoint,
            scope,
        )
    }
}
