//! The requests as a broker forwards them to the source.

use bytes::Bytes;
use headers::HeaderMapExt;
use http::{Method, Request};
use ngsi_ld::{http::headers::ContentType, model::NgsiLdMediaType};

/// The root under which the source serves the API.
const ROOT: &str = "http://source.test/ngsi-ld/v1";

/// A request as it reaches the source: `method` on `target`, a path under [`ROOT`] with its query string, with a
/// JSON `body` when it has one.
pub(crate) fn forwarded(method: Method, target: &str, body: Option<&'static str>) -> anyhow::Result<Request<Bytes>> {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("{ROOT}{target}"))
        .body(body.map_or_else(Bytes::new, |text| Bytes::from_static(text.as_bytes())))?;
    if body.is_some() {
        request.headers_mut().typed_insert(ContentType::NgsiLd(NgsiLdMediaType::Json));
    }
    Ok(request)
}

/// The query string of `request`, empty when it has none.
pub(crate) fn query_string(request: &Request<Bytes>) -> &str {
    request.uri().query().unwrap_or_default()
}
