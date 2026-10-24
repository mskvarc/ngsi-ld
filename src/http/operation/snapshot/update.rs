//! Update Snapshot Status (NGSI-LD clauses 5.16.4 and 6.37.3.2).

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
    snapshot::{Snapshot, SnapshotFragment, SnapshotId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Updates a snapshot (NGSI-LD clause 5.16.4): `PATCH /snapshots/{snapshotId}` with the [`SnapshotFragment`] as the
/// body (clause 6.37.3.2), answered 200 with the updated snapshot, so the request asks for `application/json`.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, UpdateSnapshot},
///     },
///     snapshot::{SnapshotFragment, SnapshotId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"));
/// let longer = SnapshotFragment::builder()
///     .lifetime("PT2H".parse()?)
///     .build()?;
/// let request = UpdateSnapshot {
///     id: &id,
///     fragment: &longer,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateSnapshot<'a> {
    /// The snapshot.
    pub id: &'a SnapshotId,
    /// The changes.
    pub fragment: &'a SnapshotFragment,
}

impl Sealed for UpdateSnapshot<'_> {}

impl Operation for UpdateSnapshot<'_> {
    type Outcome = Snapshot<Output>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Snapshot<Output>, ResponseError> {
        READER.decode_one(response)
    }
}

impl BuildsRequest for UpdateSnapshot<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::PATCH, ResourcePath::snapshot(self.id), QueryString::default())
            .with_body(self.fragment.to_object())
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
