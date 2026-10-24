//! Aiming an operation at a snapshot (NGSI-LD clauses 5.5.15 and 6.3.22): the operations of the Core API and the
//! Temporal API run on the information of a snapshot when their request carries the `NGSILD-Snapshot` header.

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            AddTemporalAttributes,
            AppendAttributes,
            BatchCreate,
            BatchDelete,
            BatchMerge,
            BatchUpdate,
            BatchUpsert,
            CreateEntity,
            CreateOrUpdateSimplifiedTemporalEvolution,
            CreateOrUpdateTemporalEvolution,
            CreateSubscription,
            DeleteAttribute,
            DeleteAttributeInstance,
            DeleteEntity,
            DeleteSubscription,
            DeleteTemporalAttribute,
            DeleteTemporalEvolution,
            MergeEntity,
            MergeSimplifiedEntity,
            ModifyAttributeInstance,
            Operation,
            PartialAttributeUpdate,
            QuerySubscriptions,
            ReplaceAttribute,
            ReplaceEntity,
            RetrieveAttributeDetails,
            RetrieveAttributeInfo,
            RetrieveAttributes,
            RetrieveEntity,
            RetrieveEntityTypeDetails,
            RetrieveEntityTypeInfo,
            RetrieveEntityTypes,
            RetrieveSubscription,
            RetrieveTemporalEvolution,
            UpdateAttributes,
            UpdateSubscription,
            sealed::{BuildsRequest, Sealed},
        },
        request::scope::RequestScope,
    },
    model::{Completeness, Tenant},
    snapshot::SnapshotId,
    subscription::EntitySubscription,
};
use bytes::Bytes;
use http::{Request, Response};

/// An operation that can be aimed at a snapshot.
///
/// It is an operation of the Core API or the Temporal API (NGSI-LD clause 5.5.15, Table 4.3.5-1), that is of Context
/// Information Provision and Consumption (clauses 5.6 and 5.7.1 to 5.7.10, Purge Entities of clause 5.6.21 included,
/// which the table does not list), of Context Information Subscription (clause 5.8), and of the temporal evolution of
/// entities (clauses 5.6.11 to 5.6.16, 5.7.3 and 5.7.4).
///
/// The operations of the Registry, Snapshot and `JSONLDContext` APIs, and the support operations of distributed
/// deployments (entity maps, source identity), do not implement it, so [`OnSnapshot`] cannot wrap them. Like
/// [`Operation`], the trait is implemented by the operations of this crate only.
pub trait SnapshotOperation: Operation {
    /// Builds the request the operation sends when it is aimed at a snapshot, before the `NGSILD-Snapshot` header is
    /// added. It is the operation's own request, except for Query Entities, Purge Entities and Query Temporal
    /// Evolution of Entities: an operation on a snapshot is local by itself (clauses 5.5.15 and 6.3.18), so they need
    /// no type, `attrs`, `q` or geoquery, and send no `local=true` for it.
    ///
    /// # Errors
    ///
    /// Returns [`RequestError`] when the parameters are a combination the specification refuses.
    fn snapshot_request(&self, endpoint: &Endpoint, tenant: Option<&Tenant>) -> Result<Request<Bytes>, RequestError> {
        self.scoped_snapshot_request(endpoint, RequestScope::on_tenant(tenant))
    }
}

impl SnapshotOperation for CreateEntity<'_> {}
impl<F, J: Default, C> SnapshotOperation for RetrieveEntity<'_, F, J, C> where Self: Operation {}
impl SnapshotOperation for ReplaceEntity<'_> {}
impl SnapshotOperation for DeleteEntity<'_> {}
impl<C: Completeness> SnapshotOperation for MergeEntity<'_, C> where Self: Operation {}
impl SnapshotOperation for MergeSimplifiedEntity<'_> {}
impl<C: Completeness> SnapshotOperation for UpdateAttributes<'_, C> where Self: Operation {}
impl<C: Completeness> SnapshotOperation for AppendAttributes<'_, C> where Self: Operation {}
impl SnapshotOperation for PartialAttributeUpdate<'_> {}
impl SnapshotOperation for ReplaceAttribute<'_> {}
impl SnapshotOperation for DeleteAttribute<'_> {}
impl SnapshotOperation for BatchCreate<'_> {}
impl SnapshotOperation for BatchUpsert<'_> {}
impl SnapshotOperation for BatchUpdate<'_> {}
impl SnapshotOperation for BatchMerge<'_> {}
impl SnapshotOperation for BatchDelete<'_> {}
impl SnapshotOperation for RetrieveEntityTypes {}
impl SnapshotOperation for RetrieveEntityTypeDetails {}
impl SnapshotOperation for RetrieveEntityTypeInfo<'_> {}
impl SnapshotOperation for RetrieveAttributes {}
impl SnapshotOperation for RetrieveAttributeDetails {}
impl SnapshotOperation for RetrieveAttributeInfo<'_> {}
impl SnapshotOperation for CreateSubscription<'_, EntitySubscription> {}
impl SnapshotOperation for UpdateSubscription<'_, EntitySubscription> {}
impl SnapshotOperation for RetrieveSubscription<'_, EntitySubscription> {}
impl SnapshotOperation for QuerySubscriptions<EntitySubscription> {}
impl SnapshotOperation for DeleteSubscription<'_, EntitySubscription> {}
impl SnapshotOperation for CreateOrUpdateTemporalEvolution<'_> {}
impl SnapshotOperation for CreateOrUpdateSimplifiedTemporalEvolution<'_> {}
impl SnapshotOperation for AddTemporalAttributes<'_> {}
impl SnapshotOperation for DeleteTemporalAttribute<'_> {}
impl SnapshotOperation for ModifyAttributeInstance<'_> {}
impl SnapshotOperation for DeleteAttributeInstance<'_> {}
impl SnapshotOperation for DeleteTemporalEvolution<'_> {}
impl<F, C> SnapshotOperation for RetrieveTemporalEvolution<'_, F, C> where Self: Operation {}

/// An operation aimed at a snapshot.
///
/// Its request is the operation's own, with the `NGSILD-Snapshot` header naming the snapshot, and its outcome is the
/// operation's (NGSI-LD clauses 5.5.15 and 6.3.22).
///
/// The operation runs on the information of the snapshot only, and locally (clause 5.5.15), so no `local` is added, and
/// a query or a purge needs no restriction (see [`SnapshotOperation::snapshot_request`]). The tenant is orthogonal: an
/// operation on a snapshot of a tenant names both. A page of a query on a snapshot remembers it, so its following pages
/// are aimed at it too.
///
/// ```
/// use headers::Header;
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         headers::NgsiLdSnapshot,
///         operation::{DeleteEntity, OnSnapshot, Operation},
///     },
///     model::EntityId,
///     snapshot::SnapshotId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let delete = DeleteEntity::builder().id(&id).build();
/// let snapshot = SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"));
/// let request = OnSnapshot::new(&snapshot, &delete).to_request(&endpoint)?;
/// assert_eq!(
///     request.headers()[NgsiLdSnapshot::name()],
///     "urn:ngsi-ld:Snapshot:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnSnapshot<'a, O> {
    /// The snapshot.
    pub snapshot: &'a SnapshotId,
    /// The operation.
    pub operation: &'a O,
}

impl<'a, O: SnapshotOperation> OnSnapshot<'a, O> {
    /// `operation`, aimed at `snapshot`.
    #[must_use]
    pub const fn new(snapshot: &'a SnapshotId, operation: &'a O) -> OnSnapshot<'a, O> {
        OnSnapshot { snapshot, operation }
    }
}

impl<O: SnapshotOperation> Sealed for OnSnapshot<'_, O> {}

impl<O: SnapshotOperation> BuildsRequest for OnSnapshot<'_, O> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.operation.scoped_snapshot_request(endpoint, scope.on_snapshot(Some(self.snapshot)))
    }
}

impl<O: SnapshotOperation> Operation for OnSnapshot<'_, O> {
    type Outcome = O::Outcome;

    fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<O::Outcome, ResponseError> {
        self.operation.decode(context, response)
    }
}
