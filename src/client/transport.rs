//! The one capability the client needs from an HTTP library: sending a request and returning the response.

use bytes::Bytes;
use http::{Request, Response};
use std::error::Error;

/// Sends HTTP requests. The [`http`](crate::http) binding builds every request and reads every response; a
/// transport only moves them.
///
/// It is `Sync` because the client's futures borrow it across the await, and they are `Send`. Implement it to
/// use any HTTP client, a test double, or a recorded exchange. The `reqwest` feature provides
/// `ReqwestTransport`.
///
/// ```
/// use bytes::Bytes;
/// use http::{Request, Response, StatusCode};
/// use ngsi_ld::client::Transport;
/// use std::convert::Infallible;
///
/// /// Answers every request with 204, for tests.
/// struct NoContent;
///
/// impl Transport for NoContent {
///     type Error = Infallible;
///
///     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
///         let mut response = Response::new(Bytes::new());
///         *response.status_mut() = StatusCode::NO_CONTENT;
///         Ok(response)
///     }
/// }
/// ```
pub trait Transport: Sync {
    /// Why a request could not be sent or its response not received.
    type Error: Error + Send + Sync + 'static;

    /// Sends `request` and returns the response, whatever its status.
    ///
    /// # Errors
    ///
    /// Returns [`Transport::Error`] when no response was received.
    fn send(&self, request: Request<Bytes>) -> impl Future<Output = Result<Response<Bytes>, Self::Error>> + Send;
}

// A borrowed transport sends as the transport itself does, so a client derived from another shares its connections.
// No derive forwards a trait to a reference.
impl<T: Transport> Transport for &T {
    type Error = T::Error;

    fn send(&self, request: Request<Bytes>) -> impl Future<Output = Result<Response<Bytes>, Self::Error>> + Send {
        (**self).send(request)
    }
}
