//! Keeps [`Operation`](crate::http::operation::Operation) closed to the operations of this crate.

use crate::http::{Endpoint, RequestError, request::scope::RequestScope};
use bytes::Bytes;
use http::Request;

/// Implemented by the operation types of this crate only.
pub trait Sealed {}

/// How an operation builds its request: through the pipeline of [`request`](crate::http::request), within
/// `scope`, which a wrapper of the operation widens instead of changing the request built.
pub trait BuildsRequest {
    /// Builds the HTTP request for the broker at `endpoint` within `scope`.
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError>;

    /// Builds the request the operation sends when it is aimed at a snapshot: its own, except for the queries and
    /// the purge, which on a snapshot need no restriction of the entities (clause 5.5.15).
    fn scoped_snapshot_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        self.scoped_request(endpoint, scope)
    }
}
