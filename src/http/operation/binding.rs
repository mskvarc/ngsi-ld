//! The binding of an NGSI-LD operation to HTTP: how its request is built and how its response is read.

use crate::http::{Endpoint, RequestError, ResponseContext, ResponseError, operation::sealed, request::scope::RequestScope};
use bytes::Bytes;
use http::{Request, Response};

/// An NGSI-LD operation bound to HTTP: how its request is built and how its response is read.
///
/// The trait is sealed: the set of operations is the specification's.
pub trait Operation: sealed::Sealed + sealed::BuildsRequest {
    /// What a successful response yields.
    type Outcome;

    /// Builds the HTTP request for the broker at `endpoint`, on the tenant the endpoint addresses.
    ///
    /// # Errors
    ///
    /// Returns [`RequestError`] when the parameters are a combination the specification refuses, such as
    /// `limit=0` without `count` (clause 6.3.13).
    fn to_request(&self, endpoint: &Endpoint) -> Result<Request<Bytes>, RequestError> {
        self.scoped_request(endpoint, RequestScope::on_tenant(endpoint.tenant()))
    }

    /// Reads the broker's response to the request described by `context`.
    ///
    /// # Errors
    ///
    /// Returns [`ResponseError`] for every error response, and for a success response whose status or body is not
    /// one the operation defines.
    fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Self::Outcome, ResponseError>;
}
