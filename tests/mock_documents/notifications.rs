//! The notifications a broker sends, built in typed form as a mock broker sends them to a receiver under test
//! (clauses 5.3.1, 5.3.2 and 5.3.4): build, write, read back, compare.

use iri_rs::iri;
use ngsi_ld::{
    model::{DateTime, Entity, Lossless, Output, SimplifiedEntity},
    registration::{CSourceNotification, CSourceRegistration, TriggerReason},
    snapshot::{ExecutionResultDetails, ExecutionStatus, SettledStatus, SnapshotNotification, SnapshotPriority},
    subscription::Notification,
};

/// A time.
fn at(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// A vehicle as a broker notifies it.
fn vehicle() -> Entity<Output> {
    Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", 55)
        .modified_at(at("2024-05-01T09:59:00Z"))
        .build()
        .unwrap()
}

#[test]
fn a_notification_of_entities_round_trips_in_both_forms() {
    let notification = Notification::builder()
        .id(iri!("urn:ngsi-ld:Notification:1"))
        .subscription_id(iri!("urn:ngsi-ld:Subscription:mySubscription"))
        .notified_at(at("2024-05-01T10:00:00Z"))
        .data(vec![vehicle()])
        .build();
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = notification.render(form).to_string();
        assert_eq!(Notification::<Vec<Entity<Output>>>::parse(&text, form).unwrap(), notification, "{text}");
    }
}

#[test]
fn a_notification_of_simplified_entities_round_trips() {
    let notification = Notification::builder()
        .id(iri!("urn:ngsi-ld:Notification:1"))
        .subscription_id(iri!("urn:ngsi-ld:Subscription:mySubscription"))
        .notified_at(at("2024-05-01T10:00:00Z"))
        .data(vec![SimplifiedEntity::from(&vehicle())])
        .build();
    let text = notification.render().to_string();
    assert_eq!(Notification::<Vec<SimplifiedEntity>>::parse(&text).unwrap(), notification);
}

#[test]
fn a_context_source_notification_round_trips() {
    let registration = CSourceRegistration::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","type":"ContextSourceRegistration","information":[{"entities":[{"type":"Vehicle"}]}],"endpoint":"http://my.csource.org:1026/"}"#,
    )
    .unwrap();
    let notification = CSourceNotification::builder()
        .id(iri!("urn:ngsi-ld:Notification:2"))
        .subscription_id(iri!("urn:ngsi-ld:Subscription:registrations"))
        .notified_at(at("2024-05-01T10:00:00Z"))
        .trigger_reason(TriggerReason::Updated)
        .data(vec![registration])
        .build();
    assert_eq!(CSourceNotification::parse(&notification.render().to_string()).unwrap(), notification);
}

#[test]
fn a_snapshot_notification_with_query_details_round_trips() {
    let notification = SnapshotNotification::builder()
        .id(iri!("urn:ngsi-ld:Notification:3"))
        .expires_at(at("2030-01-01T00:00:00Z"))
        .notified_at(at("2024-05-01T10:05:00Z"))
        .snapshot_id(iri!("urn:ngsi-ld:Snapshot:1"))
        .priority(SnapshotPriority::try_new(7).unwrap())
        .status(SettledStatus::Success)
        .queries_details(vec![ExecutionResultDetails::new(ExecutionStatus::Empty)])
        .temporal_queries_details(vec![ExecutionResultDetails::new(ExecutionStatus::Success)])
        .build();
    assert_eq!(SnapshotNotification::parse(&notification.render().to_string()).unwrap(), notification);
}
