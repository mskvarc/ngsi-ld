//! The MQTT notification message of clause 7.2.

use crate::{
    model::{DocumentError, Entity, Lossless, MemberError, NgsiLdMediaType, Output},
    registration::CSourceNotification,
    subscription::{MqttNotification, Notification, ReadErrorKind},
};
use pretty_assertions::assert_eq;
use std::convert::Infallible;

/// A notification of one vehicle.
const BODY: &str = r#"{"id":"urn:ngsi-ld:Notification:1","type":"Notification","subscriptionId":"urn:ngsi-ld:Subscription:1","notifiedAt":"2024-05-01T10:00:00Z","data":[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80}}]}"#;

/// The `Link` metadata of a notification in `application/json`.
const LINK: &str = r#"<https://example.org/context.jsonld>; rel=\"http://www.w3.org/ns/json-ld#context\"; type=\"application/ld+json\""#;

fn error(text: &str) -> DocumentError<ReadErrorKind<Infallible>> {
    MqttNotification::parse(text).unwrap_err()
}

#[test]
fn a_message_holds_its_metadata_and_its_notification() {
    let text = format!(r#"{{"metadata":{{"Content-Type":"application/json","Link":"{LINK}","X-Auth-Token":"secret"}},"body":{BODY}}}"#);
    let message = MqttNotification::parse(&text).unwrap();
    assert_eq!(message.content_type(), NgsiLdMediaType::Json);
    assert_eq!(message.receiver_info("X-Auth-Token"), Some("secret"));
    assert_eq!(message.metadata().len(), 3);
    assert_eq!(message.body(), BODY);
    let notification = Notification::<Vec<Entity<Output>>>::parse(message.body(), Lossless::Normalized).unwrap();
    assert_eq!(notification.data().len(), 1);
}

#[cfg(feature = "http")]
#[test]
fn the_link_names_the_context() {
    let text = format!(r#"{{"metadata":{{"Link":"{LINK}"}},"body":{BODY}}}"#);
    let message = MqttNotification::parse(&text).unwrap();
    assert_eq!(
        message.context().unwrap().map(|context| context.as_str().to_owned()),
        Some("https://example.org/context.jsonld".to_owned())
    );
}

#[test]
fn without_a_content_type_the_body_is_json_and_ld_json_is_read() {
    let plain = MqttNotification::parse(&format!(r#"{{"metadata":{{}},"body":{BODY}}}"#)).unwrap();
    assert_eq!((plain.content_type(), plain.link()), (NgsiLdMediaType::Json, None));
    let linked = MqttNotification::parse(&format!(r#"{{"metadata":{{"Content-Type":"application/ld+json"}},"body":{BODY}}}"#)).unwrap();
    assert_eq!(linked.content_type(), NgsiLdMediaType::JsonLd);
}

#[test]
fn a_registration_notification_travels_the_same_way() {
    let body = r#"{"id":"urn:ngsi-ld:Notification:2","type":"ContextSourceNotification","subscriptionId":"urn:ngsi-ld:Subscription:2","notifiedAt":"2024-05-01T10:00:00Z","data":[],"triggerReason":"newlyMatching"}"#;
    let message = MqttNotification::parse(&format!(r#"{{"metadata":{{}},"body":{body}}}"#)).unwrap();
    assert!(CSourceNotification::parse(message.body()).is_ok());
}

#[test]
fn a_malformed_message_is_refused() {
    let geo = error(&format!(r#"{{"metadata":{{"Content-Type":"application/geo+json"}},"body":{BODY}}}"#));
    assert_eq!(geo.path.to_string(), "metadata.Content-Type");
    assert!(matches!(
        geo.kind,
        ReadErrorKind::Member(MemberError::InvalidToken { member: "Content-Type", .. })
    ));
    assert!(matches!(
        error(r#"{"metadata":{}}"#).kind,
        ReadErrorKind::Member(MemberError::MissingMember { member: "body" })
    ));
    assert!(matches!(
        error(&format!(r#"{{"body":{BODY}}}"#)).kind,
        ReadErrorKind::Member(MemberError::MissingMember { member: "metadata" })
    ));
    let not_text = error(&format!(r#"{{"metadata":{{"X-Count":3}},"body":{BODY}}}"#));
    assert_eq!(not_text.path.to_string(), "metadata.X-Count");
    assert!(matches!(
        error(r#"{"metadata":{},"body":[]}"#).kind,
        ReadErrorKind::Member(MemberError::NotAnObject)
    ));
    assert!(matches!(
        error(&format!(r#"{{"metadata":{{}},"body":{BODY},"topic":"x"}}"#)).kind,
        ReadErrorKind::Member(MemberError::UnexpectedMember { .. })
    ));
}
