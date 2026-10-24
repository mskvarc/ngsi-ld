//! Why an API root or a setting of an [`Endpoint`](crate::http::Endpoint) is refused.

use crate::http::headers::LinkBuildError;
use http::HeaderName;
use iri_rs::IriRefBuf;

/// Why a URL cannot be an API root, or a setting cannot be stated on every request.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EndpointError {
    /// The URL has no hierarchical path to append resource paths to (a `mailto:` or `data:` URL).
    #[error("the API root must be a URL with a path")]
    NotABase,
    /// The URL carries a query or a fragment, which the resource paths cannot follow.
    #[error("the API root must not have a query or a fragment")]
    QueryOrFragment,
    /// The context reference cannot be carried in a `Link` header.
    #[error("the context reference cannot be sent in a Link header")]
    Context(#[source] LinkBuildError),
    /// A header the request pipeline sets itself from the operation and the endpoint (`Accept`, `Content-Type`,
    /// `Content-Length`, `Content-Encoding`, `Transfer-Encoding`, `Host`, `Link`, `Prefer`, `NGSILD-Tenant`,
    /// `NGSILD-EntityMap`, `NGSILD-Snapshot`), which an extra header must not override or contradict;
    /// [`Endpoint::with_accept`](crate::http::Endpoint::with_accept),
    /// [`Endpoint::with_prefer`](crate::http::Endpoint::with_prefer) and
    /// [`Endpoint::with_link`](crate::http::Endpoint::with_link) state the ones a caller chooses.
    #[error("the {name} header is set by the request pipeline")]
    ReservedHeader {
        /// The header.
        name: HeaderName,
    },
    /// A context that is not a single reference holds a relative reference. Such a context travels whole in an
    /// `application/ld+json` body (clause 6.3.5), where nothing resolves the reference for the broker; a single
    /// relative reference travels in the `Link` header instead, whose target the broker resolves against the request
    /// URI (IETF RFC 8288 section 3.2). A context kept in local files is loaded and made self-contained with
    /// `context_file::load`.
    #[error("the context reference {reference} is relative, and only a context that is a single reference may be")]
    RelativeContextReference {
        /// The reference.
        reference: IriRefBuf,
    },
    /// A profile is stated for the context link (clause 6.3.21), but the context is not a single reference, so no
    /// `Link` header carries it.
    #[error("a context profile needs a context that travels in a Link header")]
    ProfileWithoutContextLink,
    /// A further `Link` names a JSON-LD context, which only the endpoint's context may do (clause 6.3.5).
    #[error("a further Link header must not name a JSON-LD context")]
    ContextLink,
}
