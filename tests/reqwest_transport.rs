//! The client over `ReqwestTransport` against a mock server: the exact request that reaches the wire, and each
//! response shape the specification allows coming back typed.
#![cfg(feature = "reqwest")]

use crate::{
    fixtures::{client, vehicle},
    links::{context_link, link_field, link_value},
};
use headers::Header;
use http::header;
use iri_rs::{IriRefBuf, iri};
use ngsi_ld::{
    client::ClientError,
    http::{
        BatchOutcome,
        CreateOutcome,
        RequestError,
        ResponseError,
        headers::{LinkValue, NgsiLdEntityMap, NgsiLdResultsCount, NgsiLdTenant, OptionalHeader, RelationType},
        operation::{ArgumentError, EntityIdMember, EntityMapRequest, InlineJoin, ResultsCount},
    },
    model::{APPLICATION_JSON, Entity, EntityFragment, EntityId, ErrorType, Tenant},
    query::{Join, Quality, language_filter, ordering, q},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use std::{net::TcpListener, num::NonZeroU32};
use url::Url;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// `Link` fields built from typed link-values.
#[cfg(test)]
#[path = "support/links.rs"]
mod links;

/// Clients and entities.
#[cfg(test)]
mod fixtures {
    use crate::links::CONTEXT;
    use iri_rs::{IriRefBuf, iri};
    use ngsi_ld::{
        client::{Client, ReqwestTransport},
        model::{Entity, Input},
    };
    use url::Url;

    /// A client of `base`, with the test context.
    pub(crate) fn client(base: &str) -> Client<ReqwestTransport> {
        Client::builder()
            .base_url(Url::parse(&format!("{base}/ngsi-ld/v1/")).unwrap())
            .context(IriRefBuf::new(CONTEXT).unwrap())
            .transport(ReqwestTransport::try_default().unwrap())
            .build()
            .unwrap()
    }

    /// A small vehicle.
    pub(crate) fn vehicle() -> Entity<Input> {
        Entity::builder()
            .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
            .entity_type("Vehicle")
            .property("brandName", "Mercedes")
            .build()
            .unwrap()
    }
}

/// The `GeoJSON` representation, projected entities and the entity map of the following pages, over the client.
#[cfg(test)]
#[path = "reqwest_transport/representations.rs"]
mod representations;

/// Partial Attribute Update with each body it takes, over the client.
#[cfg(test)]
#[path = "reqwest_transport/attributes.rs"]
mod attributes;

/// Merge Entity with a simplified fragment, over the client.
#[cfg(test)]
#[path = "reqwest_transport/merge_simplified.rs"]
mod merge_simplified;

/// The snapshot operations and a client's snapshot view, over the client.
#[cfg(test)]
#[path = "reqwest_transport/snapshots.rs"]
mod snapshots;

/// The JSON-LD context loader over the client.
#[cfg(test)]
#[cfg(feature = "linked-data")]
#[path = "reqwest_transport/context_loader.rs"]
mod context_loader;

/// Clients derived from another, and the head of an error read from the wire.
#[cfg(test)]
#[path = "reqwest_transport/overrides.rs"]
mod overrides;

/// The batch operations, over the client.
#[cfg(test)]
#[path = "reqwest_transport/batch.rs"]
mod batch;

/// Query Entities with a geoquery and a scope query, over the client.
#[cfg(test)]
#[path = "reqwest_transport/geo_and_scope_queries.rs"]
mod geo_and_scope_queries;

/// The subscription operations, over the client.
#[cfg(test)]
#[path = "reqwest_transport/subscriptions.rs"]
mod subscriptions;

/// The context source registration and registration subscription operations, over the client.
#[cfg(test)]
#[path = "reqwest_transport/registrations.rs"]
mod registrations;

/// The temporal operations, over the client.
#[cfg(test)]
#[path = "reqwest_transport/temporal.rs"]
mod temporal;

/// The discovery and stored `@context` operations, over the client.
#[cfg(test)]
#[path = "reqwest_transport/discovery.rs"]
mod discovery;

/// The entity map and source identity operations, over the client.
#[cfg(test)]
#[path = "reqwest_transport/entity_maps.rs"]
mod entity_maps;

/// Preferences, locality and the implicit locality on a snapshot, over the client.
#[cfg(test)]
#[path = "reqwest_transport/request_options.rs"]
mod request_options;

/// The `NGSILD-Warning` header, over the client.
#[cfg(test)]
#[path = "reqwest_transport/warnings.rs"]
mod warnings;

/// The flags and inputs a client may state explicitly, over the client.
#[cfg(test)]
#[path = "reqwest_transport/request_flags.rs"]
mod request_flags;

#[tokio::test]
async fn create_sends_the_entity_and_reads_the_location() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(
            json!({"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "brandName": {"type": "Property", "value": "Mercedes"}}),
        ))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .expect(1)
        .mount(&server)
        .await;
    let vehicle = vehicle();
    let outcome = client(&server.uri()).entities().create(&vehicle).send().await.unwrap();
    let CreateOutcome::Created {
        location: OptionalHeader::Valid(location),
    } = outcome
    else {
        panic!("expected Created, got {outcome:?}")
    };
    assert_eq!(location.as_str(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
}

#[tokio::test]
async fn a_duplicate_create_comes_back_as_already_exists() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/AlreadyExists", "title": "Already exists"})))
        .mount(&server)
        .await;
    let vehicle = vehicle();
    let error = client(&server.uri()).entities().create(&vehicle).send().await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::AlreadyExists));
}

#[tokio::test]
async fn a_simplified_query_sends_its_parameters_and_reads_the_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("q", r#"brandName!="Mercedes""#))
        .and(query_param("format", "simplified"))
        .and(query_param("limit", "1"))
        .and(query_param("count", "true"))
        .and(header(&header::ACCEPT, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "brandName": "Volvo"}]))
                .insert_header(NgsiLdResultsCount::name(), "2")
                .insert_header(
                    &header::LINK,
                    link_field([
                        LinkValue::builder(IriRefBuf::new("/ngsi-ld/v1/entities?type=Vehicle&format=simplified&limit=1&offset=1").unwrap())
                            .rel(RelationType::Next)
                            .media_type(APPLICATION_JSON)
                            .build(),
                    ]),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let page = client
        .entities()
        .query_simplified()
        .entity_type("Vehicle")
        .q(q::attr("brandName").ne("Mercedes"))
        .limit(1)
        .count(ResultsCount::Include)
        .send()
        .await
        .unwrap();
    assert_eq!(page.items()[0].id.as_str(), "urn:ngsi-ld:Vehicle:B9211");
    assert_eq!(page.total(), Some(2));
    assert_eq!(
        page.next().map(Url::as_str),
        Some(format!("{}/ngsi-ld/v1/entities?type=Vehicle&format=simplified&limit=1&offset=1", server.uri()).as_str())
    );
}

#[tokio::test]
async fn the_next_page_is_requested_from_its_link() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:A456", "type": "Vehicle", "brandName": "Mercedes"}])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "brandName": "Volvo"}]))
                .insert_header(
                    &header::LINK,
                    link_field([link_value(
                        "/ngsi-ld/v1/entities?type=Vehicle&format=simplified&limit=1&offset=1",
                        RelationType::Next,
                    )]),
                ),
        )
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let first = client.entities().query_simplified().entity_type("Vehicle").limit(1).send().await.unwrap();
    let second = client.send(&first.next_page().unwrap()).await.unwrap();
    assert_eq!(second.items()[0].id.as_str(), "urn:ngsi-ld:Vehicle:A456");
    assert!(second.next_page().is_none());
}

#[tokio::test]
async fn every_page_is_walked_on_the_tenant_of_the_query_and_its_items_collected() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .and(header(NgsiLdTenant::name(), "t1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:A456", "type": "Vehicle", "brandName": "Mercedes"}])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(header(NgsiLdTenant::name(), "t1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "brandName": "Volvo"}]))
                .insert_header(
                    &header::LINK,
                    &link_field([link_value("?type=Vehicle&format=simplified&limit=1&offset=1", RelationType::Next)]),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let tenant: Tenant = "t1".parse().unwrap();
    let first = client
        .entities()
        .query_simplified()
        .entity_type("Vehicle")
        .limit(1)
        .tenant(&tenant)
        .send()
        .await
        .unwrap();
    let items = client.pages(first).try_collect().await.unwrap();
    assert_eq!(
        items.iter().map(|vehicle| vehicle.id.as_str()).collect::<Vec<_>>(),
        ["urn:ngsi-ld:Vehicle:B9211", "urn:ngsi-ld:Vehicle:A456"]
    );
}

#[tokio::test]
async fn a_failed_page_ends_the_walk_after_reporting_its_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/InternalError"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "brandName": "Volvo"}]))
                .insert_header(
                    &header::LINK,
                    &link_field([link_value("/ngsi-ld/v1/entities?type=Vehicle&limit=1&offset=1", RelationType::Next)]),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let first = client.entities().query_simplified().entity_type("Vehicle").limit(1).send().await.unwrap();
    let mut pages = client.pages(first);
    assert_eq!(pages.next().await.unwrap().unwrap().items().len(), 1);
    let error = pages.next().await.unwrap().unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::InternalError));
    assert!(pages.next().await.is_none());
}

#[tokio::test]
async fn a_missing_entity_comes_back_as_resource_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Not found"})))
        .mount(&server)
        .await;
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let error = client(&server.uri()).entities().retrieve(&id).send().await.unwrap_err();
    assert!(matches!(&error, ClientError::Response(ResponseError::Problem { status, .. }) if status.as_u16() == 404));
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound));
}

#[tokio::test]
async fn merge_with_a_tenant_sends_the_fragment_and_the_tenant_header() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(header(NgsiLdTenant::name(), "t1"))
        .and(body_json(json!({"speed": {"type": "Property", "value": 0}, "isParked": "urn:ngsi-ld:null"})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let fragment = EntityFragment::builder().property("speed", 0).delete("isParked").build().unwrap();
    let tenant: Tenant = "t1".parse().unwrap();
    let outcome = client(&server.uri()).entities().merge(&id, &fragment).tenant(&tenant).send().await.unwrap();
    assert_eq!(outcome, BatchOutcome::Done);
}

#[tokio::test]
async fn an_id_written_verbatim_keeps_its_upper_case_scheme_in_the_path_and_every_body() {
    const ID: &str = "URN:NGSI-LD:Vehicle:B9211";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(body_json(json!({"id": ID, "type": "Vehicle", "speed": {"type": "Property", "value": 0}})))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, format!("/ngsi-ld/v1/entities/{ID}").as_str()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(format!("/ngsi-ld/v1/entities/{ID}").as_str()))
        .and(body_json(json!({"id": ID, "speed": {"type": "Property", "value": 50}})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entityOperations/update"))
        .and(body_json(json!([{"id": ID, "type": "Vehicle", "speed": {"type": "Property", "value": 90}}])))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id = || EntityId::verbatim(ID).unwrap();
    let vehicle = Entity::builder().id(id()).entity_type("Vehicle").property("speed", 0).build().unwrap();
    client.entities().create(&vehicle).send().await.unwrap();
    let fragment = EntityFragment::builder().id(id()).property("speed", 50).build().unwrap();
    assert_eq!(client.entities().merge(&id(), &fragment).send().await.unwrap(), BatchOutcome::Done);
    let batch = [EntityFragment::for_entity(id(), "Vehicle").property("speed", 90).build().unwrap()];
    assert_eq!(client.entities().batch_update(&batch).send().await.unwrap(), BatchOutcome::Done);
}

#[tokio::test]
async fn replace_sends_the_entity_with_its_id_and_without_it_when_omitted() {
    let server = MockServer::start().await;
    let target = "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567";
    let brand = json!({"type": "Property", "value": "Mercedes"});
    Mock::given(method("PUT"))
        .and(path(target))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(body_json(json!({"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "brandName": brand})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(target))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(body_json(json!({"type": "Vehicle", "brandName": brand})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let vehicle = vehicle();
    let written = client.entities().replace(&vehicle).send().await.unwrap();
    assert_eq!(written, BatchOutcome::Done);
    let omitted = client.entities().replace(&vehicle).id_member(EntityIdMember::Omitted).send().await.unwrap();
    assert_eq!(omitted, BatchOutcome::Done);
}

#[tokio::test]
async fn an_invalid_query_argument_is_refused_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let error = client(&server.uri()).entities().query().entity_type("1 not a type").send().await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Request(RequestError::Argument(ArgumentError::InvalidArgument { .. }))
    ));
}

#[tokio::test]
async fn a_query_sends_its_language_filter_ordering_and_join() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("lang", "fr,en;q=0.5"))
        .and(query_param("orderBy", "speed;desc"))
        .and(query_param("join", "inline"))
        .and(query_param("joinLevel", "2"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": {"type": "Property", "value": 80}}])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server.uri())
        .entities()
        .query()
        .entity_type("Vehicle")
        .lang(language_filter::language("fr").weighted_language("en", Quality::try_from_thousandths(500).unwrap()))
        .ordering(ordering::descending("speed"))
        .join(Join::Inline)
        .join_level(NonZeroU32::new(2).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(page.items()[0].id.as_str(), "urn:ngsi-ld:Vehicle:B9211");
}

#[tokio::test]
async fn a_flattened_retrieve_reads_the_entity_graph_and_the_entity_map() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(query_param("join", "flat"))
        .and(query_param("entityMap", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([
                    {"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "isParked": {"type": "Relationship", "object": "urn:ngsi-ld:OffStreetParking:Downtown1"}},
                    {"id": "urn:ngsi-ld:OffStreetParking:Downtown1", "type": "OffStreetParking"}
                ]))
                .insert_header(NgsiLdEntityMap::name(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let request = EntityMapRequest::returned();
    let retrieved = client(&server.uri())
        .entities()
        .retrieve_flattened(&id)
        .entity_map(&request)
        .send()
        .await
        .unwrap();
    let ids: Vec<&str> = retrieved.body().iter().map(|entity| entity.id.as_str()).collect();
    assert_eq!(ids, ["urn:ngsi-ld:Vehicle:A4567", "urn:ngsi-ld:OffStreetParking:Downtown1"]);
    assert_eq!(
        retrieved.entity_map().valid().map(|map| map.as_iri_ref().as_str()),
        Some("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
    );
}

#[tokio::test]
async fn jl_01_the_least_join_level_is_sent_as_one() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(query_param("join", "inline"))
        .and(query_param("joinLevel", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle"})))
        .expect(1)
        .mount(&server)
        .await;
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let retrieved = client(&server.uri())
        .entities()
        .retrieve(&id)
        .join(InlineJoin::Inline)
        .join_level(NonZeroU32::MIN)
        .send()
        .await
        .unwrap();
    assert_eq!(retrieved.body().id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
}

#[tokio::test]
async fn an_unreachable_broker_is_a_transport_error() {
    // A port that was just free is closed: nothing answers there.
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let base = format!("http://127.0.0.1:{port}");
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let error = client(&base).entities().delete(&id).send().await.unwrap_err();
    assert!(matches!(error, ClientError::Transport(_)));
}

/// Several entity selectors, every type, whole id patterns, representation spellings and projected inline joins, over
/// the client.
#[cfg(test)]
#[path = "reqwest_transport/query_selection.rs"]
mod query_selection;
