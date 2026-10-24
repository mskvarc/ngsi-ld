//! Context source notifications.

use crate::{
    model::MemberError,
    registration::{CSourceNotification, RegistrationStatus, TriggerReason, tests::ENDPOINT},
    subscription::{DocumentType, ReadErrorKind},
};
use pretty_assertions::assert_eq;

/// A context source notification with `trigger_reason` and `data`, and `extra` members after `notifiedAt`.
fn notification(trigger_reason: &str, data: &str, extra: &str) -> String {
    format!(
        r#"{{"id":"urn:ngsi-ld:Notification:2","type":"ContextSourceNotification","subscriptionId":"urn:ngsi-ld:Subscription:registrations","notifiedAt":"2024-05-01T10:00:00Z",{extra}"triggerReason":"{trigger_reason}","data":{data}}}"#
    )
}

/// Two registrations, the second with members only a broker writes.
fn registrations() -> String {
    format!(
        r#"[{{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","type":"ContextSourceRegistration","information":[{{"entities":[{{"type":"Vehicle"}}]}}],"endpoint":"{ENDPOINT}"}},{{"id":"urn:ngsi-ld:ContextSourceRegistration:csr2","type":"ContextSourceRegistration","information":[{{"propertyNames":["speed"]}}],"endpoint":"{ENDPOINT}","status":"ok","timesSent":2}}]"#
    )
}

#[test]
fn a_notification_carries_the_matching_registrations_and_why() {
    let text = notification("newlyMatching", &registrations(), "");
    let read = CSourceNotification::parse(&text).unwrap();
    assert_eq!(read.render().to_string(), text);
    assert_eq!(read.trigger_reason(), TriggerReason::NewlyMatching);
    assert_eq!(read.subscription_id().as_str(), "urn:ngsi-ld:Subscription:registrations");
    assert_eq!(read.data().len(), 2);
    assert_eq!(read.data()[1].output().status, Some(RegistrationStatus::Ok));
    assert_eq!(read.into_data()[0].id().as_str(), "urn:ngsi-ld:ContextSourceRegistration:csr1");
}

#[test]
fn each_trigger_reason_round_trips_and_no_other() {
    for reason in ["newlyMatching", "updated", "noLongerMatching"] {
        let text = notification(reason, "[]", "");
        assert_eq!(CSourceNotification::parse(&text).unwrap().render().to_string(), text);
    }
    let error = CSourceNotification::parse(&notification("changed", "[]", "")).unwrap_err();
    assert_eq!(error.path.to_string(), "triggerReason");
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "triggerReason", .. })),
        "{error}"
    );
}

#[test]
fn the_reader_refuses_a_missing_or_foreign_member() {
    let without_reason = notification("updated", "[]", "").replace(r#""triggerReason":"updated","#, "");
    assert!(matches!(
        CSourceNotification::parse(&without_reason).unwrap_err().kind,
        ReadErrorKind::Member(MemberError::MissingMember { member: "triggerReason" })
    ));
    let entity_notification = notification("updated", "[]", "").replace("ContextSourceNotification", "Notification");
    assert!(matches!(
        CSourceNotification::parse(&entity_notification).unwrap_err().kind,
        ReadErrorKind::WrongType {
            expected: DocumentType::ContextSourceNotification,
            ..
        }
    ));
    let error = CSourceNotification::parse(&notification("updated", "{}", "")).unwrap_err();
    assert_eq!(error.path.to_string(), "data");
    assert!(matches!(error.kind, ReadErrorKind::Member(MemberError::NotAnArray)));
    let without_id = format!(r#"[{{"type":"ContextSourceRegistration","information":[{{"propertyNames":["speed"]}}],"endpoint":"{ENDPOINT}"}}]"#);
    let error = CSourceNotification::parse(&notification("updated", &without_id, "")).unwrap_err();
    assert_eq!(error.path.to_string(), "data.0");
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "id" })),
        "{error}"
    );
    let foreign = CSourceNotification::parse(&notification("updated", "[]", r#""foo":1,"#)).unwrap_err();
    assert!(
        matches!(foreign.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "foo"),
        "{foreign}"
    );
}

#[test]
fn a_json_ld_notification_keeps_its_context_last() {
    let text = notification("updated", "[]", "").replacen("]}", r#"],"@context":"https://example.org/context.jsonld"}"#, 1);
    let read = CSourceNotification::parse(&text).unwrap();
    assert!(read.context().is_some());
    assert_eq!(read.render().to_string(), text);
}
