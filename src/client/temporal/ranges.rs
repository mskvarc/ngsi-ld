//! Walking the time ranges of a temporal answer (NGSI-LD clause 6.3.10): a broker that cannot answer at once
//! answers 206 Partial Content, and the client asks for the rest of the range until the answer covers it all.

use crate::{
    client::{Answering, ClientError, Temporal, Transport},
    http::{
        Retrieved,
        operation::{LocalScopeOperation, Operation, RetrieveTemporalEvolution, TemporalSelection},
    },
    model::{Completeness, Lossless, Output, Tenant, evolution::TemporalEntity},
    query::Locality,
    seal::Seal,
};

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
    /// [`Coverage::following_query`]: crate::http::Coverage::following_query
    pub(super) async fn whole_range<C: Completeness + Copy>(
        &self,
        locality: Locality,
        retrieve: RetrieveTemporalEvolution<'_, Lossless, C>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Retrieved<TemporalEntity<Output, C>>>, ClientError<T::Error>>
    where
        for<'r> RetrieveTemporalEvolution<'r, Lossless, C>: LocalScopeOperation + Operation<Outcome = Retrieved<TemporalEntity<Output, C>>> + Sync,
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
        Ok(M::answer(Seal::new(), answer, last))
    }
}
