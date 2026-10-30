//! One page of a query's results (NGSI-LD clauses 5.5.9, 6.3.10 and 6.3.13).

use crate::{
    http::{
        Coverage,
        ResponseContext,
        ResponseError,
        headers::{NgsiLdEntityMap, NgsiLdResultsCount, NgsiLdWarnings, OptionalHeader, RelationType, SpecVersion, link::header::read_line},
        response::{Success, classify},
        result_headers::ResultHeaders,
    },
    model::{Context, Tenant},
    snapshot::SnapshotId,
};
use bytes::Bytes;
use http::{HeaderValue, Response, StatusCode, header::LINK};
use url::Url;

/// One page of results.
///
/// A page holds the items and the links to the neighbouring pages (NGSI-LD clause 6.3.10). It also holds the total when
/// the query asked for it (clause 6.3.13), the location of the entity map the broker used when the query asked for one
/// (clause 6.4.3.2), the version the broker amended the payload to when it answered 203 (clause 4.3.6.8), and the
/// `@context` of a `GeoJSON` `FeatureCollection` (clause 4.5.16.3).
///
/// It holds, too, the warnings the broker attached when a distributed part of the query was served from a cache or
/// behaved abnormally (clause 6.3.17), and, for the temporal evolution of entities, the part of the requested time
/// range a 206 Partial Content covers (clause 6.3.10).
///
/// The links are resolved against the URL of the request that returned the page, the default context of a link (RFC
/// 8288 section 3.2), so they can be requested as they are. The page remembers the tenant that request addressed, the
/// snapshot it was aimed at and the entity map the query used: the pages it links belong to the same query, so
/// their requests address that tenant, aim at that snapshot (clause 6.3.22) and name that map, which keeps the
/// pagination consistent across distributed sources (clause 4.3.6.7).
///
/// The `Link` and `Preference-Applied` headers never fail a page. A `Link` line holding a link-value that cannot be
/// read, or a target that cannot be resolved, is kept in [`unread_links`](Self::unread_links), and every readable
/// link-value of it is still used. A `Preference-Applied` without a readable `ngsi-ld` version is ignored, as RFC
/// 7240 makes preferences advisory; the 203 status alone still says the payload was amended.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    /// The items of this page.
    items: Vec<T>,
    /// The next page.
    next: Option<Url>,
    /// The previous page.
    prev: Option<Url>,
    /// The first page.
    first: Option<Url>,
    /// The last page.
    last: Option<Url>,
    /// The number of matching items over all pages.
    total: Option<u64>,
    /// The entity map the request for this page asked the broker to use.
    requested_entity_map: Option<NgsiLdEntityMap>,
    /// The `@context` of the body as a whole: a `GeoJSON` `FeatureCollection`'s.
    context: Option<Context>,
    /// The tenant the request for this page addressed; `None` is the default tenant.
    tenant: Option<Tenant>,
    /// The snapshot the request for this page was aimed at.
    snapshot: Option<SnapshotId>,
    /// The `Link` field lines that held a link-value which could not be read or resolved.
    unread_links: Vec<HeaderValue>,
    /// The entity map, version, warnings and coverage the headers carry.
    headers: ResultHeaders,
    /// The success status the broker answered.
    status: StatusCode,
}

impl<T> Page<T> {
    /// The success status the broker answered: 200 OK, 203 Non-Authoritative Information when it amended the payload
    /// to another version (clause 4.3.6.8), 206 Partial Content when a temporal query covers part of its range
    /// (clause 6.3.10), or 201 Created for the entity map a query created (clauses 5.14.4 and 5.14.5).
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// The items of this page.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Takes the items.
    #[must_use]
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    /// The URL of the next page, if there is one.
    #[must_use]
    pub const fn next(&self) -> Option<&Url> {
        self.next.as_ref()
    }

    /// The URL of the previous page, if there is one.
    #[must_use]
    pub const fn prev(&self) -> Option<&Url> {
        self.prev.as_ref()
    }

    /// The URL of the first page, when the broker links it (`rel="first"`, RFC 8288 section 6.2.2).
    #[must_use]
    pub const fn first(&self) -> Option<&Url> {
        self.first.as_ref()
    }

    /// The URL of the last page, when the broker links it (`rel="last"`, RFC 8288 section 6.2.2).
    #[must_use]
    pub const fn last(&self) -> Option<&Url> {
        self.last.as_ref()
    }

    /// The number of matching items over all pages, when the query asked for the count.
    #[must_use]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }

    /// The location of the entity map the broker used, which it returns when the query asked for one
    /// (`NGSILD-EntityMap`, clause 6.4.3.2).
    #[must_use]
    pub const fn entity_map(&self) -> &OptionalHeader<NgsiLdEntityMap> {
        &self.headers.entity_map
    }

    /// The entity map the requests for the pages this page links name (`NGSILD-EntityMap`, clause 4.3.6.7): the map
    /// the broker said it used for this page, or else the map the request for this page named; `None` when the
    /// query uses no entity map.
    #[must_use]
    pub fn following_entity_map(&self) -> Option<&NgsiLdEntityMap> {
        self.headers.entity_map.valid().or(self.requested_entity_map.as_ref())
    }

    /// The `@context` the body carried as a whole: the one of a `GeoJSON` `FeatureCollection` (Table 5.2.30-1), present
    /// when the response carries its context in the body (clause 6.3.6). `None` for a JSON array of entities, which
    /// carries none of its own.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// The NGSI-LD version the broker says the payload conforms to (`Preference-Applied`, clause 6.3.6).
    #[must_use]
    pub const fn preference_applied(&self) -> Option<&SpecVersion> {
        self.headers.preference_applied.as_ref()
    }

    /// The warnings the broker attached when a distributed part of the query was served from a cache or behaved
    /// abnormally (`NGSILD-Warning`, clause 6.3.17); [`OptionalHeader::Absent`] when it attached none.
    #[must_use]
    pub const fn warnings(&self) -> &OptionalHeader<NgsiLdWarnings> {
        &self.headers.warnings
    }

    /// The tenant the request for this page addressed, `None` for the default tenant (clause 6.3.14).
    #[must_use]
    pub const fn tenant(&self) -> Option<&Tenant> {
        self.tenant.as_ref()
    }

    /// The snapshot the request for this page was aimed at (`NGSILD-Snapshot`, clause 6.3.22), which the
    /// pages it links are aimed at too.
    #[must_use]
    pub const fn snapshot(&self) -> Option<&SnapshotId> {
        self.snapshot.as_ref()
    }

    /// The `Link` field lines, as received, that held a link-value which could not be read or resolved. When it is
    /// not empty, a missing [`next`](Self::next) may be a link the broker sent but this page could not read.
    #[must_use]
    pub fn unread_links(&self) -> &[HeaderValue] {
        &self.unread_links
    }

    /// The part of the requested time range the page covers: [`Coverage::Partial`] when a Query Temporal
    /// Evolution answered 206 Partial Content (clause 6.3.10), [`Coverage::Whole`] for every other page.
    /// [`Coverage::following_query`] is the temporal query of the rest of the range.
    #[must_use]
    pub const fn coverage(&self) -> &Coverage {
        &self.headers.coverage
    }

    /// Reads a page whose status is one of `statuses`: its items and the `@context` of the body as a whole with
    /// `read`, and its links and headers.
    pub(crate) fn decode(
        context: &ResponseContext,
        response: Response<Bytes>,
        statuses: &[StatusCode],
        read: impl FnOnce(&Success) -> Result<(Vec<T>, Option<Context>), ResponseError>,
    ) -> Result<Page<T>, ResponseError> {
        let response = classify(response)?;
        if !statuses.contains(&response.status()) {
            return Err(response.unexpected());
        }
        let (items, body_context) = read(&response)?;
        let links = PageLinks::read(&response, context);
        Ok(Page {
            items,
            next: links.next,
            prev: links.prev,
            first: links.first,
            last: links.last,
            total: response.header::<NgsiLdResultsCount>()?.map(NgsiLdResultsCount::count),
            // The page outlives the request it answers, so it keeps its own copy of the map.
            requested_entity_map: context.entity_map().cloned(),
            context: body_context,
            // The page outlives the request it answers, so it keeps its own copy of the tenant.
            tenant: context.tenant().cloned(),
            // Likewise for the snapshot.
            snapshot: context.snapshot().cloned(),
            unread_links: links.unread,
            headers: ResultHeaders::read(&response),
            status: response.status(),
        })
    }
}

/// The pagination links of a response (clause 6.3.10).
#[derive(Debug, Default)]
struct PageLinks {
    /// The first resolvable `next` link.
    next: Option<Url>,
    /// The first resolvable `prev` link.
    prev: Option<Url>,
    /// The first resolvable `first` link.
    first: Option<Url>,
    /// The first resolvable `last` link.
    last: Option<Url>,
    /// The lines holding a link-value that could not be read or resolved.
    unread: Vec<HeaderValue>,
}

impl PageLinks {
    /// Reads every `Link` line, keeping the first `next`, `prev`, `first` and `last` targets, resolved against the
    /// request URL.
    fn read(response: &Success, context: &ResponseContext) -> PageLinks {
        response.headers().get_all(LINK).iter().fold(PageLinks::default(), |mut links, line| {
            let read = read_line(line);
            let mut unresolved = 0_usize;
            for value in &read.values {
                for (rel, slot) in [
                    (RelationType::Next, &mut links.next),
                    (RelationType::Prev, &mut links.prev),
                    (RelationType::First, &mut links.first),
                    (RelationType::Last, &mut links.last),
                ] {
                    if slot.is_none() && value.has_rel(&rel) {
                        match context.resolve(value.target().as_str()) {
                            Some(url) => *slot = Some(url),
                            None => unresolved += 1,
                        }
                    }
                }
            }
            if read.unread + unresolved > 0 {
                // The header map keeps the line; the page outlives the response, so it keeps its own handle.
                links.unread.push(line.clone());
            }
            links
        })
    }
}
