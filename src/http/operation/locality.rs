//! Limiting an operation to the broker's own data (NGSI-LD clause 6.3.18): every operation on `/entities`,
//! `/entityOperations`, `/temporal/entities`, `/temporal/entityOperations`, `/types` and `/attributes` takes the
//! `local` parameter, including those whose own tables do not list it.

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
            DeleteAttribute,
            DeleteAttributeInstance,
            DeleteEntity,
            DeleteTemporalAttribute,
            DeleteTemporalEvolution,
            MergeEntity,
            MergeSimplifiedEntity,
            ModifyAttributeInstance,
            Operation,
            PartialAttributeUpdate,
            ReplaceAttribute,
            ReplaceEntity,
            RetrieveEntity,
            RetrieveTemporalEvolution,
            UpdateAttributes,
            sealed::{BuildsRequest, Sealed},
        },
        request::scope::RequestScope,
    },
    model::Completeness,
    query::Locality,
};
use bytes::Bytes;
use http::{Request, Response};

/// An operation that clause 6.3.18 extends the `local` parameter to although its own table does not list it: every
/// operation on `/entities`, `/entityOperations`, `/temporal/entities` and `/temporal/entityOperations` other than the
/// queries and Purge Entities, which carry their own `locality` because their refusal of an unbounded selection
/// depends on it (Tables 6.4.3.2-1, 6.4.3.3-1 and 6.18.3.2-1). The discovery operations of `/types` and
/// `/attributes` carry their own too. The subscription, registry, snapshot, `@context` and entity map operations are
/// outside the clause and do not implement it.
///
/// Like [`Operation`], the trait is implemented by the operations of this crate only.
pub trait LocalScopeOperation: Operation {}

impl LocalScopeOperation for CreateEntity<'_> {}
impl<F, J: Default, C> LocalScopeOperation for RetrieveEntity<'_, F, J, C> where Self: Operation {}
impl LocalScopeOperation for ReplaceEntity<'_> {}
impl LocalScopeOperation for DeleteEntity<'_> {}
impl<C: Completeness> LocalScopeOperation for MergeEntity<'_, C> where Self: Operation {}
impl LocalScopeOperation for MergeSimplifiedEntity<'_> {}
impl<C: Completeness> LocalScopeOperation for UpdateAttributes<'_, C> where Self: Operation {}
impl<C: Completeness> LocalScopeOperation for AppendAttributes<'_, C> where Self: Operation {}
impl LocalScopeOperation for PartialAttributeUpdate<'_> {}
impl LocalScopeOperation for ReplaceAttribute<'_> {}
impl LocalScopeOperation for DeleteAttribute<'_> {}
impl LocalScopeOperation for BatchCreate<'_> {}
impl LocalScopeOperation for BatchUpsert<'_> {}
impl LocalScopeOperation for BatchUpdate<'_> {}
impl LocalScopeOperation for BatchMerge<'_> {}
impl LocalScopeOperation for BatchDelete<'_> {}
impl LocalScopeOperation for CreateOrUpdateTemporalEvolution<'_> {}
impl LocalScopeOperation for CreateOrUpdateSimplifiedTemporalEvolution<'_> {}
impl LocalScopeOperation for AddTemporalAttributes<'_> {}
impl LocalScopeOperation for DeleteTemporalAttribute<'_> {}
impl LocalScopeOperation for ModifyAttributeInstance<'_> {}
impl LocalScopeOperation for DeleteAttributeInstance<'_> {}
impl LocalScopeOperation for DeleteTemporalEvolution<'_> {}
impl<F, C> LocalScopeOperation for RetrieveTemporalEvolution<'_, F, C> where Self: Operation {}

/// An operation with the `local` parameter of clause 6.3.18: its request is the operation's own, with `local=true`
/// for [`Locality::Local`], `local=false` for [`Locality::Distributed`], and nothing for [`Locality::Unspecified`];
/// its outcome is the operation's.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteEntity, Operation, WithLocality},
///     },
///     model::EntityId,
///     query::Locality,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let delete = DeleteEntity::builder().id(&id).build();
/// let request = WithLocality::new(Locality::Local, &delete).to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567?local=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WithLocality<'a, O> {
    /// Whether the operation is limited to the broker's own data.
    pub locality: Locality,
    /// The operation.
    pub operation: &'a O,
}

impl<'a, O: LocalScopeOperation> WithLocality<'a, O> {
    /// `operation`, limited as `locality` says.
    #[must_use]
    pub const fn new(locality: Locality, operation: &'a O) -> WithLocality<'a, O> {
        WithLocality { locality, operation }
    }
}

impl<O: LocalScopeOperation> Sealed for WithLocality<'_, O> {}

impl<O: LocalScopeOperation> BuildsRequest for WithLocality<'_, O> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.operation.scoped_request(endpoint, scope.limited(self.locality))
    }
}

impl<O: LocalScopeOperation> Operation for WithLocality<'_, O> {
    type Outcome = O::Outcome;

    fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<O::Outcome, ResponseError> {
        self.operation.decode(context, response)
    }
}
