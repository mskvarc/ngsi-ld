//! Batch Entity Delete (NGSI-LD clauses 5.6.10 and 6.17.3.1).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            batch::{non_empty, post},
            sealed::{BuildsRequest, Sealed},
        },
        path::BatchResource,
        query_string::QueryString,
        request::scope::RequestScope,
        response::classify,
    },
    model::EntityId,
};
use bytes::Bytes;
use http::{Request, Response};

/// Deletes every entity the batch names.
///
/// Its request is `POST /entityOperations/delete` (NGSI-LD clause 6.17.3.1). Of several occurrences of one id, the
/// first deletes the entity and every later one is reported as not found (clause 5.5.11.4).
///
/// The body is an array of ids, which has no object to hold an `@context`, so it is always sent as
/// `application/json` with the `Link` header (clause 6.3.5).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{BatchDelete, Operation},
///     },
///     model::EntityId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let ids = [
///     EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")),
///     EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211")),
/// ];
/// let request = BatchDelete { ids: &ids }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityOperations/delete"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchDelete<'a> {
    /// The ids of the entities, in the order the broker processes them.
    pub ids: &'a [EntityId],
}

impl Sealed for BatchDelete<'_> {}

impl Operation for BatchDelete<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for BatchDelete<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        post(BatchResource::Delete, QueryString::default())
            .with_entity_ids(non_empty(self.ids)?)
            .into_request(endpoint, scope)
    }
}
