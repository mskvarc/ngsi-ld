//! The client methods for the `GeoJSON` representation and for projected entities, and the entity map the client
//! names on the following pages of a query.

use crate::{
    fixtures::client,
    links::{link_field, link_value},
};
use headers::Header;
use http::header;
use iri_rs::{IriRefBuf, iri};
use ngsi_ld::{
    client::ClientError,
    http::{
        RequestError,
        headers::{NgsiLdEntityMap, RelationType},
        operation::{ArgumentError, EntityMapRequest},
    },
    model::{AttributeName, EntityId},
    query::projection::{self, EntityMember},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{header, method, path, query_param, query_param_is_missing},
};

#[tokio::test]
async fn a_geo_json_query_walks_its_feature_collections() {
    let server = MockServer::start().await;
    let feature = |id: &str| json!({"id": id, "type": "Feature", "geometry": null, "properties": {"type": "Vehicle", "speed": 80}});
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .and(header(&header::ACCEPT, "application/geo+json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"type": "FeatureCollection", "features": [feature("urn:ngsi-ld:Vehicle:B9211")]})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("geometryProperty", "site"))
        .and(header(&header::ACCEPT, "application/geo+json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"type": "FeatureCollection", "features": [feature("urn:ngsi-ld:Vehicle:A4567")]}))
                .insert_header(
                    &header::LINK,
                    link_field([link_value(
                        "?type=Vehicle&format=simplified&geometryProperty=site&limit=1&offset=1",
                        RelationType::Next,
                    )]),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let site: AttributeName = "site".parse().unwrap();
    let first = client
        .entities()
        .query_geo_json_simplified()
        .entity_type("Vehicle")
        .geometry_property(&site)
        .limit(1)
        .send()
        .await
        .unwrap();
    let features = client.pages(first).try_collect().await.unwrap();
    assert_eq!(
        features.iter().map(|feature| feature.entity.id.as_str()).collect::<Vec<_>>(),
        ["urn:ngsi-ld:Vehicle:A4567", "urn:ngsi-ld:Vehicle:B9211"]
    );
}

#[tokio::test]
async fn a_projection_removing_id_is_refused_for_complete_entities_and_read_for_projected_ones() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(query_param("pick", "speed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"speed": {"type": "Property", "value": 80}})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let error = client
        .entities()
        .retrieve(&id)
        .pick(projection::attributes(["speed"]))
        .send()
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Request(RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Id }))
    ));
    let entity = client
        .entities()
        .retrieve_projected(&id)
        .pick(projection::attributes(["speed"]))
        .send()
        .await
        .unwrap()
        .into_body();
    assert_eq!(entity.id, None);
    assert_eq!(entity.attributes.len(), 1);
}

#[tokio::test]
async fn a_projected_query_walks_every_page_of_entities_without_id_in_both_representations() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"speed": 40}])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("limit", "1"))
        .and(query_param("pick", "speed"))
        .and(query_param("format", "simplified"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    &header::LINK,
                    link_field([link_value("?type=Vehicle&format=simplified&pick=speed&limit=1&offset=1", RelationType::Next)]),
                )
                .set_body_json(json!([{"speed": 80}])),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("pick", "speed"))
        .and(query_param_is_missing("format"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"speed": {"type": "Property", "value": 80}}])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let first = client
        .entities()
        .query_projected_simplified()
        .entity_type("Vehicle")
        .pick(projection::attributes(["speed"]))
        .limit(1)
        .send()
        .await
        .unwrap();
    let every = client.pages(first).try_collect().await.unwrap();
    assert_eq!(every.len(), 2);
    assert!(every.iter().all(|entity| entity.id.is_none() && entity.entity_type.is_none()));
    let normalized = client
        .entities()
        .query_projected()
        .entity_type("Vehicle")
        .pick(projection::attributes(["speed"]))
        .send()
        .await
        .unwrap();
    assert_eq!(normalized.items().len(), 1);
    assert_eq!(normalized.items()[0].id, None);
}

#[tokio::test]
async fn the_client_names_the_entity_map_of_the_query_on_every_following_page() {
    let server = MockServer::start().await;
    let broker_map = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:2";
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .and(header(NgsiLdEntityMap::name(), broker_map))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle"}])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Vehicle"))
        .and(header(NgsiLdEntityMap::name(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle"}]))
                .insert_header(NgsiLdEntityMap::name(), broker_map)
                .insert_header(
                    &header::LINK,
                    &link_field([link_value("?type=Vehicle&format=simplified&limit=1&offset=1", RelationType::Next)]),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let map = NgsiLdEntityMap::try_new(IriRefBuf::new("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1".to_owned()).unwrap()).unwrap();
    let reuse = EntityMapRequest::reuse(map);
    let first = client
        .entities()
        .query_simplified()
        .entity_type("Vehicle")
        .entity_map(&reuse)
        .limit(1)
        .send()
        .await
        .unwrap();
    let items = client.pages(first).try_collect().await.unwrap();
    assert_eq!(items.len(), 2);
}
