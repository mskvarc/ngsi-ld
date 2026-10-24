//! The subscription operations over the client (clauses 6.10 and 6.11).

use crate::{
    fixtures::client,
    links::{context_link, link_field},
};
use http::header;
use iri_rs::IriRefBuf;
use ngsi_ld::{
    client::ClientError,
    http::{
        RequestError,
        headers::{LinkValue, RelationType},
        operation::{ArgumentError, SystemAttributes},
    },
    model::{APPLICATION_JSON, DateTime, Input},
    subscription::{Activation, Subscription, SubscriptionFragment, SubscriptionId},
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// `S`.
const S: &str = "urn:ngsi-ld:Subscription:mySubscription";

/// A subscription to every vehicle with the mandatory members only, as JSON.
fn vehicles_json() -> Value {
    json!({
        "type": "Subscription",
        "entities": [{"type": "Vehicle"}],
        "notification": {"endpoint": {"uri": "http://my.endpoint.org/notify"}}
    })
}

/// `vehicles_json` as a subscription.
fn vehicles() -> Subscription<Input> {
    Subscription::parse(&vehicles_json().to_string()).unwrap()
}

/// A retrieved subscription with the id `id`.
fn retrieved(id: &str) -> Value {
    json!({
        "id": id,
        "type": "Subscription",
        "entities": [{"type": "Vehicle"}],
        "notification": {"endpoint": {"uri": "http://my.endpoint.org/notify"}, "status": "ok"},
        "status": "active"
    })
}

#[tokio::test]
async fn create_sends_the_subscription_and_reads_its_id_from_the_location() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/subscriptions"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(vehicles_json()))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, format!("/ngsi-ld/v1/subscriptions/{S}").as_str()))
        .expect(1)
        .mount(&server)
        .await;
    let created = client(&server.uri()).subscriptions().create(&vehicles()).send().await.unwrap();
    assert_eq!(created.id(), Some(S.parse::<SubscriptionId>().unwrap()));
}

#[tokio::test]
async fn query_subscriptions_walks_every_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/subscriptions"))
        .and(query_param("offset", "1"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([retrieved("urn:ngsi-ld:Subscription:second")])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/subscriptions"))
        .and(query_param("limit", "1"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    &header::LINK,
                    &link_field([LinkValue::builder(IriRefBuf::new("?limit=1&offset=1").unwrap())
                        .rel(RelationType::Next)
                        .media_type(APPLICATION_JSON)
                        .build()]),
                )
                .set_body_json(json!([retrieved("urn:ngsi-ld:Subscription:first")])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let first = client.subscriptions().query().limit(1).send().await.unwrap();
    let every = client.pages(first).try_collect().await.unwrap();
    let ids: Vec<&str> = every.iter().map(|subscription| subscription.id().as_str()).collect();
    assert_eq!(ids, ["urn:ngsi-ld:Subscription:first", "urn:ngsi-ld:Subscription:second"]);
}

#[tokio::test]
async fn retrieve_update_and_delete_send_the_exact_requests() {
    let server = MockServer::start().await;
    let target = format!("/ngsi-ld/v1/subscriptions/{S}");
    Mock::given(method("GET"))
        .and(path(target.as_str()))
        .and(query_param("options", "sysAttrs"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(retrieved(S)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(target.as_str()))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(json!({
            "id": S,
            "type": "Subscription",
            "isActive": false,
            "expiresAt": "2024-05-01T10:00:00Z"
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(target.as_str()))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id: SubscriptionId = S.parse().unwrap();
    let subscription = client
        .subscriptions()
        .retrieve(&id)
        .system_attributes(SystemAttributes::Include)
        .send()
        .await
        .unwrap();
    assert_eq!(subscription.id(), &id);
    let pause = SubscriptionFragment::builder()
        .activation(Activation::Paused)
        .expires_at("2024-05-01T10:00:00Z".parse::<DateTime>().unwrap())
        .build()
        .unwrap();
    client.subscriptions().update(&id, &pause).send().await.unwrap();
    client.subscriptions().delete(&id).send().await.unwrap();
}

#[tokio::test]
async fn a_query_with_a_limit_of_zero_and_no_count_is_refused_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(0)
        .mount(&server)
        .await;
    let error = client(&server.uri()).subscriptions().query().limit(0).send().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::ZeroLimitWithoutCount))),
        "{error:?}"
    );
}
