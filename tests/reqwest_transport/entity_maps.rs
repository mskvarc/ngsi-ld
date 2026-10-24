//! The entity map and source identity operations over the client (clauses 6.32 to 6.34).

use crate::{fixtures::client, links::context_link};
use headers::Header;
use http::header;
use ngsi_ld::{
    http::headers::{NgsiLdEntityMap, NgsiLdTenant},
    model::Tenant,
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
