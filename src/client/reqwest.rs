//! [`Transport`] over `reqwest` (feature `reqwest`).

use crate::client::Transport;
use bytes::Bytes;
use http::{Request, Response};
use std::mem;

/// Why `reqwest` could not send a request or receive its response.
#[derive(Debug, thiserror::Error)]
#[error("the HTTP exchange failed")]
pub struct ReqwestError(#[from] reqwest::Error);

/// Sends requests with a `reqwest::Client`.
///
/// The crate enables no `reqwest` features: TLS, HTTP/2 and proxies are the caller's choice, through
/// `reqwest`'s own features in the caller's manifest. The response keeps `reqwest`'s extensions, so what the caller
/// enabled there stays readable from the response head: for instance the peer certificate, with
/// `reqwest::ClientBuilder::tls_info(true)`, as `reqwest::tls::TlsInfo`.
///
/// ```
/// use ngsi_ld::client::ReqwestTransport;
///
/// let transport = ReqwestTransport::new(reqwest::Client::builder().build()?);
/// # let _ = transport;
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    /// The underlying client, which pools connections.
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// Uses `client`.
    #[must_use]
    pub const fn new(client: reqwest::Client) -> ReqwestTransport {
        ReqwestTransport { client }
    }
}

impl Transport for ReqwestTransport {
    type Error = ReqwestError;

    async fn send(&self, request: Request<Bytes>) -> Result<Response<Bytes>, ReqwestError> {
        let mut response = self.client.execute(reqwest::Request::try_from(request)?).await?;
        let status = response.status();
        let version = response.version();
        let headers = mem::take(response.headers_mut());
        let extensions = mem::take(response.extensions_mut());
        let mut decoded = Response::new(response.bytes().await?);
        *decoded.status_mut() = status;
        *decoded.version_mut() = version;
        *decoded.headers_mut() = headers;
        *decoded.extensions_mut() = extensions;
        Ok(decoded)
    }
}
