//! The discovery and stored `@context` operations over the client (clauses 6.25 to 6.30).

use crate::{fixtures::client, links::context_link};
use http::header;
use ngsi_ld::{
    client::ClientError,
    http::{ResponseError, operation::ContextDeletion},
    model::{AttributeName, Context, ContextEntry, ContextId, ContextMetadata, InlineContext, StoredContextKind},
    query::Locality,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use url::Url;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// `C`.
const C: &str = "urn:ngsi-ld:context:c1";

/// `inline`.
fn inline() -> Context {
    Context::Single(ContextEntry::Inline(
        r#"{"speed":"https://example.org/speed"}"#.parse::<InlineContext>().unwrap(),
    ))
}

#[tokio::test]
async fn the_detailed_entity_type_list_is_requested_locally_and_read() {
    let server = MockServer::start().await;
    let vehicle = json!({"id": "https://example.org/Vehicle", "type": "EntityType", "attributeNames": ["speed"], "typeName": "Vehicle"});
    let bus = json!({"id": "https://example.org/Bus", "type": "EntityType", "attributeNames": ["speed"], "typeName": "Bus"});
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/types"))
        .and(query_param("details", "true"))
        .and(query_param("local", "true"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([vehicle, bus])))
        .expect(1)
        .mount(&server)
        .await;
    let details = client(&server.uri())
        .entity_types()
        .list_details()
        .locality(Locality::Local)
        .send()
        .await
        .unwrap();
    let names: Vec<&str> = details.iter().map(|details| details.type_name.as_ref()).collect();
    assert_eq!(names, ["Vehicle", "Bus"]);
}

#[tokio::test]
async fn attribute_information_is_requested_by_name_and_read() {
    let server = MockServer::start().await;
    let body = json!({
        "id": "https://example.org/speed",
        "type": "Attribute",
        "attributeName": "speed",
        "attributeCount": 3,
        "attributeTypes": ["Property", "GeoProperty"],
        "typeNames": ["Vehicle", "Bus"]
    });
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/attributes/speed"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;
    let speed: AttributeName = "speed".parse().unwrap();
    let info = client(&server.uri()).attributes().retrieve(&speed).send().await.unwrap();
    assert_eq!(info.attribute_count, Some(3));
    assert_eq!(info.type_names.map(|names| names.len()), Some(2));
}

#[tokio::test]
async fn a_context_is_added_served_and_reloaded() {
    let server = MockServer::start().await;
    let document = json!({"@context": {"speed": "https://example.org/speed"}});
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/jsonldContexts"))
        .and(header(&header::CONTENT_TYPE, "application/ld+json"))
        .and(body_json(&document))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, format!("/ngsi-ld/v1/jsonldContexts/{C}").as_str()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/ngsi-ld/v1/jsonldContexts/{C}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&document))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("/ngsi-ld/v1/jsonldContexts/{C}")))
        .and(query_param("reload", "true"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let context = inline();
    let created = client.contexts().add(&context).send().await.unwrap();
    let id = created.id().unwrap();
    assert_eq!(id, C.parse::<ContextId>().unwrap());
    assert_eq!(client.contexts().serve(&id).send().await.unwrap(), context);
    client.contexts().delete(&id).deletion(ContextDeletion::Reload).send().await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let add = requests.iter().find(|request| request.method.as_str() == "POST").unwrap();
    assert!(add.headers.get(&header::LINK).is_none(), "{:?}", add.headers);
}

#[tokio::test]
async fn the_details_of_stored_contexts_are_read_and_a_member_outside_their_table_is_refused() {
    let server = MockServer::start().await;
    let url = format!("{}/ngsi-ld/v1/jsonldContexts/{C}", server.uri());
    let hosted = json!({"URL": url, "localId": C, "kind": "Hosted", "createdAt": "2024-05-01T10:00:00Z"});
    let cached = json!({
        "URL": "https://example.org/context.jsonld",
        "localId": C,
        "kind": "Cached",
        "createdAt": "2024-05-01T10:00:00Z",
        "numberOfHits": 3,
        "extraInfo": {"etag": "x1"}
    });
    let unknown = json!({"URL": url, "localId": C, "kind": "Hosted", "createdAt": "2024-05-01T10:00:00Z", "foo": 1});
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts"))
        .and(query_param("details", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([hosted])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/ngsi-ld/v1/jsonldContexts/{C}")))
        .and(query_param("details", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(cached))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/ngsi-ld/v1/jsonldContexts/{C}")))
        .and(query_param("details", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(unknown))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id: ContextId = C.parse().unwrap();
    let listed = client.contexts().list_details().send().await.unwrap();
    let created_at = "2024-05-01T10:00:00Z".parse().unwrap();
    assert_eq!(
        listed,
        [ContextMetadata::new(
            Url::parse(&url).unwrap(),
            C.parse().unwrap(),
            StoredContextKind::Hosted,
            created_at
        )]
    );
    let cached_details = client.contexts().serve_details(&id).send().await.unwrap();
    assert_eq!((cached_details.kind, cached_details.number_of_hits), (StoredContextKind::Cached, Some(3)));
    assert_eq!(cached_details.extra_info.map(|info| info.to_string()).as_deref(), Some(r#"{"etag":"x1"}"#));
    let refused = client.contexts().serve_details(&id).send().await.unwrap_err();
    assert!(matches!(refused, ClientError::Response(ResponseError::Body { .. })), "{refused:?}");
}
