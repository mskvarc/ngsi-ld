//! Clone Snapshot (NGSI-LD clauses 5.16.2 and 6.38.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResourceCreated,
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
    snapshot::{SnapshotClone, SnapshotId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Clones a snapshot (NGSI-LD clause 5.16.2): `POST /snapshots/{snapshotId}/clone` with the [`SnapshotClone`] as the
/// body (clause 6.38.3.1).
///
/// The broker answers 201 with the location of the clone, whose last path segment is the clone's id.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CloneSnapshot, Operation},
///     },
///     snapshot::{SnapshotClone, SnapshotId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"));
/// let copy = SnapshotClone::builder()
///     .id(iri!("urn:ngsi-ld:Snapshot:2"))
///     .lifetime("P1D".parse()?)
///     .build()?;
/// let request = CloneSnapshot {
///     id: &id,
///     clone: &copy,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1/clone"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloneSnapshot<'a> {
    /// The snapshot to clone.
    pub id: &'a SnapshotId,
    /// The id and settings of the clone.
    pub clone: &'a SnapshotClone,
}

impl Sealed for CloneSnapshot<'_> {}

impl Operation for CloneSnapshot<'_> {
    type Outcome = ResourceCreated<SnapshotId>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ResourceCreated<SnapshotId>, ResponseError> {
        ResourceCreated::decode(&classify(response)?)
    }
}

impl BuildsRequest for CloneSnapshot<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::snapshot_clone(self.id), QueryString::default())
            .with_body(self.clone.to_object())
            .into_request(endpoint, scope)
    }
}
