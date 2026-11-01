//! Walking the time ranges of a temporal answer: a broker that cannot answer at once answers 206 Partial Content
//! (NGSI-LD clause 6.3.10), or pages the attribute instances and links the following page (ETSI TS 104 176 clause
//! 6.4.7.3), and the client asks for the rest until the answer covers it all.

use crate::{
    client::{Answering, ClientError, Temporal, Transport},
    http::{
        Retrieved,
        operation::{IntervalRequest, LocalScopeOperation, Operation, RetrieveTemporalEvolution, TemporalSelection},
    },
    model::{Completeness, Lossless, Output, Tenant, evolution::TemporalEntity},
    query::Locality,
    seal::Seal,
};
use std::collections::HashSet;

impl<T: Transport, M: Answering> Temporal<'_, T, M> {
    /// Sends one Retrieve Temporal Evolution and returns its answer as it is.
    pub(super) async fn answer<O: LocalScopeOperation + Sync>(
        &self,
        locality: Locality,
        retrieve: O,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<O::Outcome>, ClientError<T::Error>> {
        self.route.send_with_locality(locality, &retrieve, tenant).await
    }

    /// Sends a Retrieve Temporal Evolution, then, while the answer is a 206 Partial Content with a readable
    /// `Content-Range`, the request for the rest of the range ([`Coverage::following_query`]), and joins the answers
    /// ([`TemporalEntity::absorb`], or [`TemporalEntity::absorb_last`] for a request with `lastN`, so each attribute
    /// keeps the last `lastN` instances of the whole range). The walk ends at an answer that covers the rest of the
    /// range, at one whose range cannot be read, and at one whose range does not end strictly inside the range asked
    /// for, since the broker made no progress; the coverage of the last answer is the coverage of the whole.
    ///
    /// The answer of a broker that pages the attribute instances links the following page: `intervalbefore` for a
    /// request with `lastN`, which pages in descending order, `intervalafter` otherwise (ETSI TS 104 176 clause
    /// 6.4.7.3). Each link is then requested and the answers are joined with every instance
    /// ([`TemporalEntity::absorb`]); that walk ends at an answer without the link and at a link already requested.
    ///
    /// [`Coverage::following_query`]: crate::http::Coverage::following_query
    pub(super) async fn whole_range<C: Completeness + Copy>(
        &self,
        locality: Locality,
        retrieve: RetrieveTemporalEvolution<'_, Lossless, C>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Retrieved<TemporalEntity<Output, C>>>, ClientError<T::Error>>
    where
        for<'r> RetrieveTemporalEvolution<'r, Lossless, C>: LocalScopeOperation + Operation<Outcome = Retrieved<TemporalEntity<Output, C>>> + Sync,
        for<'r> IntervalRequest<'r, TemporalEntity<Output, C>>: Operation<Outcome = Retrieved<TemporalEntity<Output, C>>> + Sync,
    {
        let (mut answer, mut last) = M::split(Seal::new(), self.answer(locality, retrieve, tenant).await?);
        let mut rest = answer.coverage().following_query(retrieve.selection);
        // Each rest lies strictly inside the range before it (`Coverage::following_query`), so the walk narrows the
        // range and ends whatever ranges the broker answers with.
        while let Some(query) = rest {
            let next = RetrieveTemporalEvolution {
                selection: TemporalSelection::Query(&query),
                ..retrieve
            };
            let (later, capture) = M::split(Seal::new(), self.answer(locality, next, tenant).await?);
            let following = later.coverage().following_query(next.selection);
            answer = answer.followed_by(later, retrieve.selection);
            // The joined answer comes with the head of the last response, the one that completed the range.
            last = capture;
            rest = following;
        }
        let order = InstanceOrder::of(retrieve.selection);
        let mut requested = HashSet::new();
        // The set outlives the answer each link is read from, so it keeps its own copy of each URL.
        while let Some(link) = order.following(&answer).filter(|link| requested.insert(link.url().clone())) {
            let (later, capture) = M::split(Seal::new(), self.route.send(&link, tenant).await?);
            answer = answer.followed_by_interval(later);
            last = capture;
        }
        Ok(M::answer(Seal::new(), answer, last))
    }
}

/// The order the broker pages the attribute instances in (ETSI TS 104 176 clause 6.4.7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstanceOrder {
    /// Ascending order of the timestamps: a request without `lastN`.
    Ascending,
    /// Descending order of the timestamps: a request with `lastN`.
    Descending,
}

impl InstanceOrder {
    /// The order the instances `selection` asks for are paged in.
    const fn of(selection: TemporalSelection<'_>) -> InstanceOrder {
        match selection.last_n() {
            Some(_) => InstanceOrder::Descending,
            None => InstanceOrder::Ascending,
        }
    }

    /// The request for the page that follows `answer` in this order, if the broker linked it.
    fn following<T>(self, answer: &Retrieved<T>) -> Option<IntervalRequest<'_, T>> {
        match self {
            InstanceOrder::Ascending => answer.interval_after_request(),
            InstanceOrder::Descending => answer.interval_before_request(),
        }
    }
}
