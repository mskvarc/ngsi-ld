//! Walking every page of a query (NGSI-LD clause 6.3.10).

use crate::{
    client::{Answering, Client, ClientError, OutcomeOnly, Transport},
    http::{
        Page,
        headers::NgsiLdEntityMap,
        operation::{Operation, PageRequest},
    },
    model::Tenant,
    seal::Seal,
    snapshot::SnapshotId,
};
use std::mem;
use url::Url;

/// Where a walk through the pages stands; `A` is the answer that holds the first page.
#[derive(Debug)]
enum Cursor<A> {
    /// The first page, already received and not yet handed out; boxed, as a page is much larger than a link.
    First(Box<A>),
    /// The page to request next; boxed, as its link, tenant, snapshot and map are much larger than nothing.
    Following(Box<FollowingPage>),
    /// No page is left, or a request failed.
    Finished,
}

/// The page to request next, on the tenant and snapshot and with the entity map of the query.
#[derive(Debug)]
struct FollowingPage {
    /// The `next` link of the page handed out last.
    url: Url,
    /// The tenant of the query; `None` is the default tenant.
    tenant: Option<Tenant>,
    /// The snapshot the query was aimed at.
    snapshot: Option<SnapshotId>,
    /// The entity map of the query, if it uses one (clause 4.3.6.7).
    entity_map: Option<NgsiLdEntityMap>,
}

impl<A> Cursor<A> {
    /// The cursor after `page` is handed out.
    fn after<I>(page: &Page<I>) -> Cursor<A> {
        // The page goes to the caller, so the cursor keeps its own copy of the link, the tenant, the snapshot and the
        // map.
        page.next().map_or(Cursor::Finished, |url| {
            Cursor::Following(Box::new(FollowingPage {
                url: url.clone(),
                tenant: page.tenant().cloned(),
                snapshot: page.snapshot().cloned(),
                entity_map: page.following_entity_map().cloned(),
            }))
        })
    }
}

/// Every page of a query, from [`Client::pages`]: the first page as received, then each page its predecessor's `next`
/// link leads to, requested on the tenant and snapshot and with the entity map of the query (clauses 4.3.6.7 and
/// 6.3.22), until a page has no `next` link.
///
/// A failed request ends the walk: [`next`](Self::next) returns that error once and `None` afterwards.
///
/// A client from [`Client::with_heads`] walks the pages in [`Answered`](crate::client::Answered) form:
/// every page comes with the status and head of its response.
#[derive(Debug)]
pub struct Pages<'c, T, I, M: Answering = OutcomeOnly> {
    /// The client the following pages are requested through.
    client: &'c Client<T, M>,
    /// Where the walk stands.
    cursor: Cursor<M::Answer<Page<I>>>,
}

impl<'c, T, I, M: Answering> Pages<'c, T, I, M> {
    /// The walk that starts with `first`.
    pub(crate) fn new(client: &'c Client<T, M>, first: M::Answer<Page<I>>) -> Pages<'c, T, I, M> {
        Pages {
            client,
            cursor: Cursor::First(Box::new(first)),
        }
    }
}

impl<T: Transport, I: Send, M: Answering> Pages<'_, T, I, M>
where
    for<'a> PageRequest<'a, I>: Operation<Outcome = Page<I>> + Sync,
{
    /// The following page: the first one, then one request per page; `None` when no page is left.
    ///
    /// # Errors
    ///
    /// Yields [`ClientError`] when a page cannot be requested or read; the walk ends there.
    pub async fn next(&mut self) -> Option<Result<M::Answer<Page<I>>, ClientError<T::Error>>> {
        let page = match mem::replace(&mut self.cursor, Cursor::Finished) {
            Cursor::First(page) => Ok(*page),
            Cursor::Following(following) => {
                let FollowingPage {
                    url,
                    tenant,
                    snapshot,
                    entity_map,
                } = *following;
                self.client
                    .send(&PageRequest::new(&url, tenant.as_ref(), snapshot.as_ref(), entity_map.as_ref()))
                    .await
            }
            Cursor::Finished => return None,
        };
        if let Ok(page) = &page {
            self.cursor = Cursor::after(M::outcome(page));
        }
        Some(page)
    }

    /// The items of every page, in order.
    ///
    /// # Errors
    ///
    /// Returns the first [`ClientError`] met; the items read before it are dropped.
    pub async fn try_collect(mut self) -> Result<Vec<I>, ClientError<T::Error>> {
        let mut items = Vec::new();
        while let Some(page) = self.next().await {
            items.extend(M::split(Seal::new(), page?).0.into_items());
        }
        Ok(items)
    }
}
