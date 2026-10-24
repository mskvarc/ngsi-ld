//! Create, Update, Retrieve, Query and Delete Subscription (clauses 6.10 and 6.11), and the notifications a
//! subscription sends (clause 6.3.8).

use crate::{
    broker::{client, create, name, property_value, remove, unique},
    receiver::{notification, notification_uri, receiver},
};
use bytes::Bytes;
use http::header;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::{CreateSubscription, ResultsCount},
    model::{Entity, EntityFragment, EntityId, ErrorType, Input, Lossless, NgsiLdMediaType, Output, Representation, SimplifiedEntity, SimplifiedValue, Value},
    query::q,
    subscription::{Activation, Endpoint, Notification, NotificationParams, Subscription, SubscriptionFragment, SubscriptionId, SubscriptionStatus},
};
use pretty_assertions::assert_eq;
use wiremock::Request;

/// A subscription to the entities of type `entity_type`, watching `speed`, notifying `uri` in `format`.
fn subscription(id: Option<&SubscriptionId>, entity_type: &str, uri: &str, format: Representation) -> Subscription<Input> {
    let endpoint = Endpoint::builder(uri).accept(NgsiLdMediaType::Json).build().unwrap();
    Subscription::builder()
        .maybe_id(id.cloned())
        .entities([entity_type])
        .watched_attributes(["speed"])
        .notification(NotificationParams::builder(endpoint).format(format).build().unwrap())
        .build()
        .unwrap()
}

/// A subscription id unique to the calling test.
fn subscription_id(label: &str) -> SubscriptionId {
    format!("urn:ngsi-ld:Subscription:{}", unique(label)).parse().unwrap()
}

/// Deletes the subscription `id` if it is still there, so a test leaves nothing behind.
async fn unsubscribe(client: &Client<ReqwestTransport>, id: &SubscriptionId) {
    match client.subscriptions().delete(id).send().await {
        Ok(()) => {}
        Err(error) => assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_subscription_is_created_retrieved_updated_and_deleted() {
    let client = client();
    let id = subscription_id("lifecycle");
    let entity_type = unique("SubscribedVehicle");
    let sent = subscription(Some(&id), &entity_type, "http://notify.test/lifecycle", Representation::Normalized);
    let created = client.subscriptions().create(&sent).send().await;
    let retrieved = client.subscriptions().retrieve(&id).send().await;
    let pause = SubscriptionFragment::builder()
        .activation(Activation::Paused)
        .q(q::attr("speed").gt(50).build().unwrap())
        .build()
        .unwrap();
    let updated = client.subscriptions().update(&id, &pause).send().await;
    let after_update = client.subscriptions().retrieve(&id).send().await;
    let deleted = client.subscriptions().delete(&id).send().await;
    let after_delete = client.subscriptions().retrieve(&id).send().await.unwrap_err();
    unsubscribe(&client, &id).await;

    assert_eq!(created.unwrap().id(), Some(id.clone()));
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id(), &id);
    assert_eq!(retrieved.entities(), sent.entities());
    assert_eq!(retrieved.watched_attributes(), sent.watched_attributes());
    assert_eq!(retrieved.notification().endpoint().uri(), sent.notification().endpoint().uri());
    assert_eq!(retrieved.output().status, Some(SubscriptionStatus::Active));
    updated.unwrap();
    let after_update = after_update.unwrap();
    assert_eq!(after_update.activation(), Activation::Paused);
    assert_eq!(after_update.output().status, Some(SubscriptionStatus::Paused));
    assert_eq!(after_update.q().map(ToString::to_string).as_deref(), Some("speed>50"));
    deleted.unwrap();
    assert_eq!(after_delete.error_type(), Some(&ErrorType::ResourceNotFound), "{after_delete:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn the_notified_attributes_are_retrieved_as_an_array() {
    let client = client();
    let id = subscription_id("notifiedAttributes");
    let endpoint = Endpoint::builder("http://notify.test/attributes").build().unwrap();
    let sent = Subscription::builder()
        .id(id.clone())
        .entities([unique("AttributesVehicle")])
        .notification(NotificationParams::builder(endpoint).attributes(["speed"]).build().unwrap())
        .build()
        .unwrap();
    let created = client.subscriptions().create(&sent).send().await;
    let retrieved = client.subscriptions().retrieve(&id).send().await;
    unsubscribe(&client, &id).await;
    created.unwrap();
    // Known deviation, Table 5.2.14.1-1: `attributes` is an array of names, but the one name of this subscription
    // is returned as a bare string (`"attributes": "speed"`), which the reader refuses.
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.notification().attributes(), sent.notification().attributes());
}

/// Creates two subscriptions of the calling test and returns their ids.
async fn subscribe_twice(client: &Client<ReqwestTransport>, label: &str) -> [SubscriptionId; 2] {
    let ids = [subscription_id(&format!("{label}A")), subscription_id(&format!("{label}B"))];
    let entity_type = unique(label);
    for id in &ids {
        let sent = subscription(Some(id), &entity_type, "http://notify.test/pages", Representation::Normalized);
        client.subscriptions().create(&sent).send().await.unwrap();
    }
    ids
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_subscriptions_follows_the_next_links_to_every_subscription() {
    let client = client();
    let ids = subscribe_twice(&client, "pages").await;
    let first = client.subscriptions().query().limit(1).send().await;
    let every = match first {
        Ok(first) => client.pages(first).try_collect().await,
        Err(error) => Err(error),
    };
    for id in &ids {
        unsubscribe(&client, id).await;
    }
    let every = every.unwrap();
    for id in &ids {
        // Known deviation, clauses 5.5.9 and 6.3.10: a page of one subscription carries no `next` link however many
        // subscriptions follow it, so only the first page is reached.
        assert!(
            every.iter().any(|subscription| subscription.id() == id),
            "Query Subscriptions never listed {id}"
        );
    }
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_subscriptions_with_limit_zero_and_count_returns_the_total_and_no_subscription() {
    let client = client();
    let ids = subscribe_twice(&client, "count").await;
    let page = client.subscriptions().query().limit(0).count(ResultsCount::Include).send().await;
    for id in &ids {
        unsubscribe(&client, id).await;
    }
    // Known deviation, clause 6.3.13: the answer is 200 with an empty body, which is not JSON, and without
    // `NGSILD-Results-Count`.
    let page = page.unwrap();
    assert!(page.items().is_empty(), "{page:?}");
    assert!(page.total().is_some_and(|total| total >= 2), "{page:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_subscription_without_an_id_is_named_by_the_location() {
    let client = client();
    let sent = subscription(None, &unique("UnnamedVehicle"), "http://notify.test/unnamed", Representation::Normalized);
    let created = client.subscriptions().create(&sent).send().await.unwrap();
    let id = created.id();
    let retrieved = match &id {
        Some(id) => Some(client.subscriptions().retrieve(id).send().await),
        None => None,
    };
    if let Some(id) = &id {
        unsubscribe(&client, id).await;
    }
    let id = id.unwrap_or_else(|| panic!("the location names no subscription: {created:?}"));
    assert_eq!(retrieved.unwrap().unwrap().id(), &id);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn creating_a_subscription_twice_is_already_exists() {
    let client = client();
    let id = subscription_id("twice");
    let sent = subscription(Some(&id), &unique("TwiceVehicle"), "http://notify.test/twice", Representation::Normalized);
    let first = client.subscriptions().create(&sent).send().await;
    let second = client.subscriptions().create(&sent).send().await;
    unsubscribe(&client, &id).await;
    first.unwrap();
    let error = second.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::AlreadyExists), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn creating_a_subscription_without_notification_is_bad_request_data() {
    let client = client();
    let id = subscription_id("noNotification");
    let entity_type = unique("UnnotifiedVehicle");
    let sent = subscription(Some(&id), &entity_type, "http://notify.test/none", Representation::Normalized);
    let body = format!(r#"{{"id":"{id}","type":"Subscription","entities":[{{"type":"{entity_type}"}}]}}"#);
    let request = client.unchecked(&CreateSubscription { subscription: &sent }).unwrap().body(Bytes::from(body));
    let outcome = client.send_unchecked(request).await;
    unsubscribe(&client, &id).await;
    let error = outcome.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::BadRequestData), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn updating_or_deleting_a_missing_subscription_is_resource_not_found() {
    let client = client();
    let id = subscription_id("missing");
    let pause = SubscriptionFragment::builder().activation(Activation::Paused).build().unwrap();
    let updated = client.subscriptions().update(&id, &pause).send().await.unwrap_err();
    let deleted = client.subscriptions().delete(&id).send().await.unwrap_err();
    assert_eq!(updated.error_type(), Some(&ErrorType::ResourceNotFound), "{updated:?}");
    assert_eq!(deleted.error_type(), Some(&ErrorType::ResourceNotFound), "{deleted:?}");
}

/// Subscribes to a vehicle of a type of its own in `format`, sets its speed to 55, and returns the body and the
/// request of the notification that reports it, read by `read`, with the ids of the vehicle and the subscription.
async fn notified_speed_change<T>(label: &str, format: Representation, read: impl Fn(&str) -> Option<T>) -> (T, Request, EntityId, SubscriptionId) {
    let client = client();
    let receiver = receiver().await;
    let entity_type = unique(label);
    let vehicle_id: EntityId = format!("urn:ngsi-ld:Vehicle:{entity_type}").parse().unwrap();
    let vehicle = Entity::builder()
        .id(vehicle_id.as_str())
        .entity_type(entity_type.as_str())
        .property("speed", 0)
        .build()
        .unwrap();
    create(&client, &vehicle).await;
    let id = subscription_id(label);
    let created = client
        .subscriptions()
        .create(&subscription(Some(&id), &entity_type, &notification_uri(&receiver), format))
        .send()
        .await;
    let faster = EntityFragment::builder().property("speed", 55).build().unwrap();
    let updated = client.entities().update_attributes(&vehicle_id, &faster).send().await;
    let found = if created.is_ok() && updated.is_ok() {
        Some(notification(&receiver, read).await)
    } else {
        None
    };
    unsubscribe(&client, &id).await;
    remove(&client, &vehicle_id).await;
    created.unwrap();
    updated.unwrap();
    let (body, request) = found
        .unwrap()
        .unwrap_or_else(|received| panic!("no matching notification arrived; received {received:#?}"));
    (body, request, vehicle_id, id)
}

/// Asserts the HTTP binding of a notification under `application/json`: that media type and a `Link` header
/// (clause 6.3.8).
fn assert_json_notification(request: &Request) {
    let content_type = request.headers.get(&header::CONTENT_TYPE).map(|value| value.to_str().unwrap());
    assert!(content_type.is_some_and(|value| value.starts_with("application/json")), "{content_type:?}");
    assert!(request.headers.get(&header::LINK).is_some(), "{:?}", request.headers);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_normalized_notification_reports_the_new_speed() {
    let (notification, request, vehicle_id, id) = notified_speed_change("normalizedNotification", Representation::Normalized, |body| {
        Notification::<Vec<Entity<Output>>>::parse(body, Lossless::Normalized)
            .ok()
            .filter(|notification| {
                notification
                    .data()
                    .iter()
                    .any(|vehicle| property_value(vehicle, "speed") == Some(&Value::from(55)))
            })
    })
    .await;
    assert_eq!(notification.subscription_id(), &id);
    assert_eq!(notification.data()[0].id, vehicle_id);
    assert_json_notification(&request);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_simplified_notification_reports_the_new_speed() {
    let speed = name("speed");
    // Known deviation, clauses 5.2.14 and 4.5.4: a subscription whose `format` is `simplified` is notified in
    // normalized form (`"speed": {"type": "Property", "value": 55}`), so no simplified notification arrives.
    let (notification, request, vehicle_id, id) = notified_speed_change("simplifiedNotification", Representation::Simplified, |body| {
        Notification::<Vec<SimplifiedEntity>>::parse(body).ok().filter(|notification| {
            notification
                .data()
                .iter()
                .any(|vehicle| vehicle.attributes.get(&speed) == Some(&SimplifiedValue::Value(Value::from(55))))
        })
    })
    .await;
    assert_eq!(notification.subscription_id(), &id);
    assert_eq!(notification.data()[0].id, vehicle_id);
    assert_json_notification(&request);
}
