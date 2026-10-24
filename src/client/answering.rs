//! What a call of a client returns: the outcome alone, or the outcome with the status and head of the response.

use crate::{http::ResponseHead, seal::Seal};
use bytes::Bytes;
use http::{Response, StatusCode};
use std::fmt::Debug;

/// The private supertrait that keeps `Answering` implemented by this crate alone.
mod sealed {
    /// Seals [`Answering`](super::Answering): the two modes are the crate's.
    pub trait Sealed {}
}

/// How a client answers a call.
///
/// [`OutcomeOnly`], the default, answers with the typed outcome alone. [`WithHead`], from
/// [`Client::with_heads`](crate::client::Client::with_heads), answers with the outcome, the status and the head of the
/// response in an [`Answered`].
///
/// The trait is sealed: the two modes are the only ones.
pub trait Answering: sealed::Sealed + Debug + Clone + Copy + Default + Send + Sync + 'static {
    /// What a call answers when the operation succeeds with `V`.
    type Answer<V>;
    /// What is kept of the response before the operation reads it.
    #[doc(hidden)]
    type Capture: Send;

    /// Keeps what the answer needs of `response` before the operation reads it.
    #[doc(hidden)]
    fn capture(seal: Seal, response: &Response<Bytes>) -> Self::Capture;

    /// The answer of a call whose operation succeeded with `outcome`.
    #[doc(hidden)]
    fn answer<V>(seal: Seal, outcome: V, capture: Self::Capture) -> Self::Answer<V>;

    /// Separates an answer into its outcome and what was kept of the response, for a call that joins the answers
    /// of several requests and answers with what was kept of the last one.
    #[doc(hidden)]
    fn split<V>(seal: Seal, answer: Self::Answer<V>) -> (V, Self::Capture);

    /// The outcome an answer holds.
    fn outcome<V>(answer: &Self::Answer<V>) -> &V;
}

/// The default mode: a call answers its typed outcome alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OutcomeOnly;

/// Calls answer with the typed outcome, the status and the head of the response.
///
/// It serves a caller that also reads a header the outcome does not carry (`Content-Type`, `ETag`, `Vary`,
/// `NGSILD-Tenant`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WithHead;

/// The typed outcome of a call, with the status and the head of its response.
///
/// ```
/// use http::StatusCode;
/// use ngsi_ld::{client::Answered, http::ResponseHead};
///
/// let answered = Answered::new(7_u8, StatusCode::OK, ResponseHead::default());
/// assert_eq!(answered.status, StatusCode::OK);
/// assert_eq!(answered.into_outcome(), 7);
/// ```
#[derive(Debug, Clone)]
pub struct Answered<V> {
    /// The typed outcome.
    pub outcome: V,
    /// The status of the response.
    pub status: StatusCode,
    /// The rest of the head of the response: version, headers and the transport's extensions.
    pub head: ResponseHead,
}

impl<V> Answered<V> {
    /// The outcome `outcome`, answered with `status` and `head`.
    #[must_use]
    pub const fn new(outcome: V, status: StatusCode, head: ResponseHead) -> Answered<V> {
        Answered { outcome, status, head }
    }

    /// Takes the outcome.
    #[must_use]
    pub fn into_outcome(self) -> V {
        self.outcome
    }
}

impl sealed::Sealed for OutcomeOnly {}
impl sealed::Sealed for WithHead {}

impl Answering for OutcomeOnly {
    type Answer<V> = V;
    type Capture = ();

    fn capture(_: Seal, _: &Response<Bytes>) {}

    fn answer<V>(_: Seal, outcome: V, (): ()) -> V {
        outcome
    }

    fn split<V>(_: Seal, answer: V) -> (V, ()) {
        (answer, ())
    }

    fn outcome<V>(answer: &V) -> &V {
        answer
    }
}

impl Answering for WithHead {
    type Answer<V> = Answered<V>;
    type Capture = (StatusCode, ResponseHead);

    fn capture(_: Seal, response: &Response<Bytes>) -> (StatusCode, ResponseHead) {
        (response.status(), ResponseHead::of(response))
    }

    fn answer<V>(_: Seal, outcome: V, (status, head): (StatusCode, ResponseHead)) -> Answered<V> {
        Answered::new(outcome, status, head)
    }

    fn split<V>(_: Seal, answer: Answered<V>) -> (V, (StatusCode, ResponseHead)) {
        (answer.outcome, (answer.status, answer.head))
    }

    fn outcome<V>(answer: &Answered<V>) -> &V {
        &answer.outcome
    }
}
