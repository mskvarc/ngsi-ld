//! The [`Client`]: the endpoint and transport every operation goes through, and the handles of its resources.

use crate::{
    client::{
        Answering,
        AvailableAttributes,
        AvailableEntityTypes,
        ClientError,
        Entities,
        EntityMaps,
        OutcomeOnly,
        Pages,
        Registrations,
        SnapshotView,
        Snapshots,
        StoredContexts,
        Subscriptions,
        Temporal,
        Transport,
        route::Route,
    },
    http::{
        ContextDelivery,
        Endpoint,
        EndpointError,
        Page,
        ResponseContext,
        UncheckedRequest,
        headers::{Accept, Prefer},
        operation::Operation,
        request::scope::RequestScope,
        response::classify,
    },
    model::{Context, Tenant},
    seal::Seal,
    snapshot::SnapshotId,
    subscription::{EntitySubscription, RegistrationSubscription},
};
use bon::bon;
use bytes::Bytes;
use http::Response;
use url::Url;

/// A client of one NGSI-LD broker.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         Ok(Response::new(Bytes::new()))
/// #     }
/// # }
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::client::Client;
/// use url::Url;
///
/// let client = Client::builder()
///     .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?)
///     .context(IriRefBuf::new("https://example.org/context.jsonld")?)
///     .transport(Offline)
///     .build()?;
/// assert_eq!(
///     client.endpoint().base_url().as_str(),
///     "http://localhost:9090/ngsi-ld/v1/"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// A client derives others that state something else on every request than the client they come from: another
/// `@context`, `Accept`, `Prefer`, tenant or further header ([`with_context`](Client::with_context),
/// [`with_accept`](Client::with_accept), [`with_tenant`](Client::with_tenant), [`with_header`](Client::with_header)
/// and the others). Each holds a clone of the original's transport, which for `ReqwestTransport` shares its
/// connection pool, and [`Client::by_ref`] gives a client over a borrowed transport for a transport that is not
/// `Clone`. Derived clients chain: `client.with_accept(..).with_tenant(..)`.
#[derive(Debug, Clone)]
pub struct Client<T, M = OutcomeOnly> {
    /// The broker, the tenant of calls that name none, and how to speak to it.
    pub(super) endpoint: Endpoint,
    /// Moves requests and responses.
    pub(super) transport: T,
    /// What a call answers: the outcome alone, or with the head of the response.
    pub(super) answering: M,
}

#[bon]
impl<T: Transport> Client<T> {
    /// Builds a client. `prefer` is stated on every request, for instance the version of the specification the
    /// payloads are wanted in (clause 6.3.6), which a 203 answer confirms with `Preference-Applied`; `accept`, when
    /// given, replaces the representation each operation asks for (clause 6.3.4).
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError`] when `base_url` cannot be an API root, the context holds a relative reference, or
    /// the context reference cannot be carried in a `Link` header.
    #[builder]
    pub fn new(
        base_url: Url,
        #[builder(into)] context: Option<Context>,
        #[builder(default)] context_delivery: ContextDelivery,
        prefer: Option<Prefer>,
        accept: Option<Accept>,
        tenant: Option<Tenant>,
        transport: T,
    ) -> Result<Client<T>, EndpointError> {
        let endpoint = Endpoint::builder()
            .base_url(base_url)
            .maybe_context(context)
            .context_delivery(context_delivery)
            .maybe_prefer(prefer)
            .maybe_accept(accept)
            .maybe_tenant(tenant)
            .build()?;
        Ok(Client {
            endpoint,
            transport,
            answering: OutcomeOnly,
        })
    }
}

impl<T: Transport, M: Answering> Client<T, M> {
    /// The broker and how the client speaks to it.
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The tenant of calls that name none.
    #[must_use]
    pub const fn tenant(&self) -> Option<&Tenant> {
        self.endpoint.tenant()
    }

    /// Sends any operation on the client's tenant: builds its request, sends it, and reads the response. A client
    /// from [`with_tenant`](Client::with_tenant) sends on another tenant. The answer is the outcome, or, for a client
    /// from [`with_heads`](Client::with_heads), the outcome with the status and head of the response, and for a client
    /// from [`with_responses`](Client::with_responses), the outcome with the status, head and body of the response.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request is refused before sending, the transport fails, or the broker
    /// answers with an error.
    pub async fn send<O: Operation + Sync>(&self, operation: &O) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        self.send_on(operation, None, None).await
    }

    /// Sends an operation as [`send`](Self::send) does, on `tenant` instead of the client's when one is given: the
    /// tenant a handle's call names.
    pub(crate) async fn send_on_tenant<O: Operation + Sync>(
        &self,
        operation: &O,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        self.send_on(operation, tenant, None).await
    }

    /// Sends an operation as [`send`](Self::send) does, on `tenant` instead of the client's when one is given, and
    /// aimed at `snapshot` when there is one; the handles of a [`SnapshotView`], which only offers operations of the
    /// Core and Temporal APIs, send through it.
    pub(crate) async fn send_on<O: Operation + Sync>(
        &self,
        operation: &O,
        tenant: Option<&Tenant>,
        snapshot: Option<&SnapshotId>,
    ) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        let scope = RequestScope::on_tenant(tenant.or(self.endpoint.tenant())).on_snapshot(snapshot);
        let request = operation.scoped_request(&self.endpoint, scope).map_err(ClientError::Request)?;
        let context = ResponseContext::new(&request);
        let response = self.transport.send(request).await.map_err(ClientError::Transport)?;
        let capture = M::capture(Seal::new(), &response);
        let outcome = operation.decode(&context, response).map_err(ClientError::Response)?;
        Ok(M::answer(Seal::new(), outcome, capture))
    }

    /// Builds the request `operation` would send on the client's tenant, for breaking on purpose (see
    /// [`UncheckedRequest`]).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Request`] when the operation itself refuses its parameters.
    pub fn unchecked<O: Operation>(&self, operation: &O) -> Result<UncheckedRequest, ClientError<T::Error>> {
        UncheckedRequest::try_from_operation(operation, &self.endpoint).map_err(ClientError::Request)
    }

    /// Sends an unchecked request. An error response is decoded as for every other call, so a broker's rejection
    /// comes back as [`ClientError::Response`] with its problem; a success response is returned as received.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the transport fails or the broker answers with an error.
    pub async fn send_unchecked(&self, request: UncheckedRequest) -> Result<Response<Bytes>, ClientError<T::Error>> {
        let response = self.transport.send(request.into_request()).await.map_err(ClientError::Transport)?;
        classify(response).map(|success| success.0).map_err(ClientError::Response)
    }
}

impl<T: Transport, M: Answering> Client<T, M> {
    /// The operations on entities and their attributes (NGSI-LD clauses 5.6 and 5.7).
    #[must_use]
    pub const fn entities(&self) -> Entities<'_, T, M> {
        Entities::new(Route::direct(self))
    }

    /// The operations on context information subscriptions (NGSI-LD clause 5.8).
    #[must_use]
    pub const fn subscriptions(&self) -> Subscriptions<'_, T, EntitySubscription, M> {
        Subscriptions::new(Route::direct(self), EntitySubscription)
    }

    /// The operations on context source registrations: registration and discovery (NGSI-LD clauses 5.9 and 5.10).
    #[must_use]
    pub const fn registrations(&self) -> Registrations<'_, T, M> {
        Registrations::new(self)
    }

    /// The operations on Context Source Registration Subscriptions (NGSI-LD clause 5.11).
    #[must_use]
    pub const fn registration_subscriptions(&self) -> Subscriptions<'_, T, RegistrationSubscription, M> {
        Subscriptions::new(Route::direct(self), RegistrationSubscription)
    }

    /// The operations on the temporal evolution of entities (NGSI-LD clauses 5.6.11 to 5.6.16, 5.7.3 and 5.7.4).
    #[must_use]
    pub const fn temporal(&self) -> Temporal<'_, T, M> {
        Temporal::new(Route::direct(self))
    }

    /// The entity type discovery operations (NGSI-LD clauses 5.7.5 to 5.7.7).
    #[must_use]
    pub const fn entity_types(&self) -> AvailableEntityTypes<'_, T, M> {
        AvailableEntityTypes::new(Route::direct(self))
    }

    /// The attribute discovery operations (NGSI-LD clauses 5.7.8 to 5.7.10).
    #[must_use]
    pub const fn attributes(&self) -> AvailableAttributes<'_, T, M> {
        AvailableAttributes::new(Route::direct(self))
    }

    /// The operations on the `@context`s the broker stores and serves (NGSI-LD clause 5.13).
    #[must_use]
    pub const fn contexts(&self) -> StoredContexts<'_, T, M> {
        StoredContexts::new(self)
    }

    /// The operations on the entity maps of distributed queries (NGSI-LD clause 5.14).
    #[must_use]
    pub const fn entity_maps(&self) -> EntityMaps<'_, T, M> {
        EntityMaps::new(self)
    }

    /// The operations on snapshots (NGSI-LD clause 5.16).
    #[must_use]
    pub const fn snapshots(&self) -> Snapshots<'_, T, M> {
        Snapshots::new(self)
    }

    /// The handles of the Core and Temporal APIs, every call aimed at `snapshot` (NGSI-LD clauses 5.5.15 and
    /// 6.3.22).
    #[must_use]
    pub const fn on_snapshot<'c>(&'c self, snapshot: &'c SnapshotId) -> SnapshotView<'c, T, M> {
        SnapshotView::new(self, snapshot)
    }

    /// Every page of a query, starting with `first`, the page its `send()` returned; each following page is
    /// requested from the `next` link of the one before (clause 6.3.10), on the tenant of the query.
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use http::{Request, Response};
    /// # use ngsi_ld::client::Transport;
    /// # use std::convert::Infallible;
    /// # struct Offline;
    /// # impl Transport for Offline {
    /// #     type Error = Infallible;
    /// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
    /// #         Ok(Response::new(Bytes::from_static(b"[]")))
    /// #     }
    /// # }
    /// use ngsi_ld::{client::Client, model::SimplifiedEntity};
    ///
    /// async fn every_vehicle(client: &Client<Offline>) -> anyhow::Result<Vec<SimplifiedEntity>> {
    ///     let first = client
    ///         .entities()
    ///         .query_simplified()
    ///         .entity_type("Vehicle")
    ///         .limit(100)
    ///         .send()
    ///         .await?;
    ///     Ok(client.pages(first).try_collect().await?)
    /// }
    /// ```
    #[must_use]
    pub fn pages<I>(&self, first: M::Answer<Page<I>>) -> Pages<'_, T, I, M> {
        Pages::new(self, first)
    }
}
