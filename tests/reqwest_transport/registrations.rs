//! The context source registration operations and the Context Source Registration Subscription operations over the
//! client (clauses 6.8, 6.9, 6.12 and 6.13).

use crate::{
    fixtures::client,
    links::{context_link, link_field, link_value},
};
use http::header;
use ngsi_ld::{
    client::ClientError,
    http::{
        RequestError,
        headers::RelationType,
        operation::{ArgumentError, SystemAttributes},
    },
    model::{DateTime, Input},
    registration::{CSourceRegistration, RegistrationFragment, RegistrationId, RegistrationTypeMember},
    subscription::{RegistrationSubscription, Subscription, SubscriptionId},
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// `R`.
const R: &str = "urn:ngsi-ld:ContextSourceRegistration:csr1";

/// `RS`.
const RS: &str = "urn:ngsi-ld:Subscription:registrations";

/// A context source providing the `speed` of every vehicle, as JSON.
fn vehicle_source_json() -> Value {
    json!({
        "type": "ContextSourceRegistration",
        "information": [{"entities": [{"type": "Vehicle"}], "propertyNames": ["speed"]}],
        "endpoint": "http://my.csource.org:1026/"
    })
}

/// `vehicle_source_json` as a registration.
fn vehicle_source() -> CSourceRegistration<Input> {
    CSourceRegistration::parse(&vehicle_source_json().to_string()).unwrap()
}

/// A retrieved registration with the id `id`.
fn retrieved(id: &str) -> Value {
    json!({
        "id": id,
        "type": "ContextSourceRegistration",
        "information": [{"entities": [{"type": "Vehicle"}], "propertyNames": ["speed"]}],
        "endpoint": "http://my.csource.org:1026/",
        "status": "ok"
    })
}

/// A Context Source Registration Subscription for the inclusive registrations of vehicles, as JSON.
fn watch_json() -> Value {
    json!({
        "type": "Subscription",
        "entities": [{"type": "Vehicle"}],
        "csf": "mode==\"inclusive\"",
        "notification": {"endpoint": {"uri": "http://my.endpoint.org/registrations"}}
    })
}

#[tokio::test]
async fn register_sends_the_registration_and_reads_its_id_from_the_location() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/csourceRegistrations"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(vehicle_source_json()))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, format!("/ngsi-ld/v1/csourceRegistrations/{R}").as_str()))
        .expect(1)
        .mount(&server)
        .await;
    let created = client(&server.uri()).registrations().register(&vehicle_source()).send().await.unwrap();
    assert_eq!(created.id(), Some(R.parse::<RegistrationId>().unwrap()));
}

#[tokio::test]
async fn query_registrations_sends_its_parameters_and_walks_every_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/csourceRegistrations"))
        .and(query_param("offset", "1"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([retrieved("urn:ngsi-ld:ContextSourceRegistration:second")])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/csourceRegistrations"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("attrs", "speed"))
        .and(query_param("limit", "1"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    &header::LINK,
                    &link_field([link_value("?type=Vehicle&attrs=speed&limit=1&offset=1", RelationType::Next)]),
                )
                .set_body_json(json!([retrieved("urn:ngsi-ld:ContextSourceRegistration:first")])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let first = client
        .registrations()
        .query()
        .entity_type("Vehicle")
        .attrs(vec!["speed".parse().unwrap()])
        .limit(1)
        .send()
        .await
        .unwrap();
    let every = client.pages(first).try_collect().await.unwrap();
    let ids: Vec<&str> = every.iter().map(|registration| registration.id().as_str()).collect();
    assert_eq!(
        ids,
        ["urn:ngsi-ld:ContextSourceRegistration:first", "urn:ngsi-ld:ContextSourceRegistration:second"]
    );
}

#[tokio::test]
async fn retrieve_update_and_delete_a_registration() {
    let server = MockServer::start().await;
    let registration_path = format!("/ngsi-ld/v1/csourceRegistrations/{R}");
    Mock::given(method("GET"))
        .and(path(registration_path.as_str()))
        .and(query_param("options", "sysAttrs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(retrieved(R)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(registration_path.as_str()))
        .and(body_json(json!({"type": "ContextSourceRegistration", "expiresAt": "2030-01-01T00:00:00Z"})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(registration_path.as_str()))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id: RegistrationId = R.parse().unwrap();
    let registration = client
        .registrations()
        .retrieve(&id)
        .system_attributes(SystemAttributes::Include)
        .send()
        .await
        .unwrap();
    assert_eq!(registration.id(), &id);
    let postpone = RegistrationFragment::builder()
        .expires_at("2030-01-01T00:00:00Z".parse::<DateTime>().unwrap())
        .build()
        .unwrap();
    client.registrations().update(&id, &postpone).send().await.unwrap();
    client.registrations().delete(&id).send().await.unwrap();
}

#[tokio::test]
async fn an_update_sends_a_fragment_that_leaves_its_type_out() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(format!("/ngsi-ld/v1/csourceRegistrations/{R}").as_str()))
        .and(header(header::CONTENT_TYPE, "application/json"))
        .and(body_json(json!({"expiresAt": "2030-01-01T00:00:00Z"})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id: RegistrationId = R.parse().unwrap();
    let postpone = RegistrationFragment::builder()
        .type_member(RegistrationTypeMember::Omitted)
        .expires_at("2030-01-01T00:00:00Z".parse::<DateTime>().unwrap())
        .build()
        .unwrap();
    client.registrations().update(&id, &postpone).send().await.unwrap();
}

#[tokio::test]
async fn a_registration_subscription_is_created_and_retrieved_on_csource_subscriptions_with_its_csf() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/csourceSubscriptions"))
        .and(body_json(watch_json()))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, format!("/ngsi-ld/v1/csourceSubscriptions/{RS}").as_str()))
        .expect(1)
        .mount(&server)
        .await;
    let mut retrieved = watch_json();
    retrieved["id"] = json!(RS);
    Mock::given(method("GET"))
        .and(path(format!("/ngsi-ld/v1/csourceSubscriptions/{RS}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(retrieved))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let watch: Subscription<Input, RegistrationSubscription> = Subscription::parse(&watch_json().to_string()).unwrap();
    let created = client.registration_subscriptions().create(&watch).send().await.unwrap();
    let id = created.id().unwrap();
    assert_eq!(id, RS.parse::<SubscriptionId>().unwrap());
    let read = client.registration_subscriptions().retrieve(&id).send().await.unwrap();
    assert_eq!(read.id(), &id);
    assert_eq!(read.csf(), watch.csf());
    assert_eq!(read.csf().map(ToString::to_string).as_deref(), Some(r#"mode=="inclusive""#));
}

#[tokio::test]
async fn an_unbounded_registration_query_is_refused_without_a_request() {
    let server = MockServer::start().await;
    let error = client(&server.uri()).registrations().query().send().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::UnboundedRegistrationQuery))),
        "{error:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}
