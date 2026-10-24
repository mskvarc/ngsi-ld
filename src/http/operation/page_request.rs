//! Requests for the neighbouring pages of a query (NGSI-LD clause 6.3.10).

use crate::{
    http::{
        Endpoint,
        Page,
        RequestError,
        headers::NgsiLdEntityMap,
        operation::{
            GeoJson,
            Simplified,
            entity_format::{EntityFormat, decode_page},
            sealed::Sealed,
        },
        request::{RequestParts, Target, scope::RequestScope},
    },
    model::{Complete, Entity, Feature, Lossless, NgsiLdMediaType, Output, Projected, SimplifiedEntity, Tenant},
    snapshot::SnapshotId,
};
use bytes::Bytes;
use http::{Method, Request};
use std::marker::PhantomData;
use url::Url;

/// Requests a page link a broker returned, read as items of type `T`.
///
/// Its request is a `GET` of the link (NGSI-LD clause 6.3.10).
///
/// The link already carries every parameter of the original query, including its `format` and `geometryProperty`.
/// A page of [`Entity`] values is read with the concise reader, which also reads normalized attributes (clause
/// 4.5.2.3), so either representation reads correctly; a page of [`Feature`] values is asked for with the `GeoJSON`
/// `Accept` header again, which the link cannot carry (clause 6.3.15).
///
/// The request addresses the tenant of the page it was taken from, whatever tenant it is sent with, is aimed at the
/// snapshot the page was taken from, and names the entity map the page reports: the following page belongs to the
/// same query, and a query on one tenant never continues on another (clause 6.3.14), nor on another snapshot (clause
/// 6.3.22) or entity map (clause 4.3.6.7). To walk every page, see
/// `Client::pages`.
///
/// ```
/// use bytes::Bytes;
/// use http::Request;
/// use ngsi_ld::{
///     http::{Endpoint, Page, operation::Operation},
///     model::SimplifiedEntity,
/// };
///
/// /// The request for the page after `page`, if there is one.
/// fn next_request(
///     page: &Page<SimplifiedEntity>,
///     endpoint: &Endpoint,
/// ) -> anyhow::Result<Option<Request<Bytes>>> {
///     let Some(next) = page.next_page() else {
///         return Ok(None);
///     };
///     Ok(Some(next.to_request(endpoint)?))
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest<'a, T> {
    /// The page link.
    url: &'a Url,
    /// The tenant of the query, `None` for the default tenant.
    tenant: Option<&'a Tenant>,
    /// The snapshot the query was aimed at.
    snapshot: Option<&'a SnapshotId>,
    /// The entity map of the query, if it uses one.
    entity_map: Option<&'a NgsiLdEntityMap>,
    /// The item type.
    item: PhantomData<fn() -> T>,
}

impl<'a, T> PageRequest<'a, T> {
    /// The request for the page at `url`, on `tenant` and `snapshot`, naming `entity_map`.
    pub(crate) const fn new(
        url: &'a Url,
        tenant: Option<&'a Tenant>,
        snapshot: Option<&'a SnapshotId>,
        entity_map: Option<&'a NgsiLdEntityMap>,
    ) -> PageRequest<'a, T> {
        PageRequest {
            url,
            tenant,
            snapshot,
            entity_map,
            item: PhantomData,
        }
    }
}

impl<T> Page<T> {
    /// The request for the next page, if there is one.
    #[must_use]
    pub fn next_page(&self) -> Option<PageRequest<'_, T>> {
        self.next()
            .map(|url| PageRequest::new(url, self.tenant(), self.snapshot(), self.following_entity_map()))
    }

    /// The request for the previous page, if there is one.
    #[must_use]
    pub fn prev_page(&self) -> Option<PageRequest<'_, T>> {
        self.prev()
            .map(|url| PageRequest::new(url, self.tenant(), self.snapshot(), self.following_entity_map()))
    }
}

impl<T> Sealed for PageRequest<'_, T> {}

/// Implements [`Operation`] and [`BuildsRequest`] for the pages of a query whose items are `Item`:
/// `page_operation!(impl<generics> for Item => decode, media_type)` reads a page with `decode`, called with the
/// response context and the response, and asks for `media_type`. The request is built on the page's own tenant.
/// The generics are optional.
macro_rules! page_operation {
    (impl $(<$($param:ident: $bound:ident),+>)? for $item:ty => $decode:expr, $media_type:expr) => {
        impl$(<$($param: $bound),+>)? $crate::http::operation::Operation for $crate::http::operation::PageRequest<'_, $item> {
            type Outcome = $crate::http::Page<$item>;

            fn decode(
                &self,
                context: &$crate::http::ResponseContext,
                response: ::http::Response<::bytes::Bytes>,
            ) -> Result<$crate::http::Page<$item>, $crate::http::ResponseError> {
                ($decode)(context, response)
            }
        }

        impl$(<$($param: $bound),+>)? $crate::http::operation::sealed::BuildsRequest for $crate::http::operation::PageRequest<'_, $item> {
            /// Builds the request on the page's own tenant; the tenant of `scope` is not used.
            fn scoped_request(
                &self,
                endpoint: &$crate::http::Endpoint,
                scope: $crate::http::request::scope::RequestScope<'_>,
            ) -> Result<::http::Request<::bytes::Bytes>, $crate::http::RequestError> {
                self.request_accepting(endpoint, scope, $media_type)
            }
        }
    };
}

/// Implements the page operation for entities of type `$item`, read in `$format` with the completeness
/// `$completeness`, and asking for the media type of the format.
macro_rules! entity_page {
    ($item:ty, $completeness:ty, $format:expr) => {
        page_operation!(
            impl for $item => |context, response| decode_page::<$completeness, _>(context, response, $format),
            EntityFormat::<$completeness>::media_type($format)
        );
    };
}

entity_page!(Entity<Output>, Complete, Lossless::Concise);
entity_page!(SimplifiedEntity, Complete, Simplified);
entity_page!(Entity<Output, Projected>, Projected, Lossless::Concise);
entity_page!(SimplifiedEntity<Projected>, Projected, Simplified);
entity_page!(
    Feature<Entity<Output>>,
    Complete,
    GeoJson {
        form: Lossless::Concise,
        geometry_property: None,
    }
);
entity_page!(
    Feature<SimplifiedEntity>,
    Complete,
    GeoJson {
        form: Simplified,
        geometry_property: None,
    }
);

impl<T> PageRequest<'_, T> {
    /// The HTTP request for the page, asking for `media_type`, on the page's own tenant, and aimed at the snapshot of
    /// `scope` or else at the page's own.
    pub(crate) fn request_accepting(&self, endpoint: &Endpoint, scope: RequestScope<'_>, media_type: NgsiLdMediaType) -> Result<Request<Bytes>, RequestError> {
        let scope = RequestScope::on_tenant(self.tenant).on_snapshot(self.snapshot).on_snapshot(scope.snapshot());
        self.parts(media_type).into_request(endpoint, scope)
    }

    /// The request: a GET of the link, asking for `media_type`, with the headers of any other query.
    fn parts(&self, media_type: NgsiLdMediaType) -> RequestParts {
        RequestParts {
            method: Method::GET,
            // The parts own their target; a URL is small next to the request it becomes.
            target: Target::Url(self.url.clone()),
            body: None,
            accept: Some(media_type),
            entity_map: None,
        }
        .with_entity_map(self.entity_map)
    }
}
