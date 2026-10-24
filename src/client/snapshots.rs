//! The snapshot operations (NGSI-LD clause 5.16), as builders that finish with `send()`.

use crate::{
    client::{Answering, Client, ClientError, OutcomeOnly, Transport, arguments::build},
    http::{
        BatchOutcome,
        ResourceCreated,
        operation::{CloneSnapshot, CreateSnapshot, DeleteSnapshot, PurgeSnapshots, RetrieveSnapshot, UpdateSnapshot},
    },
    model::{Input, Output, Tenant},
    query::q::{Query, builders::QueryBuilder},
    snapshot::{Snapshot, SnapshotClone, SnapshotFragment, SnapshotId},
};
use bon::bon;

/// The snapshot operations of a [`Client`].
///
/// NGSI-LD clauses 6.36 to 6.38.
///
/// Each operation is a builder that finishes with `send()`; operations of the Core and Temporal APIs are aimed at a
/// snapshot through [`Client::on_snapshot`].
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response, StatusCode, header::LOCATION};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response = Response::new(Bytes::new());
/// #         *response.status_mut() = StatusCode::CREATED;
/// #         response.headers_mut().insert(
/// #             LOCATION,
/// #             http::HeaderValue::from_static(
/// #                 "/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1",
/// #             ),
/// #         );
/// #         Ok(response)
/// #     }
/// # }
/// use anyhow::Context;
/// use ngsi_ld::{
///     client::Client,
///     snapshot::{QueryDocument, Snapshot},
/// };
///
/// async fn freeze_vehicles(client: &Client<Offline>) -> anyhow::Result<()> {
///     let vehicles = QueryDocument::builder().entities(["Vehicle"]).build()?;
///     let snapshot = Snapshot::builder().query(vehicles).build()?;
///     let created = client.snapshots().create(&snapshot).send().await?;
///     let id = created.id().context("the broker named no snapshot")?;
///     assert_eq!(id.as_str(), "urn:ngsi-ld:Snapshot:1");
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Snapshots<'c, T, M = OutcomeOnly> {
    /// The client the calls go through.
    client: &'c Client<T, M>,
}

impl<'c, T, M> Snapshots<'c, T, M> {
    /// The handle of `client`.
    pub(crate) const fn new(client: &'c Client<T, M>) -> Self {
        Self { client }
    }
}

#[bon]
impl<T: Transport, M: Answering> Snapshots<'_, T, M> {
    /// Create Snapshot (clause 6.36.3.1); the broker names a snapshot sent without an `id` (clause 5.16.1.4), which
    /// [`ResourceCreated::id`] reads. The snapshot is `preparing` until its queries have run.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the snapshot (`AlreadyExists`, `BadRequestData`) or the call
    /// fails.
    #[builder(finish_fn = send)]
    pub async fn create(
        &self,
        #[builder(start_fn)] snapshot: &Snapshot<Input>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<ResourceCreated<SnapshotId>>, ClientError<T::Error>> {
        self.client.send_on_tenant(&CreateSnapshot { snapshot }, tenant).await
    }

    /// Clone Snapshot (clause 6.38.3.1): copies the snapshot `id` into a new one, named and set up by `clone`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the snapshot does not exist, the clone's id is taken, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn clone_snapshot(
        &self,
        #[builder(start_fn)] id: &SnapshotId,
        #[builder(start_fn)] clone: &SnapshotClone,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<ResourceCreated<SnapshotId>>, ClientError<T::Error>> {
        self.client.send_on_tenant(&CloneSnapshot { id, clone }, tenant).await
    }

    /// Retrieve Snapshot Status (clause 6.37.3.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the snapshot does not exist, the answer is not a snapshot, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve(&self, #[builder(start_fn)] id: &SnapshotId, tenant: Option<&Tenant>) -> Result<M::Answer<Snapshot<Output>>, ClientError<T::Error>> {
        self.client.send_on_tenant(&RetrieveSnapshot { id }, tenant).await
    }

    /// Update Snapshot Status (clause 6.37.3.2): applies a fragment to the snapshot and returns it updated.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the snapshot does not exist, the broker refuses the fragment, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn update(
        &self,
        #[builder(start_fn)] id: &SnapshotId,
        #[builder(start_fn)] fragment: &SnapshotFragment,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Snapshot<Output>>, ClientError<T::Error>> {
        self.client.send_on_tenant(&UpdateSnapshot { id, fragment }, tenant).await
    }

    /// Delete Snapshot (clause 6.37.3.3).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the snapshot does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(&self, #[builder(start_fn)] id: &SnapshotId, tenant: Option<&Tenant>) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.client.send_on_tenant(&DeleteSnapshot { id }, tenant).await
    }

    /// Purge Snapshots (clause 6.36.3.2): deletes the snapshots `q` selects, a query of the members of the
    /// Snapshot data type (clause 5.16.7.4).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when `q` is invalid, the broker refuses it or finds no snapshot, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn purge(&self, #[builder(start_fn, into)] q: QueryBuilder, tenant: Option<&Tenant>) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let q: Query = build(q)?;
        self.client.send_on_tenant(&PurgeSnapshots { q: &q }, tenant).await
    }
}
