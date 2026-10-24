//! Clients derived from another, which state something else on every request.

use crate::{
    client::{Answering, Client, Transport, WithHead},
    http::{
        ContextDelivery,
        Endpoint,
        EndpointError,
        headers::{Accept, Link, Prefer},
    },
    model::{Context, Tenant},
};
use headers::Header;
use http::{HeaderName, HeaderValue};
use iri_rs::IriBuf;

impl<T: Transport + Clone, M: Answering> Client<T, M> {
    /// A client stating `context` on every request (clause 6.3.5): any `@context`, a single reference sent in the
    /// `Link` header, or an inline definition or an array, which only an `application/ld+json` body carries
    /// (see [`with_context_delivery`](Self::with_context_delivery)).
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use http::{Request, Response};
    /// # use std::convert::Infallible;
    /// # #[derive(Clone)]
    /// # struct Offline;
    /// # impl ngsi_ld::client::Transport for Offline {
    /// #     type Error = Infallible;
    /// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> { Ok(Response::new(Bytes::new())) }
    /// # }
    /// use iri_rs::IriRefBuf;
    /// use ngsi_ld::client::Client;
    /// use url::Url;
    ///
    /// let client = Client::builder().base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?).transport(Offline).build()?;
    /// let fleet = client.with_context(IriRefBuf::new("https://example.org/fleet.jsonld")?)?;
    /// assert_eq!(fleet.endpoint().context().and_then(|context| context.link_target()).map(|target| target.as_str()), Some("https://example.org/fleet.jsonld"));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::RelativeContextReference`] when the context holds a relative reference and is not a
    /// single reference (a context kept in local files is read with `with_context_file`),
    /// [`EndpointError::ProfileWithoutContextLink`] when this client states a context profile and the context is not a
    /// single reference, and
    /// [`EndpointError::Context`] when the context reference cannot be carried in a `Link` header.
    pub fn with_context(&self, context: impl Into<Context>) -> Result<Client<T, M>, EndpointError> {
        self.try_derive(|endpoint| endpoint.with_context(context.into()))
    }

    /// A client stating no `@context` of its own, so every term expands under the core context (clause 4.3.4).
    #[must_use]
    pub fn without_context(&self) -> Client<T, M> {
        self.derive(Endpoint::without_context)
    }

    /// A client delivering the `@context` of request bodies as `delivery` says (clause 6.3.5).
    #[must_use]
    pub fn with_context_delivery(&self, delivery: ContextDelivery) -> Client<T, M> {
        self.derive(|endpoint| endpoint.with_context_delivery(delivery))
    }

    /// A client accepting `accept` on every request instead of the representation each operation asks for
    /// (clause 6.3.4): for instance `application/ld+json`, which returns the `@context` in the body.
    #[must_use]
    pub fn with_accept(&self, accept: impl Into<Accept>) -> Client<T, M> {
        self.derive(|endpoint| endpoint.with_accept(Some(accept.into())))
    }

    /// A client stating `prefer` on every request instead of this client's preferences (clause 6.3.6).
    #[must_use]
    pub fn with_prefer(&self, prefer: Prefer) -> Client<T, M> {
        self.derive(|endpoint| endpoint.with_prefer(Some(prefer)))
    }

    /// A client sending `header` on every request, such as `Via` (clause 6.3.18) or a header a context source
    /// expects to have forwarded (clause 4.3.6.5).
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ReservedHeader`] for a header the request pipeline sets itself.
    pub fn with_header<H: Header>(&self, header: H) -> Result<Client<T, M>, EndpointError> {
        self.try_derive(|endpoint| endpoint.with_header(header))
    }

    /// A client sending the field `name: value` on every request: a header no typed [`Header`] models, such as one
    /// a context source expects to have forwarded from the original request (clause 4.3.6.5).
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ReservedHeader`] for a header the request pipeline sets itself.
    pub fn with_header_field(&self, name: HeaderName, value: HeaderValue) -> Result<Client<T, M>, EndpointError> {
        self.try_derive(|endpoint| endpoint.with_header_field(name, value))
    }

    /// A client naming `profile` in the `profile` parameter of the link to its context, such as
    /// `https://uri.etsi.org/ngsi-ld/profile/version` (clause 6.3.21).
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ProfileWithoutContextLink`] when the context is not a single reference.
    pub fn with_context_profile(&self, profile: IriBuf) -> Result<Client<T, M>, EndpointError> {
        self.try_derive(|endpoint| endpoint.with_context_profile(profile))
    }

    /// A client naming no profile in the link to its context.
    #[must_use]
    pub fn without_context_profile(&self) -> Client<T, M> {
        self.derive(Endpoint::without_context_profile)
    }

    /// A client sending no further `Link`.
    #[must_use]
    pub fn without_link(&self) -> Client<T, M> {
        self.derive(Endpoint::without_link)
    }

    /// A client accepting the representation each operation asks for, instead of this client's `Accept`.
    #[must_use]
    pub fn without_accept(&self) -> Client<T, M> {
        self.derive(|endpoint| endpoint.with_accept(None))
    }

    /// A client stating no preferences (clause 6.3.6).
    #[must_use]
    pub fn without_prefer(&self) -> Client<T, M> {
        self.derive(|endpoint| endpoint.with_prefer(None))
    }

    /// A client sending `link` on every request beside the `Link` naming the context, such as the profile link of
    /// clause 6.3.21.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ContextLink`] when a link-value of `link` names a JSON-LD context.
    pub fn with_link(&self, link: Link) -> Result<Client<T, M>, EndpointError> {
        self.try_derive(|endpoint| endpoint.with_link(link))
    }

    /// A client whose calls that name no tenant go to the default tenant (clause 4.14).
    #[must_use]
    pub fn without_tenant(&self) -> Client<T, M> {
        self.derive(Endpoint::without_tenant)
    }

    /// A client whose calls that name no tenant go to `tenant` (clause 4.14).
    #[must_use]
    pub fn with_tenant(&self, tenant: Tenant) -> Client<T, M> {
        self.derive(|endpoint| endpoint.with_tenant(tenant))
    }

    /// A client whose calls answer the outcome together with the status and head of the response
    /// ([`Answered`](crate::client::Answered)), for reading a header the outcome does not carry.
    #[must_use]
    pub fn with_heads(&self) -> Client<T, WithHead> {
        Client {
            // The derived client owns its endpoint, and this client keeps its own.
            endpoint: self.endpoint.clone(),
            // The derived client owns its transport; a transport clone shares what the transport pools.
            transport: self.transport.clone(),
            answering: WithHead,
        }
    }

    /// A client on a clone of this client's transport, with this client's endpoint changed by `change`.
    fn derive(&self, change: impl FnOnce(Endpoint) -> Endpoint) -> Client<T, M> {
        // The derived client owns its endpoint, and this client keeps its own.
        self.derive_for(change(self.endpoint.clone()))
    }

    /// A client on a clone of this client's transport, with this client's endpoint changed by `change`,
    /// which may refuse the change.
    fn try_derive(&self, change: impl FnOnce(Endpoint) -> Result<Endpoint, EndpointError>) -> Result<Client<T, M>, EndpointError> {
        // The derived client owns its endpoint, and this client keeps its own.
        Ok(self.derive_for(change(self.endpoint.clone())?))
    }

    /// A client on a clone of this client's transport, with `endpoint`.
    fn derive_for(&self, endpoint: Endpoint) -> Client<T, M> {
        Client {
            endpoint,
            // The derived client owns its transport; a transport clone shares what the transport pools.
            transport: self.transport.clone(),
            answering: self.answering,
        }
    }
}

impl<T: Transport, M: Answering> Client<T, M> {
    /// The same client over a borrowed transport, which is `Clone` whatever the transport, so the derived clients
    /// of this module are available for any transport.
    #[must_use]
    pub fn by_ref(&self) -> Client<&T, M> {
        Client {
            // The borrowed client owns its endpoint, and this client keeps its own.
            endpoint: self.endpoint.clone(),
            transport: &self.transport,
            answering: self.answering,
        }
    }
}
