//! [`Transport`] over `reqwest` (feature `reqwest`).

use crate::client::Transport;
use bytes::Bytes;
use http::{Request, Response};
use std::mem;

/// Why `reqwest` could not send a request or receive its response.
#[derive(Debug, thiserror::Error)]
#[error("the HTTP exchange failed")]
pub struct ReqwestError(#[from] reqwest::Error);

/// Why [`ReqwestTransport::try_default`] could not build its `reqwest::Client`.
///
/// No TLS backend could be initialized, or the system's resolver configuration could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the reqwest client could not be built")]
pub struct ReqwestBuildError(#[source] reqwest::Error);

/// Sends requests with a `reqwest::Client`.
///
/// [`try_default`](ReqwestTransport::try_default) builds a client with `reqwest`'s default settings;
/// [`new`](ReqwestTransport::new) takes one the caller configured. The crate's `reqwest-rustls` or `reqwest-native-tls`
/// feature picks the TLS backend, and `reqwest-http2` and `reqwest-system-proxy` turn on `reqwest`'s `http2` and
/// `system-proxy`; the default features are `reqwest-rustls`, `reqwest-http2` and `reqwest-system-proxy`. Without a
/// TLS backend, the transport can only reach `http` URLs. The response keeps `reqwest`'s extensions, so what the
/// caller enabled there stays readable from the response head: for instance the peer certificate, with
/// `reqwest::ClientBuilder::tls_info(true)`, as `reqwest::tls::TlsInfo`.
///
/// ```
/// use ngsi_ld::client::ReqwestTransport;
///
/// let transport = ReqwestTransport::try_default()?;
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

    /// Uses a `reqwest::Client` with `reqwest`'s default settings.
    ///
    /// # Errors
    ///
    /// Returns [`ReqwestBuildError`] when the TLS backend cannot be initialized or the system's resolver
    /// configuration cannot be read.
    pub fn try_default() -> Result<ReqwestTransport, ReqwestBuildError> {
        reqwest::Client::builder().build().map(ReqwestTransport::new).map_err(ReqwestBuildError)
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
