//! Purge Snapshots (NGSI-LD clauses 5.16.7 and 6.36.3.2).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    query::q::Query,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Purges the snapshots a query selects.
///
/// NGSI-LD clause 5.16.7 defines the operation. Its request is `DELETE /snapshots?q=...` (clause 6.36.3.2).
///
/// The query is mandatory (Table 6.36.3.2-1) and restricted to the members of the Snapshot data type (clause
/// 5.16.7.4), which the broker checks. The broker answers 204, or 207 with the snapshots it could not delete.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, PurgeSnapshots},
///     },
///     query::q::Query,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let low_priority: Query = "snapshotPriority<3".parse()?;
/// let request = PurgeSnapshots { q: &low_priority }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/snapshots?q=snapshotPriority%3C3"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurgeSnapshots<'a> {
    /// The query the snapshots to purge match (`q`).
    pub q: &'a Query,
}

impl Sealed for PurgeSnapshots<'_> {}

impl Operation for PurgeSnapshots<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for PurgeSnapshots<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push(QueryParameter::Q, self.q);
        RequestParts::resource(Method::DELETE, ResourcePath::snapshots(), query).into_request(endpoint, scope)
    }
}
