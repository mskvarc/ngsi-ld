//! Retrieve Snapshot Status (NGSI-LD clauses 5.16.3 and 6.37.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
            snapshot::READER,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{NgsiLdMediaType, Output},
    snapshot::{Snapshot, SnapshotId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Retrieves a snapshot and its status (NGSI-LD clause 5.16.3): `GET /snapshots/{snapshotId}` (clause 6.37.3.1),
/// answered 200 with the snapshot.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RetrieveSnapshot},
///     },
///     snapshot::SnapshotId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"));
/// let request = RetrieveSnapshot { id: &id }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrieveSnapshot<'a> {
    /// The snapshot.
    pub id: &'a SnapshotId,
}

impl Sealed for RetrieveSnapshot<'_> {}

impl Operation for RetrieveSnapshot<'_> {
    type Outcome = Snapshot<Output>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Snapshot<Output>, ResponseError> {
        READER.decode_one(response)
    }
}

impl BuildsRequest for RetrieveSnapshot<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::GET, ResourcePath::snapshot(self.id), QueryString::default())
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
