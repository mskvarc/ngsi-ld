//! Everything of a response's head besides its status: the HTTP version, the header fields, and what the transport
//! attached.

use crate::http::headers::OptionalHeader;
use bytes::Bytes;
use headers::Header;
use http::{Extensions, HeaderMap, Response, Version, response::Parts};

/// The head of a broker's response, without its status, which the outcome or the error states on its own: the HTTP
/// version, every header field as received, and the extensions the transport attached (for instance the TLS
/// details its HTTP library reports).
///
/// Every header is readable in typed form with [`typed`](Self::typed): the NGSI-LD headers of
/// [`headers`](crate::http::headers) and every header of the `headers` crate (`ETag`, `Vary`, `CacheControl`,
/// `Allow`, `RetryAfter`, `ContentType`, ...).
///
/// ```
/// use headers::{ETag, Vary};
/// use http::{HeaderMap, HeaderValue, header};
/// use ngsi_ld::http::ResponseHead;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(&header::ETAG, HeaderValue::from_static("\"v1\""));
/// headers.insert(&header::VARY, HeaderValue::from_static("Accept"));
/// let head = ResponseHead::new(headers);
/// assert!(head.typed::<ETag>().is_valid());
/// assert!(
///     head.typed::<Vary>()
///         .into_valid()
///         .is_some_and(|vary| vary.iter_strs().eq(["Accept"]))
/// );
/// ```
#[derive(Debug, Clone, Default)]
pub struct ResponseHead {
    /// The HTTP version.
    version: Version,
    /// The header fields.
    headers: HeaderMap,
    /// What the transport attached.
    extensions: Extensions,
}

impl ResponseHead {
    /// A head with `headers`, HTTP/1.1 and no extensions, for a response built by hand, such as a test double's.
    #[must_use]
    pub fn new(headers: HeaderMap) -> ResponseHead {
        ResponseHead {
            version: Version::HTTP_11,
            headers,
            extensions: Extensions::new(),
        }
    }

    /// The head of `parts`, which it takes.
    pub(crate) fn from_parts(parts: Parts) -> ResponseHead {
        ResponseHead {
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
        }
    }

    /// A copy of the head of `response`, which keeps its own.
    pub(crate) fn of(response: &Response<Bytes>) -> ResponseHead {
        ResponseHead {
            version: response.version(),
            // The head is kept beside the outcome, and the response keeps its own until it is read.
            headers: response.headers().clone(),
            extensions: response.extensions().clone(),
        }
    }

    /// The HTTP version of the response.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// The header fields, as received.
    #[must_use]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// What the transport attached to the response.
    #[must_use]
    pub const fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// The header `H`: absent, read, or present but unreadable with its field lines kept.
    #[must_use]
    pub fn typed<H: Header>(&self) -> OptionalHeader<H> {
        OptionalHeader::read(&self.headers)
    }
}

#[cfg(test)]
mod tests {
    use crate::http::{
        ResponseHead,
        headers::{NgsiLdTenant, OptionalHeader},
    };
    use bytes::Bytes;
    use headers::{Allow, Header, RetryAfter};
    use http::{HeaderMap, HeaderValue, Method, Response, Version, header};
    use pretty_assertions::assert_eq;

    #[test]
    fn a_head_keeps_version_headers_and_extensions_of_the_response() {
        let mut response = Response::new(Bytes::new());
        *response.version_mut() = Version::HTTP_2;
        response.headers_mut().insert(&header::ALLOW, HeaderValue::from_static("GET, DELETE"));
        response.extensions_mut().insert(7_u8);
        let head = ResponseHead::of(&response);
        assert_eq!(head.version(), Version::HTTP_2);
        assert_eq!(head.extensions().get::<u8>(), Some(&7));
        let allow = head.typed::<Allow>().into_valid().unwrap();
        assert_eq!(allow.iter().collect::<Vec<_>>(), [Method::GET, Method::DELETE]);
        let (parts, _) = response.into_parts();
        assert_eq!(ResponseHead::from_parts(parts).headers()[&header::ALLOW], "GET, DELETE");
    }

    #[test]
    fn a_typed_header_is_absent_read_or_malformed() {
        let mut headers = HeaderMap::new();
        headers.insert(NgsiLdTenant::name(), HeaderValue::from_static("fleet"));
        headers.insert(&header::RETRY_AFTER, HeaderValue::from_static("soon"));
        let head = ResponseHead::new(headers);
        assert_eq!(
            head.typed::<NgsiLdTenant>().valid().map(|tenant| tenant.tenant().to_string()).as_deref(),
            Some("fleet")
        );
        assert!(head.typed::<RetryAfter>().is_malformed());
        assert!(head.typed::<Allow>().is_absent());
        assert!(matches!(ResponseHead::default().typed::<Allow>(), OptionalHeader::Absent));
    }
}
