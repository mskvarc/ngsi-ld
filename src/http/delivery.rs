//! The receiving side of the HTTP notification binding (NGSI-LD clauses 6.3.8, 6.3.9 and 6.3.14): what a broker's
//! notification request carries besides the notification itself.

use crate::{
    http::headers::{ContentType, Link, NgsiLdTenant},
    model::{NgsiLdMediaType, Tenant},
};
use bytes::Bytes;
use headers::{Header, HeaderMapExt};
use http::{HeaderMap, HeaderName, HeaderValue, Method, Request, header::LINK};
use iri_rs::IriRefBuf;
use std::str::{self, Utf8Error};

/// Why a request is not a notification this crate can read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DeliveryError {
    /// The request is not a `POST`, which clause 6.3.8 makes every notification.
    #[error("a notification is sent with POST, not {method}")]
    NotPost {
        /// The method of the request.
        method: Method,
    },
    /// The `Content-Type` is missing, or is not `application/json`, `application/ld+json` or
    /// `application/geo+json` (clause 6.3.8).
    #[error("the notification has no Content-Type this crate can read")]
    ContentType,
    /// The `Link` header is not one this crate can read.
    #[error("the Link header of the notification cannot be read")]
    Link,
    /// The `NGSILD-Tenant` header is not a tenant (clause 6.3.14).
    #[error("the NGSILD-Tenant header of the notification cannot be read")]
    Tenant,
    /// The body is not UTF-8 text, so it cannot be JSON.
    #[error("the body of the notification is not UTF-8 text")]
    NotText(#[source] Utf8Error),
}

/// A notification request a broker sent (NGSI-LD clause 6.3.8): the media type of its body, the `@context` the
/// `Link` header names for an `application/json` body, the tenant of the subscription (`NGSILD-Tenant`, clause
/// 6.3.14), the custom headers the subscription's `receiverInfo` asked for, and the body.
///
/// The crate implements no server; a receiver that got a request from its own HTTP server reads it with
/// [`NotificationDelivery::read`], then the body with the parse method of the notification it subscribed to:
/// [`Notification`](crate::subscription::Notification) in the representation the subscription asked for,
/// [`CSourceNotification`](crate::registration::CSourceNotification) or
/// [`SnapshotNotification`](crate::snapshot::SnapshotNotification).
///
/// ```
/// use bytes::Bytes;
/// use headers::HeaderMapExt;
/// use http::{HeaderName, Request};
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::{http::{NotificationDelivery, headers::{ContentType, Link, NgsiLdTenant}}, model::{Output, Entity, NgsiLdMediaType, Lossless}, subscription::Notification};
///
/// // The subscription's `receiverInfo` asked for this header.
/// const AUTH_TOKEN: HeaderName = HeaderName::from_static("x-auth-token");
/// let mut request = Request::post("http://receiver.test/notify")
///     .header(AUTH_TOKEN, "secret")
///     .body(Bytes::from_static(br#"{"id":"urn:ngsi-ld:Notification:1","type":"Notification","subscriptionId":"urn:ngsi-ld:Subscription:1","notifiedAt":"2024-05-01T10:00:00Z","data":[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80}}]}"#))?;
/// let headers = request.headers_mut();
/// headers.typed_insert(ContentType::NgsiLd(NgsiLdMediaType::Json));
/// headers.typed_insert(Link::json_ld_context(IriRefBuf::new("https://example.org/context.jsonld")?)?);
/// headers.typed_insert(NgsiLdTenant::new("t1".parse()?)?);
/// let delivery = NotificationDelivery::read(request)?;
/// assert_eq!(delivery.media_type(), NgsiLdMediaType::Json);
/// assert_eq!(delivery.context().map(|context| context.as_str()), Some("https://example.org/context.jsonld"));
/// assert_eq!(delivery.tenant().map(AsRef::as_ref), Some("t1"));
/// assert_eq!(delivery.receiver_info(&AUTH_TOKEN).and_then(|value| value.to_str().ok()), Some("secret"));
/// let notification = Notification::<Vec<Entity<Output>>>::parse(delivery.text()?, Lossless::Normalized)?;
/// assert_eq!(notification.data().len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct NotificationDelivery {
    /// The media type of the body.
    media_type: NgsiLdMediaType,
    /// The `@context` the `Link` header names.
    context: Option<IriRefBuf>,
    /// The tenant of the subscription.
    tenant: Option<Tenant>,
    /// Every header, for the `receiverInfo` ones.
    headers: HeaderMap,
    /// The notification.
    body: Bytes,
}

impl NotificationDelivery {
    /// Reads a notification request.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryError`] when the request is not a `POST`, has no readable `Content-Type` of a notification,
    /// or carries a `Link` or `NGSILD-Tenant` header that cannot be read.
    pub fn read(request: Request<Bytes>) -> Result<NotificationDelivery, DeliveryError> {
        let (parts, body) = request.into_parts();
        if parts.method != Method::POST {
            return Err(DeliveryError::NotPost { method: parts.method });
        }
        let media_type = match parts.headers.typed_get::<ContentType>() {
            Some(ContentType::NgsiLd(media_type)) => media_type,
            Some(ContentType::MergePatch) | None => return Err(DeliveryError::ContentType),
        };
        let context = context_of(&parts.headers)?;
        let tenant = if parts.headers.contains_key(NgsiLdTenant::name()) {
            Some(
                parts
                    .headers
                    .typed_try_get::<NgsiLdTenant>()
                    .ok()
                    .flatten()
                    .ok_or(DeliveryError::Tenant)?
                    .into_tenant(),
            )
        } else {
            None
        };
        Ok(NotificationDelivery {
            media_type,
            context,
            tenant,
            headers: parts.headers,
            body,
        })
    }

    /// The media type of the body: the subscription's `endpoint.accept`, `application/json` by default (clause
    /// 6.3.8).
    #[must_use]
    pub const fn media_type(&self) -> NgsiLdMediaType {
        self.media_type
    }

    /// The `@context` the `Link` header names (clause 6.3.8): present with an `application/json` body, and with an
    /// `application/geo+json` one when the subscription's `receiverInfo` prefers `body=json`; otherwise the context
    /// is in the body.
    #[must_use]
    pub const fn context(&self) -> Option<&IriRefBuf> {
        self.context.as_ref()
    }

    /// The tenant of the subscription (`NGSILD-Tenant`, clause 6.3.14); `None` for the default tenant.
    #[must_use]
    pub const fn tenant(&self) -> Option<&Tenant> {
        self.tenant.as_ref()
    }

    /// The header the subscription's `receiverInfo` named `name` (clause 6.3.8): the broker sends each pair as a
    /// header whose name is the pair's key.
    #[must_use]
    pub fn receiver_info(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// The body as received.
    #[must_use]
    pub const fn body(&self) -> &Bytes {
        &self.body
    }

    /// The body as text, for the parse method of the notification.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryError::NotText`] when the body is not UTF-8.
    pub fn text(&self) -> Result<&str, DeliveryError> {
        str::from_utf8(&self.body).map_err(DeliveryError::NotText)
    }
}

/// The `@context` a `Link` header names, when there is one (clause 6.3.8).
fn context_of(headers: &HeaderMap) -> Result<Option<IriRefBuf>, DeliveryError> {
    if !headers.contains_key(LINK) {
        return Ok(None);
    }
    let link = headers.typed_try_get::<Link>().ok().flatten().ok_or(DeliveryError::Link)?;
    // The context is kept beyond the header it was read from.
    Ok(link.context_urls().next().cloned())
}

/// The `@context` a `Link` field value names, when there is one: the `Link` member of an MQTT notification's
/// `metadata` (clause 7.2) has the syntax of the HTTP header.
pub(crate) fn context_of_link(value: &str) -> Result<Option<IriRefBuf>, DeliveryError> {
    let mut headers = HeaderMap::new();
    headers.insert(LINK, HeaderValue::from_str(value).map_err(|_| DeliveryError::Link)?);
    context_of(&headers)
}
