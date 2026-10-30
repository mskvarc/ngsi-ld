//! The handles of a client aimed at a snapshot (NGSI-LD clauses 5.5.15 and 6.3.22).

use crate::{
    client::{
        Answering,
        AvailableAttributes,
        AvailableEntityTypes,
        Client,
        ClientError,
        Entities,
        OutcomeOnly,
        Subscriptions,
        Temporal,
        Transport,
        route::Route,
    },
    http::{UncheckedRequest, operation::Operation},
    snapshot::SnapshotId,
    subscription::EntitySubscription,
};

/// The handles of a [`Client`] whose every call is aimed at one snapshot.
///
/// They are the operations of the Core API (entities, entity types and attributes, subscriptions) and of the Temporal
/// API, which are the ones a snapshot supports (NGSI-LD clause 5.5.15, Table 4.3.5-1). The view has no handle of the
/// other APIs, so no call it sends is outside them.
///
/// Each handle is the client's own, sending its requests with the `NGSILD-Snapshot` header (clause 6.3.22). The
/// tenant is chosen as for every call; a query's pages remember the snapshot, so `client.pages(first)` stays on it.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         Ok(Response::new(Bytes::from_static(b"[]")))
/// #     }
/// # }
/// use iri_rs::iri;
/// use ngsi_ld::{client::Client, model::SimplifiedEntity, snapshot::SnapshotId};
///
/// async fn frozen_vehicles(client: &Client<Offline>) -> anyhow::Result<Vec<SimplifiedEntity>> {
///     let snapshot = SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"));
///     let first = client
///         .on_snapshot(&snapshot)
///         .entities()
///         .query_simplified()
///         .entity_type("Vehicle")
///         .send()
///         .await?;
///     Ok(client.pages(first).try_collect().await?)
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SnapshotView<'c, T, M = OutcomeOnly> {
    /// The client.
    client: &'c Client<T, M>,
    /// The snapshot every call is aimed at.
    snapshot: &'c SnapshotId,
}

impl<'c, T, M> SnapshotView<'c, T, M> {
    /// The view of `client` aimed at `snapshot`.
    pub(crate) const fn new(client: &'c Client<T, M>, snapshot: &'c SnapshotId) -> SnapshotView<'c, T, M> {
        SnapshotView { client, snapshot }
    }

    /// Where the calls of the view's handles go.
    const fn route(&self) -> Route<'c, T, M> {
        Route::on_snapshot(self.client, self.snapshot)
    }

    /// The operations on entities and their attributes (NGSI-LD clauses 5.6 and 5.7), on the snapshot.
    #[must_use]
    pub const fn entities(&self) -> Entities<'c, T, M> {
        Entities::new(self.route())
    }

    /// The operations on the temporal evolution of entities, on the snapshot.
    #[must_use]
    pub const fn temporal(&self) -> Temporal<'c, T, M> {
        Temporal::new(self.route())
    }

    /// The entity type discovery operations, on the snapshot.
    #[must_use]
    pub const fn entity_types(&self) -> AvailableEntityTypes<'c, T, M> {
        AvailableEntityTypes::new(self.route())
    }

    /// The attribute discovery operations, on the snapshot.
    #[must_use]
    pub const fn attributes(&self) -> AvailableAttributes<'c, T, M> {
        AvailableAttributes::new(self.route())
    }

    /// The operations on context information subscriptions (NGSI-LD clause 5.8), on the snapshot; clause 6.3.22
    /// sends the notifications of such a subscription with the same header.
    #[must_use]
    pub const fn subscriptions(&self) -> Subscriptions<'c, T, EntitySubscription, M> {
        Subscriptions::new(self.route(), EntitySubscription)
    }
}

impl<T: Transport, M: Answering> SnapshotView<'_, T, M> {
    /// Builds the request `operation` would send on the client's tenant, aimed at the snapshot, for breaking on purpose
    /// (see [`UncheckedRequest`]). The client sends it with [`send_unchecked`](Client::send_unchecked).
    ///
    /// Any operation can be built, including one the view has no handle of, so a negative test can aim at a
    /// snapshot a request that clause 5.5.15 does not allow.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Request`] when the operation itself refuses its parameters.
    pub fn unchecked<O: Operation>(&self, operation: &O) -> Result<UncheckedRequest, ClientError<T::Error>> {
        self.client.unchecked_on(operation, Some(self.snapshot))
    }
}
