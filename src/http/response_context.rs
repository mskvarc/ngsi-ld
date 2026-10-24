//! What decoding a response needs to know about the request it answers.

use crate::{
    http::headers::{NgsiLdEntityMap, NgsiLdSnapshot, NgsiLdTenant},
    model::Tenant,
    snapshot::SnapshotId,
};
use headers::HeaderMapExt;
use http::Request;
use url::Url;

/// The request a response answers, as far as decoding the response depends on it: the request URL, against which a
/// relative `Link` target is resolved (RFC 8288 section 3.2 makes the request URI the default context of a link),
/// the tenant the request addressed, which the following pages of a query must address too (NGSI-LD clauses
/// 6.3.10 and 6.3.14), the snapshot it was aimed at, which they must be aimed at too (clause 6.3.22), and the entity
/// map the request asked the broker to use, which keeps the following pages
/// consistent (clause 4.3.6.7).
///
/// It is read from the request that was sent, so it states what reached the broker rather than what the caller
/// meant to send.
///
/// ```
/// use bytes::Bytes;
/// use http::Request;
/// use ngsi_ld::http::ResponseContext;
///
/// let request =
///     Request::get("http://broker.test/ngsi-ld/v1/entities?type=Vehicle").body(Bytes::new())?;
/// let context = ResponseContext::new(&request);
/// # let _ = context;
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseContext {
    /// The request URL; `None` when the request URI is not an absolute URL, so that only absolute link targets
    /// resolve.
    request_url: Option<Url>,
    /// The tenant of the `NGSILD-Tenant` header, or `None` for the default tenant.
    tenant: Option<Tenant>,
    /// The snapshot of the `NGSILD-Snapshot` header, if the request was aimed at one.
    snapshot: Option<SnapshotId>,
    /// The entity map of the `NGSILD-EntityMap` header, if the request named one.
    entity_map: Option<NgsiLdEntityMap>,
}

impl ResponseContext {
    /// The context of `request`.
    #[must_use]
    pub fn new<B>(request: &Request<B>) -> ResponseContext {
        ResponseContext {
            // The pipeline builds absolute URIs; an unchecked request may carry anything, hence the fallible parse.
            request_url: Url::parse(&request.uri().to_string()).ok(),
            tenant: request.headers().typed_get::<NgsiLdTenant>().map(NgsiLdTenant::into_tenant),
            snapshot: request.headers().typed_get::<NgsiLdSnapshot>().map(NgsiLdSnapshot::into_snapshot),
            entity_map: request.headers().typed_get::<NgsiLdEntityMap>(),
        }
    }

    /// Resolves a link target against the request URL (RFC 8288 section 3.2, RFC 3986 section 5).
    pub(crate) fn resolve(&self, target: &str) -> Option<Url> {
        Url::options().base_url(self.request_url.as_ref()).parse(target).ok()
    }

    /// The tenant the request addressed.
    pub(crate) const fn tenant(&self) -> Option<&Tenant> {
        self.tenant.as_ref()
    }

    /// The snapshot the request was aimed at.
    pub(crate) const fn snapshot(&self) -> Option<&SnapshotId> {
        self.snapshot.as_ref()
    }

    /// The entity map the request asked the broker to use.
    pub(crate) const fn entity_map(&self) -> Option<&NgsiLdEntityMap> {
        self.entity_map.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::http::{
        ResponseContext,
        headers::{NgsiLdEntityMap, NgsiLdTenant},
    };
    use bytes::Bytes;
    use headers::Header;
    use http::Request;
    use pretty_assertions::assert_eq;
    use url::Url;

    fn request(uri: &str, tenant: Option<&'static str>) -> Request<Bytes> {
        let builder = Request::get(uri);
        let builder = match tenant {
            Some(tenant) => builder.header(NgsiLdTenant::name(), tenant),
            None => builder,
        };
        builder.body(Bytes::new()).unwrap()
    }

    #[test]
    fn a_relative_target_resolves_against_the_request_url() {
        let context = ResponseContext::new(&request("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&limit=2", None));
        let resolved = |target| context.resolve(target).as_ref().map(Url::to_string);
        assert_eq!(
            resolved("?type=Vehicle&limit=2&offset=2"),
            Some("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&limit=2&offset=2".to_owned())
        );
        assert_eq!(
            resolved("entities?offset=2"),
            Some("http://broker.test/ngsi-ld/v1/entities?offset=2".to_owned())
        );
        assert_eq!(
            resolved("/other/entities?offset=2"),
            Some("http://broker.test/other/entities?offset=2".to_owned())
        );
        assert_eq!(resolved("https://elsewhere.test/page/2"), Some("https://elsewhere.test/page/2".to_owned()));
    }

    #[test]
    fn without_an_absolute_request_url_only_absolute_targets_resolve() {
        let context = ResponseContext::new(&request("/ngsi-ld/v1/entities", None));
        assert_eq!(context.resolve("?offset=2"), None);
        assert_eq!(
            context.resolve("https://elsewhere.test/page/2").map(String::from),
            Some("https://elsewhere.test/page/2".to_owned())
        );
    }

    #[test]
    fn the_tenant_is_read_from_the_request_header() {
        assert_eq!(
            ResponseContext::new(&request("http://broker.test/", Some("t1")))
                .tenant()
                .map(ToString::to_string),
            Some("t1".to_owned())
        );
        assert_eq!(ResponseContext::new(&request("http://broker.test/", None)).tenant(), None);
    }

    #[test]
    fn the_entity_map_is_read_from_the_request_header() {
        let request = Request::get("http://broker.test/ngsi-ld/v1/entities?type=Vehicle")
            .header(NgsiLdEntityMap::name(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
            .body(Bytes::new())
            .unwrap();
        assert_eq!(
            ResponseContext::new(&request).entity_map().map(|map| map.as_iri_ref().to_string()),
            Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1".to_owned())
        );
        assert_eq!(ResponseContext::new(&request_without_map()).entity_map(), None);
    }

    fn request_without_map() -> Request<Bytes> {
        request("http://broker.test/ngsi-ld/v1/entities?type=Vehicle", None)
    }
}
