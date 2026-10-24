//! Register, Update, Retrieve, Query and Delete Context Source Registration (clauses 6.8 and 6.9), the Context
//! Source Registration Subscription operations (clauses 6.12 and 6.13), and the notifications such a subscription
//! sends (clauses 5.11.7 and 6.3.9).

use crate::{
    broker::{client, unique},
    receiver::{notification, notification_uri, receiver},
};
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{DateTime, ErrorType, Input},
    registration::{CSourceNotification, CSourceRegistration, RegistrationFragment, RegistrationId, RegistrationInfo, TriggerReason},
    subscription::{Activation, Endpoint, NotificationParams, RegistrationSubscription, Subscription, SubscriptionFragment, SubscriptionId},
};
use pretty_assertions::assert_eq;

/// A registration of a context source at `http://csource.test:1026/` providing the `speed` of the entities of
/// type `entity_type`.
fn speed_source(id: Option<&RegistrationId>, entity_type: &str) -> CSourceRegistration<Input> {
    CSourceRegistration::builder("http://csource.test:1026")
        .maybe_id(id.cloned())
        .information([RegistrationInfo::builder().entities([entity_type]).property_names(["speed"]).build().unwrap()])
        .build()
        .unwrap()
}

/// A registration id unique to the calling test.
fn registration_id(label: &str) -> RegistrationId {
    format!("urn:ngsi-ld:ContextSourceRegistration:{}", unique(label)).parse().unwrap()
}

/// A subscription id unique to the calling test.
fn subscription_id(label: &str) -> SubscriptionId {
    format!("urn:ngsi-ld:Subscription:{}", unique(label)).parse().unwrap()
}

/// A Context Source Registration Subscription to the registrations of `entity_type`, notifying `uri`.
fn registration_watch(id: &SubscriptionId, entity_type: &str, uri: &str) -> Subscription<Input, RegistrationSubscription> {
    Subscription::registration_builder()
        .id(id.clone())
        .entities([entity_type])
        .notification(NotificationParams::builder(Endpoint::builder(uri).build().unwrap()).build().unwrap())
        .build()
        .unwrap()
}

/// Deletes the registration `id` if it is still there, so a test leaves nothing behind.
async fn unregister(client: &Client<ReqwestTransport>, id: &RegistrationId) {
    match client.registrations().delete(id).send().await {
        Ok(()) => {}
        Err(error) => assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}"),
    }
}

/// Deletes the registration subscription `id` if it is still there.
async fn unsubscribe(client: &Client<ReqwestTransport>, id: &SubscriptionId) {
    match client.registration_subscriptions().delete(id).send().await {
        Ok(()) => {}
        Err(error) => assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}"),
    }
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_registration_is_registered_retrieved_updated_and_deleted() {
    let client = client();
    let id = registration_id("lifecycle");
    let sent = speed_source(Some(&id), &unique("RegisteredVehicle"));
    let registered = client.registrations().register(&sent).send().await;
    let retrieved = client.registrations().retrieve(&id).send().await;
    let describe = RegistrationFragment::builder()
        .description("Vehicle speeds")
        .expires_at("2099-01-01T00:00:00Z".parse::<DateTime>().unwrap())
        .build()
        .unwrap();
    let updated = client.registrations().update(&id, &describe).send().await;
    let after_update = client.registrations().retrieve(&id).send().await;
    let deleted = client.registrations().delete(&id).send().await;
    let after_delete = client.registrations().retrieve(&id).send().await.unwrap_err();
    unregister(&client, &id).await;

    // Table 5.2.9-1 makes `contextSourceAlias` an IETF RFC 7230 pseudonym, a token. Observed deviation: the broker
    // writes the endpoint URL (`http://my.csource.org:1026/`) as the alias of a registration that sent none, which is
    // no token, so the registration it returns cannot be read.
    assert_eq!(registered.unwrap().id(), Some(id.clone()));
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id(), &id);
    assert_eq!(retrieved.information(), sent.information());
    assert_eq!(retrieved.endpoint(), sent.endpoint());
    updated.unwrap();
    let after_update = after_update.unwrap();
    assert_eq!(after_update.description(), Some("Vehicle speeds"));
    assert_eq!(after_update.expires_at().map(ToString::to_string).as_deref(), Some("2099-01-01T00:00:00Z"));
    deleted.unwrap();
    assert_eq!(after_delete.error_type(), Some(&ErrorType::ResourceNotFound), "{after_delete:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_registrations_by_type_finds_the_registration() {
    let client = client();
    let id = registration_id("query");
    let entity_type = unique("QueriedSource");
    client.registrations().register(&speed_source(Some(&id), &entity_type)).send().await.unwrap();
    let page = client.registrations().query().entity_type(entity_type.as_str()).send().await;
    let other = client.registrations().query().entity_type(unique("UnregisteredSource").as_str()).send().await;
    unregister(&client, &id).await;
    // Table 5.2.9-1 makes `contextSourceAlias` an IETF RFC 7230 pseudonym, a token. Observed deviation: the broker
    // writes the endpoint URL (`http://my.csource.org:1026/`) as the alias of a registration that sent none, which is
    // no token, so the registration it returns cannot be read.
    let ids: Vec<RegistrationId> = page.unwrap().items().iter().map(|registration| registration.id().clone()).collect();
    assert_eq!(ids, [id]);
    assert_eq!(other.unwrap().items(), []);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_registration_without_an_id_is_named_by_the_location() {
    let client = client();
    let created = client
        .registrations()
        .register(&speed_source(None, &unique("UnnamedSource")))
        .send()
        .await
        .unwrap();
    let id = created.id();
    let retrieved = match &id {
        Some(id) => Some(client.registrations().retrieve(id).send().await),
        None => None,
    };
    if let Some(id) = &id {
        unregister(&client, id).await;
    }
    let id = id.unwrap_or_else(|| panic!("the location names no registration: {created:?}"));
    // Table 5.2.9-1 makes `contextSourceAlias` an IETF RFC 7230 pseudonym, a token. Observed deviation: the broker
    // writes the endpoint URL (`http://my.csource.org:1026/`) as the alias of a registration that sent none, which is
    // no token, so the registration it returns cannot be read.
    assert_eq!(retrieved.unwrap().unwrap().id(), &id);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn registering_twice_is_already_exists() {
    let client = client();
    let id = registration_id("twice");
    let sent = speed_source(Some(&id), &unique("TwiceSource"));
    let first = client.registrations().register(&sent).send().await;
    let second = client.registrations().register(&sent).send().await;
    unregister(&client, &id).await;
    first.unwrap();
    let error = second.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::AlreadyExists), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn updating_or_deleting_a_missing_registration_is_resource_not_found() {
    let client = client();
    let id = registration_id("missing");
    let describe = RegistrationFragment::builder().description("missing").build().unwrap();
    let updated = client.registrations().update(&id, &describe).send().await.unwrap_err();
    let deleted = client.registrations().delete(&id).send().await.unwrap_err();
    assert_eq!(updated.error_type(), Some(&ErrorType::ResourceNotFound), "{updated:?}");
    assert_eq!(deleted.error_type(), Some(&ErrorType::ResourceNotFound), "{deleted:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_registration_subscription_is_created_retrieved_updated_and_deleted() {
    let client = client();
    let id = subscription_id("registrationLifecycle");
    let sent = registration_watch(&id, &unique("WatchedSource"), "http://notify.test/registrations");
    let created = client.registration_subscriptions().create(&sent).send().await;
    let retrieved = client.registration_subscriptions().retrieve(&id).send().await;
    let pause = SubscriptionFragment::registration_builder().activation(Activation::Paused).build().unwrap();
    let updated = client.registration_subscriptions().update(&id, &pause).send().await;
    let after_update = client.registration_subscriptions().retrieve(&id).send().await;
    let listed = client.registration_subscriptions().query().send().await;
    let deleted = client.registration_subscriptions().delete(&id).send().await;
    let after_delete = client.registration_subscriptions().retrieve(&id).send().await.unwrap_err();
    unsubscribe(&client, &id).await;

    assert_eq!(created.unwrap().id(), Some(id.clone()));
    // Known deviation, Tables 5.2.12-1 and 5.2.14-2: the retrieved registration subscription carries
    // `lastFailure`, `lastNotification`, `timesFailed` and `timesSent` at its top level, where Table 5.2.12-1 defines
    // no such member (they belong to `notification`), with the timestamps written `"$1"`, so the reader refuses
    // `lastFailure`.
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id(), &id);
    assert_eq!(retrieved.entities(), sent.entities());
    updated.unwrap();
    assert_eq!(after_update.unwrap().activation(), Activation::Paused);
    assert!(listed.unwrap().items().iter().any(|subscription| subscription.id() == &id));
    deleted.unwrap();
    assert_eq!(after_delete.error_type(), Some(&ErrorType::ResourceNotFound), "{after_delete:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_registration_subscription_without_an_id_is_named_by_the_location() {
    let client = client();
    let sent = Subscription::registration_builder()
        .entities([unique("UnnamedWatch")])
        .notification(
            NotificationParams::builder(Endpoint::builder("http://notify.test/unnamed").build().unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    // Known deviation, clause 5.11.2.4: a registration subscription without an `id` is refused with 409
    // AlreadyExists, "Subscription with id null exists", instead of being given an id by the broker.
    let created = client.registration_subscriptions().create(&sent).send().await.unwrap();
    let id = created.id();
    let retrieved = match &id {
        Some(id) => Some(client.registration_subscriptions().retrieve(id).send().await),
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
async fn a_registration_subscription_is_notified_of_a_newly_matching_registration() {
    let client = client();
    let receiver = receiver().await;
    let entity_type = unique("NotifiedSource");
    let subscription = subscription_id("registrationNotification");
    let registration = registration_id("registrationNotification");
    let created = client
        .registration_subscriptions()
        .create(&registration_watch(&subscription, &entity_type, &notification_uri(&receiver)))
        .send()
        .await;
    let registered = client.registrations().register(&speed_source(Some(&registration), &entity_type)).send().await;
    let found = if created.is_ok() && registered.is_ok() {
        Some(
            notification(&receiver, |body| {
                CSourceNotification::parse(body)
                    .ok()
                    .filter(|notification| notification.data().iter().any(|source| source.id() == &registration))
            })
            .await,
        )
    } else {
        None
    };
    unsubscribe(&client, &subscription).await;
    unregister(&client, &registration).await;
    created.unwrap();
    registered.unwrap();
    // Known deviation, clauses 5.11.7 and 6.3.9: registering a source whose information matches the subscription
    // sends no notification at all, newly matching or otherwise, while the receiver is reachable from the broker.
    let (notification, _) = found
        .unwrap()
        .unwrap_or_else(|received| panic!("no matching notification arrived; received {received:#?}"));
    assert_eq!(notification.subscription_id(), &subscription);
    assert_eq!(notification.trigger_reason(), TriggerReason::NewlyMatching);
}
