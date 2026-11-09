//! Receives the notifications a broker sends: one change to a vehicle in the normalized, concise and simplified forms,
//! a deleted attribute and the deleted vehicle, and the notifications of a context source registered and removed.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_CALLBACK_HOST=host.docker.internal cargo run --example notifications_live`.

use anyhow::Context;
use axum::{
    Router,
    body,
    extract::{Path, Request, State},
    routing::post,
};
use http::StatusCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::NotificationDelivery,
    model::{Entity, EntityFragment, EntityId, Lossless, Output, Representation, SimplifiedEntity},
    registration::{CSourceNotification, CSourceRegistration, RegistrationInfo, TriggerReason},
    subscription::{Endpoint, Notification, NotificationParams, NotificationTrigger, Subscription},
};
use std::{env, slice, time::Duration};
use tokio::{net::TcpListener, sync::mpsc, time};
use url::Url;

/// A notification as received: the path it was sent to, which names the form of the subscription, and its body.
type Received = (String, String);

/// Hands the body of a notification sent to `/{form}` to the channel; `400` when the request is not a notification.
async fn receive(State(sender): State<mpsc::Sender<Received>>, Path(form): Path<String>, request: Request) -> StatusCode {
    let (head, body) = request.into_parts();
    let Ok(body) = body::to_bytes(body, 1 << 20).await else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(delivery) = NotificationDelivery::read(http::Request::from_parts(head, body)) else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(text) = delivery.text() else {
        return StatusCode::BAD_REQUEST;
    };
    match sender.send((form, text.to_owned())).await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Waits for the next notification, for at most ten seconds.
async fn next(received: &mut mpsc::Receiver<Received>) -> anyhow::Result<Received> {
    time::timeout(Duration::from_secs(10), received.recv())
        .await
        .context("no notification within ten seconds")?
        .context("the receiver stopped")
}

/// Prints what each notified vehicle reports: a deleted vehicle carries only `deletedAt`, and a deleted attribute is
/// the NGSI-LD Null.
fn report(vehicles: &[Entity<Output>]) {
    for vehicle in vehicles {
        if let Some(deleted_at) = &vehicle.output.deleted_at
            && vehicle.attributes.is_empty()
        {
            println!("  {} was deleted at {deleted_at}", vehicle.id);
            continue;
        }
        for (name, instances) in &vehicle.attributes {
            for instance in instances {
                if instance.is_deleted() {
                    println!("  {} lost {name}", vehicle.id);
                } else {
                    println!("  {} has {name} = {}", vehicle.id, instance.render(Lossless::Concise));
                }
            }
        }
    }
}

/// Reads an entity notification sent to the subscription of `form`.
fn read(form: &str, text: &str) -> anyhow::Result<()> {
    match form {
        "normalized" | "concise" => {
            let lossless = if form == "normalized" { Lossless::Normalized } else { Lossless::Concise };
            let notification = Notification::<Vec<Entity<Output>>>::parse(text, lossless)?;
            println!("{form} notification at {}:", notification.notified_at());
            report(notification.data());
        }
        "simplified" => {
            let notification = Notification::<Vec<SimplifiedEntity>>::parse(text)?;
            println!("{form} notification at {}:", notification.notified_at());
            for vehicle in notification.data() {
                println!("  {}", vehicle.render());
            }
        }
        other => println!("a notification sent to /{other}"),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let host = env::var("NGSI_LD_CALLBACK_HOST").context("set NGSI_LD_CALLBACK_HOST to the host under which the broker reaches this machine")?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let callback = format!("http://{host}:{}", listener.local_addr()?.port());
    let (sender, mut received) = mpsc::channel(16);
    let router = Router::new().route("/{form}", post(receive)).with_state(sender);
    tokio::spawn(async move { axum::serve(listener, router).await });

    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:N4001"));
    client.entities().batch_delete(slice::from_ref(&id)).send().await?;

    // The normalized subscription also hears about deletions; the other two only about changed attributes.
    let forms = [
        (
            "normalized",
            Representation::Normalized,
            &[
                NotificationTrigger::AttributeUpdated,
                NotificationTrigger::AttributeDeleted,
                NotificationTrigger::EntityDeleted,
            ][..],
        ),
        ("concise", Representation::Concise, &[NotificationTrigger::AttributeUpdated][..]),
        ("simplified", Representation::Simplified, &[NotificationTrigger::AttributeUpdated][..]),
    ];
    let mut subscriptions = Vec::new();
    for (form, representation, triggers) in forms {
        let endpoint = Endpoint::builder(format!("{callback}/{form}")).build()?;
        let subscription = Subscription::builder()
            .entities(["Vehicle"])
            .notification_triggers(triggers.iter().copied())
            .notification(NotificationParams::builder(endpoint).format(representation).build()?)
            .build()?;
        let created = client.subscriptions().create(&subscription).send().await?;
        subscriptions.push(created.id().context("the broker named no subscription in the Location of its answer")?);
    }

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:N4001"))
        .entity_type("Vehicle")
        .property("speed", 30)
        .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
        .build()?;
    client.entities().create(&vehicle).send().await?;
    let moving = EntityFragment::builder().property("speed", 55).build()?;
    client.entities().update_attributes(&id, &moving).send().await?;
    for _ in 0..3 {
        let (form, text) = next(&mut received).await?;
        read(&form, &text)?;
    }

    let parked = "isParked".parse()?;
    client.entities().delete_attribute(&id, &parked).send().await?;
    let (form, text) = next(&mut received).await?;
    read(&form, &text)?;
    client.entities().delete(&id).send().await?;
    let (form, text) = next(&mut received).await?;
    read(&form, &text)?;

    let endpoint = Endpoint::builder(format!("{callback}/sources")).build()?;
    let watch = Subscription::registration_builder()
        .entities(["Truck"])
        .notification(NotificationParams::builder(endpoint).build()?)
        .build()?;
    let created = client.registration_subscriptions().create(&watch).send().await?;
    let watch_id = created.id().context("the broker named no subscription in the Location of its answer")?;
    // Nobody listens at the source's endpoint, and nothing queries trucks while it is registered.
    let source = CSourceRegistration::builder("http://depot.example:1026")
        .information([RegistrationInfo::builder().entities(["Truck"]).build()?])
        .build()?;
    let registered = client.registrations().register(&source).send().await?;
    let source_id = registered.id().context("the broker named no registration in the Location of its answer")?;
    client.registrations().delete(&source_id).send().await?;
    for _ in 0..2 {
        let (_, text) = next(&mut received).await?;
        let notification = CSourceNotification::parse(&text)?;
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

    client.registration_subscriptions().delete(&watch_id).send().await?;
    for subscription in &subscriptions {
        client.subscriptions().delete(subscription).send().await?;
    }
    client.entities().batch_delete(slice::from_ref(&id)).send().await?;
    Ok(())
}
