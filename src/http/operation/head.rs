//! The HEAD request of a retrieval (ETSI TS 104 176 clause 5): the GET of the operation, answered with the head of
//! its response and no body.

use crate::http::{
    Endpoint,
    RequestError,
    ResponseContext,
    ResponseError,
    ResponseHead,
    operation::{
        ArgumentError,
        CreateEntityMap,
        CreateTemporalEntityMap,
        ListContextDetails,
        ListContexts,
        LocalScopeOperation,
        OnSnapshot,
        Operation,
        PageRequest,
        QueryEntities,
        QueryRegistrations,
        QuerySubscriptions,
        QueryTemporalEvolution,
        RetrieveAttributeDetails,
        RetrieveAttributeInfo,
        RetrieveAttributes,
        RetrieveEntity,
        RetrieveEntityMap,
        RetrieveEntityTypeDetails,
        RetrieveEntityTypeInfo,
        RetrieveEntityTypes,
        RetrieveRegistration,
        RetrieveSnapshot,
        RetrieveSourceIdentity,
        RetrieveSubscription,
        RetrieveTemporalEvolution,
        ServeContext,
        ServeContextDetails,
        SnapshotOperation,
        WithLocality,
        sealed::{BuildsRequest, Sealed},
        subscription::resource::SubscriptionResource,
    },
    request::scope::RequestScope,
    response::classify,
};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};

/// An operation whose resource answers HEAD.
///
/// It is an operation sent through GET on a resource for which Table 5-1 of ETSI TS 104 176 defines GET and HEAD: the
/// queries and retrievals of entities, of their temporal evolution, of subscriptions and of context source
/// registrations, the discovery of entity types and attributes, the listing and serving of stored `@context`s, the
/// creation through GET and the retrieval of entity maps, the retrieval of the source identity and of a snapshot, and
/// the pages a query links. An operation aimed at a snapshot or limited to local scope is one too when the operation
/// it wraps is.
///
/// The operations that write, and the queries sent through POST, have no HEAD. A query is one of these types whatever
/// its [`QueryMethod`](crate::http::operation::QueryMethod), so [`Head`] refuses one sent through POST when it builds
/// the request. Like [`Operation`], the trait is implemented by the operations of this crate only.
pub trait HeadOperation: Operation {}

impl<F, J: Default, C> HeadOperation for RetrieveEntity<'_, F, J, C> where Self: Operation {}
impl<F, C> HeadOperation for QueryEntities<'_, F, C> where Self: Operation {}
impl<T> HeadOperation for PageRequest<'_, T> where Self: Operation {}
impl<F, C> HeadOperation for RetrieveTemporalEvolution<'_, F, C> where Self: Operation {}
impl<F, C> HeadOperation for QueryTemporalEvolution<'_, F, C> where Self: Operation {}
impl<K: SubscriptionResource> HeadOperation for RetrieveSubscription<'_, K> {}
impl<K: SubscriptionResource> HeadOperation for QuerySubscriptions<K> {}
impl HeadOperation for RetrieveRegistration<'_> {}
impl HeadOperation for QueryRegistrations<'_> {}
impl HeadOperation for RetrieveEntityTypes {}
impl HeadOperation for RetrieveEntityTypeDetails {}
impl HeadOperation for RetrieveEntityTypeInfo<'_> {}
impl HeadOperation for RetrieveAttributes {}
impl HeadOperation for RetrieveAttributeDetails {}
impl HeadOperation for RetrieveAttributeInfo<'_> {}
impl HeadOperation for ListContexts {}
impl HeadOperation for ListContextDetails {}
impl HeadOperation for ServeContext<'_> {}
impl HeadOperation for ServeContextDetails<'_> {}
impl HeadOperation for CreateEntityMap<'_> {}
impl HeadOperation for CreateTemporalEntityMap<'_> {}
impl HeadOperation for RetrieveEntityMap<'_> {}
impl HeadOperation for RetrieveSourceIdentity {}
impl HeadOperation for RetrieveSnapshot<'_> {}
impl<O: SnapshotOperation + HeadOperation> HeadOperation for OnSnapshot<'_, O> {}
impl<O: LocalScopeOperation + HeadOperation> HeadOperation for WithLocality<'_, O> {}

/// The HEAD request of an operation.
///
/// Its request is the GET the operation sends, with the method HEAD: the same URI, parameters and headers, so the
/// broker selects the same resource as for the GET (ETSI TS 104 176 clause 5). A success is 204 No Content, without a
/// body, and its outcome is the head of the response, where the headers of the GET's answer are read: for instance
/// `NGSILD-Results-Count` of a query asked to count, `Link` to the next page, `ETag` or `Content-Type`. An error
/// carries no body either, so a resource that does not exist is a [`ResponseError::Status`] with the status 404.
///
/// ```
/// use bytes::Bytes;
/// use http::{Response, StatusCode};
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         ResponseContext,
///         operation::{Head, Operation, RetrieveEntity},
///     },
///     model::{EntityId, Lossless},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let retrieve = RetrieveEntity::builder()
///     .id(&id)
///     .format(Lossless::Normalized)
///     .build();
/// let head = Head::new(&retrieve);
///
/// let request = head.to_request(&endpoint)?;
/// assert_eq!(request.method(), "HEAD");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"
/// );
///
/// let context = ResponseContext::new(&request);
/// let mut response = Response::new(Bytes::new());
/// *response.status_mut() = StatusCode::NO_CONTENT;
/// let outcome = head.decode(&context, response)?;
/// assert!(outcome.headers().is_empty());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head<'a, O> {
    /// The operation whose GET the request mirrors.
    pub operation: &'a O,
}

impl<'a, O: HeadOperation> Head<'a, O> {
    /// The HEAD request of `operation`.
    #[must_use]
    pub const fn new(operation: &'a O) -> Head<'a, O> {
        Head { operation }
    }
}

impl<O: HeadOperation> Sealed for Head<'_, O> {}

impl<O: HeadOperation> BuildsRequest for Head<'_, O> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut request = self.operation.scoped_request(endpoint, scope)?;
        if request.method() != Method::GET {
            return Err(ArgumentError::HeadWithoutGet.into());
        }
        *request.method_mut() = Method::HEAD;
        Ok(request)
    }
}

impl<O: HeadOperation> Operation for Head<'_, O> {
    type Outcome = ResponseHead;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ResponseHead, ResponseError> {
        let success = classify(response)?;
        match success.status() {
            StatusCode::NO_CONTENT => Ok(success.into_head()),
            _ => Err(success.unexpected()),
        }
    }
}
