//! Reads the notification bodies a receiver is sent, with no HTTP feature: notifications of entities in normalized,
//! concise and simplified form, context source notifications, and an MQTT notification message.
//!
//! Run it with `cargo run --example notifications`.

use ngsi_ld::{
    model::{Entity, Lossless, Output, SimplifiedEntity},
    registration::{CSourceNotification, TriggerReason},
    subscription::{MqttNotification, Notification},
};

/// A normalized notification: the speed of one vehicle changed and its `isParked` was deleted, and another vehicle
/// was deleted.
const NORMALIZED: &str = r#"{
    "id": "urn:ngsi-ld:Notification:1",
    "type": "Notification",
    "subscriptionId": "urn:ngsi-ld:Subscription:fleet",
    "notifiedAt": "2024-05-01T10:00:00Z",
    "data": [
        {"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": {"type": "Property", "value": 55, "unitCode": "KMH"}, "isParked": "urn:ngsi-ld:null"},
        {"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "deletedAt": "2024-05-01T09:59:58Z"}
    ]
}"#;

/// The change of the first vehicle, notified to a subscription whose `format` is `concise`.
const CONCISE: &str = r#"{
    "id": "urn:ngsi-ld:Notification:2",
    "type": "Notification",
    "subscriptionId": "urn:ngsi-ld:Subscription:fleetConcise",
    "notifiedAt": "2024-05-01T10:00:00Z",
    "data": [{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": {"value": 55, "unitCode": "KMH"}, "isParked": "urn:ngsi-ld:null"}]
}"#;

/// The new speed and the deleted `isParked`, notified to a subscription whose `format` is `keyValues`.
const KEY_VALUES: &str = r#"{
    "id": "urn:ngsi-ld:Notification:3",
    "type": "Notification",
    "subscriptionId": "urn:ngsi-ld:Subscription:speeds",
    "notifiedAt": "2024-05-01T10:00:00Z",
    "data": [{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": 55, "isParked": "urn:ngsi-ld:null"}]
}"#;

/// The first notification of a registration subscription: one context source holds vehicles.
const NEWLY_MATCHING: &str = r#"{
    "id": "urn:ngsi-ld:Notification:4",
    "type": "ContextSourceNotification",
    "subscriptionId": "urn:ngsi-ld:Subscription:sources",
    "notifiedAt": "2024-05-01T10:00:00Z",
    "triggerReason": "newlyMatching",
    "data": [{
        "id": "urn:ngsi-ld:ContextSourceRegistration:north",
        "type": "ContextSourceRegistration",
        "information": [{"entities": [{"type": "Vehicle"}]}],
        "endpoint": "http://north.example.org:1026/"
    }]
}"#;

/// The registration of that context source was deleted.
const NO_LONGER_MATCHING: &str = r#"{
    "id": "urn:ngsi-ld:Notification:5",
    "type": "ContextSourceNotification",
    "subscriptionId": "urn:ngsi-ld:Subscription:sources",
    "notifiedAt": "2024-05-01T11:00:00Z",
    "triggerReason": "noLongerMatching",
    "data": [{
        "id": "urn:ngsi-ld:ContextSourceRegistration:north",
        "type": "ContextSourceRegistration",
        "information": [{"entities": [{"type": "Vehicle"}]}],
        "endpoint": "http://north.example.org:1026/"
    }]
}"#;

/// An MQTT notification message: the `Content-Type` and `Link` of Table 7.2-2, an entry of the subscription's
/// `receiverInfo`, and a normalized notification as the body.
const MQTT: &str = r#"{
    "metadata": {
        "Content-Type": "application/json",
        "Link": "<https://example.org/fleet-context.jsonld>; rel=\"http://www.w3.org/ns/json-ld#context\"; type=\"application/ld+json\"",
        "X-Fleet": "north"
    },
    "body": {
        "id": "urn:ngsi-ld:Notification:6",
        "type": "Notification",
        "subscriptionId": "urn:ngsi-ld:Subscription:fleetOverMqtt",
        "notifiedAt": "2024-05-01T10:00:00Z",
        "data": [{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": {"type": "Property", "value": 55}}]
    }
}"#;

/// Prints what each notified entity reports: a deleted entity carries only `deletedAt`, and a deleted attribute is
/// the NGSI-LD Null.
fn report(entities: &[Entity<Output>]) {
    for entity in entities {
        if let Some(deleted_at) = &entity.output.deleted_at
            && entity.attributes.is_empty()
        {
            println!("  {} was deleted at {deleted_at}", entity.id);
            continue;
        }
        for (name, instances) in &entity.attributes {
            for instance in instances {
                if instance.is_deleted() {
                    println!("  {} lost {name}", entity.id);
                } else {
                    println!("  {} has {name} = {}", entity.id, instance.render(Lossless::Concise));
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    // The receiver knows the format of its subscription, so it names what `data` holds.
    let normalized = Notification::<Vec<Entity<Output>>>::parse(NORMALIZED, Lossless::Normalized)?;
    println!(
        "notification {} of {} at {}:",
        normalized.id(),
        normalized.subscription_id(),
        normalized.notified_at()
    );
    report(normalized.data());
    let concise = Notification::<Vec<Entity<Output>>>::parse(CONCISE, Lossless::Concise)?;
    println!("notification {} of {}:", concise.id(), concise.subscription_id());
    report(concise.data());

    let simplified = Notification::<Vec<SimplifiedEntity>>::parse(KEY_VALUES)?;
    println!("notification {} of {}:", simplified.id(), simplified.subscription_id());
    for entity in simplified.data() {
        println!("  {}", entity.render());
        for (name, value) in &entity.attributes {
            // The simplified form writes a deleted attribute as the NGSI-LD Null too.
            if value.is_deleted() {
                println!("  {} lost {name}", entity.id);
            }
        }
    }

    for text in [NEWLY_MATCHING, NO_LONGER_MATCHING] {
        let notification = CSourceNotification::parse(text)?;
        let reason = notification.trigger_reason();
        for registration in notification.data() {
            match reason {
                TriggerReason::NewlyMatching | TriggerReason::Updated => {
                    println!("{reason}: {} is served at {}", registration.id(), registration.endpoint());
                }
                TriggerReason::NoLongerMatching => println!("{reason}: {} is no longer served", registration.id()),
            }
        }
    }

    // The metadata of an MQTT message stands for the HTTP headers.
    let message = MqttNotification::parse(MQTT)?;
    println!("an MQTT message of {:?}", message.content_type());
    if let Some(fleet) = message.receiver_info("X-Fleet") {
        println!("  for the fleet {fleet}");
    }
    if let Some(link) = message.link() {
        println!("  its @context linked as {link}");
    }
    let notification = Notification::<Vec<Entity<Output>>>::parse(message.body(), Lossless::Normalized)?;
    println!("notification {} of {}:", notification.id(), notification.subscription_id());
    report(notification.data());
    Ok(())
}
