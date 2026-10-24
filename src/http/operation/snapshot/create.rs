//! Create Snapshot (NGSI-LD clauses 5.16.1 and 6.36.3.1).

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
    model::Input,
    snapshot::{Snapshot, SnapshotId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Creates a snapshot (NGSI-LD clause 5.16.1): `POST /snapshots` with the snapshot as the body (clause 6.36.3.1).
///
/// The broker answers 201 with the location of the snapshot, whose last path segment is its id, generated when the
/// snapshot names none (clause 5.16.1.4); the snapshot is `preparing` until its queries have run.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateSnapshot, Operation},
///     },
///     snapshot::{QueryDocument, Snapshot},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles = QueryDocument::builder().entities(["Vehicle"]).build()?;
/// let snapshot = Snapshot::builder().query(vehicles).build()?;
/// let request = CreateSnapshot {
///     snapshot: &snapshot,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/snapshots");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateSnapshot<'a> {
    /// The snapshot to create.
    pub snapshot: &'a Snapshot<Input>,
}

impl Sealed for CreateSnapshot<'_> {}

impl Operation for CreateSnapshot<'_> {
    type Outcome = ResourceCreated<SnapshotId>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ResourceCreated<SnapshotId>, ResponseError> {
        ResourceCreated::decode(&classify(response)?)
    }
}

impl BuildsRequest for CreateSnapshot<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::snapshots(), QueryString::default())
            .with_body(self.snapshot.to_object())
            .into_request(endpoint, scope)
    }
}
