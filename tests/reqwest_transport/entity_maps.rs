//! The entity map and source identity operations over the client (clauses 6.32 to 6.34).

use crate::{fixtures::client, links::context_link};
use headers::Header;
use http::header;
use ngsi_ld::{
    client::ClientError,
    http::{
        RequestError,
        headers::{NgsiLdEntityMap, NgsiLdTenant},
        operation::{ArgumentError, QueryMethod},
    },
    model::{EntityId, OneOrMany, Tenant},
    query::{EntitySelector, temporal_query},
    registration::EntityMapId,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// `M`.
const M: &str = "urn:ngsi-ld:EntityMap:1";

/// `T`.
const T: &str = "2030-01-01T00:00:00Z";

/// `MAP`.
fn map() -> serde_json::Value {
    json!({
        "id": M,
        "type": "EntityMap",
        "expiresAt": T,
        "entityMap": {
            "urn:ngsi-ld:Vehicle:A4567": ["@none", "urn:ngsi-ld:ContextSourceRegistration:r1"],
            "urn:ngsi-ld:Vehicle:B9211": ["urn:ngsi-ld:ContextSourceRegistration:r1"]
        },
        "linkedMaps": {"urn:ngsi-ld:ContextSourceRegistration:r1": "urn:ngsi-ld:EntityMap:remote7"}
    })
}

#[tokio::test]
async fn a_map_is_created_retrieved_updated_and_deleted() {
    let server = MockServer::start().await;
    let location = format!("/ngsi-ld/v1/entityMaps/{M}");
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entityMaps"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("entityMapLifetime", "PT1H"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header(NgsiLdEntityMap::name(), location.as_str())
                .set_body_json(map()),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(location.as_str()))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(map()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(location.as_str()))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(body_json(json!({"id": M, "type": "EntityMap", "expiresAt": T})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(location.as_str()))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let created = client
        .entity_maps()
        .create()
        .entity_type("Vehicle")
        .lifetime("PT1H".parse().unwrap())
        .send()
        .await
        .unwrap();
    let id: EntityMapId = created.entity_map().valid().and_then(NgsiLdEntityMap::id).unwrap();
    assert_eq!(id, created.body().id);
    let map = client.entity_maps().retrieve(&id).send().await.unwrap();
    assert_eq!(map.entity_map.len(), 2);
    client.entity_maps().update(&id, &T.parse().unwrap()).send().await.unwrap();
    client.entity_maps().delete(&id).send().await.unwrap();
}

#[tokio::test]
async fn a_post_creation_sends_every_entity_selector_in_its_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entityMaps"))
        .and(body_json(json!({
            "type": "Query",
            "entities": [
                {"type": "Vehicle", "id": ["urn:ngsi-ld:Vehicle:A4567"]},
                {"type": "Car"}
            ]
        })))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header(NgsiLdEntityMap::name(), format!("/ngsi-ld/v1/entityMaps/{M}").as_str())
                .set_body_json(map()),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let ids = OneOrMany::try_from(vec!["urn:ngsi-ld:Vehicle:A4567".parse::<EntityId>().unwrap()]).unwrap();
    let created = client
        .entity_maps()
        .create()
        .selectors([
            EntitySelector::new("Vehicle".parse().unwrap()).with_ids(ids),
            EntitySelector::new("Car".parse().unwrap()),
        ])
        .method(QueryMethod::Post)
        .send()
        .await
        .unwrap();
    assert_eq!(created.body().id.to_string(), M);
}

#[tokio::test]
async fn a_post_temporal_creation_sends_every_entity_selector_in_its_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/temporal/entityMaps"))
        .and(body_json(json!({
            "type": "Query",
            "entities": [{"type": "Car"}, {"type": "Bus"}],
            "temporalQ": {"timerel": "after", "timeAt": "2024-05-01T10:00:00Z"}
        })))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header(NgsiLdEntityMap::name(), format!("/ngsi-ld/v1/entityMaps/{M}").as_str())
                .set_body_json(map()),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let created = client
        .entity_maps()
        .create_temporal(temporal_query::after("2024-05-01T10:00:00Z".parse().unwrap()))
        .selectors(["Car", "Bus"])
        .method(QueryMethod::Post)
        .send()
        .await
        .unwrap();
    assert_eq!(created.body().id.to_string(), M);
}

#[tokio::test]
async fn a_get_creation_refuses_several_selectors_and_an_invalid_one_before_sending() {
    let server = MockServer::start().await;
    let client = client(&server.uri());
    let error = client.entity_maps().create().selectors(["Car", "Bus"]).send().await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Request(RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString))
    ));
    let error = client
        .entity_maps()
        .create_temporal(temporal_query::after("2024-05-01T10:00:00Z".parse().unwrap()))
        .selectors(["Car;Bus"])
        .method(QueryMethod::Post)
        .send()
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Request(RequestError::Argument(ArgumentError::InvalidArgument { .. }))
    ));
}

#[tokio::test]
async fn the_source_identity_of_a_tenant_is_read() {
    let server = MockServer::start().await;
    let identity = json!({
        "id": "urn:ngsi-ld:ContextSource:cs1",
        "type": "ContextSourceIdentity",
        "contextSourceAlias": "broker-1",
        "contextSourceUptime": "P1DT2H",
        "contextSourceTimeAt": "2024-05-01T10:00:00Z",
        "contextSourceExtras": {"version": "1.2", "tenants": ["t1"]}
    });
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/info/sourceIdentity"))
        .and(header(NgsiLdTenant::name(), "t1"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(identity))
        .expect(1)
        .mount(&server)
        .await;
    let tenant: Tenant = "t1".parse().unwrap();
    let identity = client(&server.uri()).source_identity().tenant(&tenant).send().await.unwrap();
    assert_eq!(identity.alias.as_ref(), "broker-1");
    assert_eq!(identity.uptime.to_string(), "P1DT2H");
}
