//! Where the calls of a resource handle go: the client, and the snapshot they are aimed at, if any.

use crate::{
    client::{Answering, Client, ClientError, OutcomeOnly, Transport},
    http::operation::{LocalScopeOperation, OnSnapshot, Operation, SnapshotOperation, WithLocality},
    model::Tenant,
    query::Locality,
    snapshot::SnapshotId,
};

/// The client a handle sends its calls through, and the snapshot it aims them at (NGSI-LD clause 6.3.22): none for
/// the handles of [`Client`], the snapshot of a [`SnapshotView`](crate::client::SnapshotView) for its
/// handles, which are only those of the Core and Temporal APIs (clause 5.5.15).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Route<'c, T, M = OutcomeOnly> {
    /// The client.
    client: &'c Client<T, M>,
    /// The snapshot every call is aimed at.
    snapshot: Option<&'c SnapshotId>,
}

impl<'c, T, M> Route<'c, T, M> {
    /// The calls of `client`, aimed at no snapshot.
    pub(crate) const fn direct(client: &'c Client<T, M>) -> Route<'c, T, M> {
        Route { client, snapshot: None }
    }

    /// The calls of `client`, aimed at `snapshot`.
    pub(crate) const fn on_snapshot(client: &'c Client<T, M>, snapshot: &'c SnapshotId) -> Route<'c, T, M> {
        Route {
            client,
            snapshot: Some(snapshot),
        }
    }
}

impl<T: Transport, M: Answering> Route<'_, T, M> {
    /// Sends `operation` on `tenant`, or on the client's tenant, aimed at the route's snapshot.
    pub(crate) async fn send<O: Operation + Sync>(&self, operation: &O, tenant: Option<&Tenant>) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        self.client.send_on(operation, tenant, self.snapshot).await
    }

    /// Sends `operation` with the `local` parameter of clause 6.3.18, as [`send`](Self::send) does.
    pub(crate) async fn send_with_locality<O: LocalScopeOperation + Sync>(
        &self,
        locality: Locality,
        operation: &O,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        self.send(&WithLocality::new(locality, operation), tenant).await
    }

    /// Sends a query or a purge, as [`send`](Self::send) does, except that on a snapshot its request is the one it
    /// sends there, which needs no restriction of the entities (clause 5.5.15).
    pub(crate) async fn send_selection<O: SnapshotOperation + Sync>(
        &self,
        operation: &O,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        match self.snapshot {
            Some(snapshot) => self.client.send_on_tenant(&OnSnapshot::new(snapshot, operation), tenant).await,
            None => self.client.send_on_tenant(operation, tenant).await,
        }
    }
}
