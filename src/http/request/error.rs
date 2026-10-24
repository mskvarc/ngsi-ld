//! Why the request pipeline did not build a request.

use crate::http::{
    headers::{InvalidFieldValue, LinkBuildError},
    operation::ArgumentError,
};
use http::uri::InvalidUri;
use iri_rs::IriRefBuf;

/// Why a request was not built: the operation's arguments are refused, the client's context cannot travel with
/// the request, or a value cannot be written into the request.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RequestError {
    /// The arguments of the operation make a request the specification refuses.
    #[error("the request is refused by its arguments")]
    Argument(#[from] ArgumentError),
    /// The client's context cannot travel with the request (clause 6.3.5).
    #[error("the context cannot travel with the request")]
    Context(#[from] ContextDeliveryError),
    /// The tenant cannot be carried in the `NGSILD-Tenant` header.
    #[error("the tenant cannot be sent in a header")]
    Tenant {
        /// Why.
        #[source]
        source: InvalidFieldValue,
    },
    /// The snapshot id cannot be sent in the `NGSILD-Snapshot` header (clause 6.3.22).
    #[error("the snapshot id cannot be sent in a header")]
    Snapshot {
        /// Why.
        #[source]
        source: InvalidFieldValue,
    },
    /// The request URI is not valid.
    #[error("invalid request URI")]
    Uri {
        /// Why.
        #[source]
        source: InvalidUri,
    },
}

/// Why the client's context cannot travel with a request (clause 6.3.5).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContextDeliveryError {
    /// The client's context is not a single reference, so it cannot travel in the `Link` header that a request
    /// without a body, or with an `application/json` body, carries it in (clause 6.3.5); such a context needs
    /// [`ContextDelivery::LdJsonBody`](crate::http::ContextDelivery::LdJsonBody) and a request with a
    /// body.
    #[error("a context that is not a single reference can only travel in an application/ld+json body")]
    NotLinkable,
    /// The client's context is a single relative reference, which only the `Link` header carries (IETF RFC 8288
    /// section 3.2 resolves its target against the request URI), but the request carries its context in an
    /// `application/ld+json` body, where nothing resolves the reference for the broker.
    #[error("the relative context reference {reference} can only travel in a Link header")]
    RelativeInBody {
        /// The reference.
        reference: IriRefBuf,
    },
    /// The endpoint states a profile for its context link (clause 6.3.21), but the request carries its context in
    /// an `application/ld+json` body, with no `Link` header to name the profile in. Add @context is not refused: it
    /// sends no context of the client's, so no profile stated for one.
    #[error("a context profile needs a Link header, which an application/ld+json body excludes")]
    ProfileWithoutLink,
    /// The link to the context with its profile cannot be carried in a `Link` header.
    #[error("the context link with its profile cannot be sent in a Link header")]
    Link {
        /// Why.
        #[source]
        source: LinkBuildError,
    },
}
