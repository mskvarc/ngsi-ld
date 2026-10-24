//! Reading a notification request a broker sent (clauses 6.3.8, 6.3.9 and 6.3.14).

use crate::links::context_link;
use bytes::Bytes;
use headers::Header;
use http::{
    HeaderName,
    Request,
    header::{CONTENT_TYPE, LINK},
};
use ngsi_ld::{
    http::{DeliveryError, NotificationDelivery, headers::NgsiLdTenant},
    model::{Entity, FeatureCollection, Lossless, NgsiLdMediaType, Output},
    subscription::Notification,
};
use pretty_assertions::assert_eq;

/// A notification of one vehicle.
const BODY: &str = r#"{"id":"urn:ngsi-ld:Notification:1","type":"Notification","subscriptionId":"urn:ngsi-ld:Subscription:1","notifiedAt":"2024-05-01T10:00:00Z","data":[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80}}]}"#;

/// The header the subscription's `receiverInfo` asks for.
const AUTH_TOKEN: HeaderName = HeaderName::from_static("x-auth-token");

/// A notification request with `headers` and `body`.
fn notification(headers: &[(&HeaderName, &str)], body: &'static str) -> Request<Bytes> {
    let mut builder = Request::post("http://receiver.test/notify");
    for &(name, value) in headers {
        builder = builder.header(name, value);
    }
    builder.body(Bytes::from_static(body.as_bytes())).unwrap()
}

#[test]
fn a_json_notification_names_its_context_tenant_and_receiver_headers() {
    let delivery = NotificationDelivery::read(notification(
        &[
            (&CONTENT_TYPE, "application/json"),
            (&LINK, &context_link().to_string()),
            (NgsiLdTenant::name(), "t1"),
            (&AUTH_TOKEN, "secret"),
        ],
        BODY,
    ))
    .unwrap();
    assert_eq!(delivery.media_type(), NgsiLdMediaType::Json);
    assert_eq!(
        delivery.context().map(ToString::to_string).as_deref(),
        Some("https://example.org/context.jsonld")
    );
    assert_eq!(delivery.tenant().map(AsRef::as_ref), Some("t1"));
    assert_eq!(delivery.receiver_info(&AUTH_TOKEN).map(|value| value.to_str().unwrap()), Some("secret"));
    let read = Notification::<Vec<Entity<Output>>>::parse(delivery.text().unwrap(), Lossless::Normalized).unwrap();
    assert_eq!(read.data().len(), 1);
}

#[test]
fn an_ld_json_notification_on_the_default_tenant_has_no_link() {
    let delivery = NotificationDelivery::read(notification(&[(&CONTENT_TYPE, "application/ld+json")], BODY)).unwrap();
    assert_eq!(
        (delivery.media_type(), delivery.context(), delivery.tenant()),
        (NgsiLdMediaType::JsonLd, None, None)
    );
}

#[test]
fn a_geo_json_notification_carries_a_feature_collection() {
    let body = r#"{"id":"urn:ngsi-ld:Notification:1","type":"Notification","subscriptionId":"urn:ngsi-ld:Subscription:1","notifiedAt":"2024-05-01T10:00:00Z","data":{"type":"FeatureCollection","features":[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":null,"properties":{"type":"Vehicle","speed":{"type":"Property","value":80}}}]}}"#;
    let delivery = NotificationDelivery::read(notification(&[(&CONTENT_TYPE, "application/geo+json")], body)).unwrap();
    assert_eq!(delivery.media_type(), NgsiLdMediaType::GeoJson);
    let read = Notification::<FeatureCollection<Entity<Output>>>::parse(delivery.text().unwrap(), Lossless::Normalized).unwrap();
    assert_eq!(read.data().features.len(), 1);
}

#[test]
fn a_request_that_is_no_notification_is_refused() {
    let get = Request::get("http://receiver.test/notify").body(Bytes::from_static(BODY.as_bytes())).unwrap();
    assert!(matches!(NotificationDelivery::read(get), Err(DeliveryError::NotPost { .. })));
    assert!(matches!(NotificationDelivery::read(notification(&[], BODY)), Err(DeliveryError::ContentType)));
    assert!(matches!(
        NotificationDelivery::read(notification(&[(&CONTENT_TYPE, "text/plain")], BODY)),
        Err(DeliveryError::ContentType)
    ));
    assert!(matches!(
        NotificationDelivery::read(notification(&[(&CONTENT_TYPE, "application/json"), (&LINK, "not a link")], BODY)),
        Err(DeliveryError::Link)
    ));
    assert!(matches!(
        NotificationDelivery::read(notification(&[(&CONTENT_TYPE, "application/json"), (NgsiLdTenant::name(), "")], BODY)),
        Err(DeliveryError::Tenant)
    ));
}
