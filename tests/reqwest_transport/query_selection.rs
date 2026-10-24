//! The client methods of Query Entities and Retrieve Entity with several entity selectors, every type (`*`), a
//! whole id pattern, the spellings of the representation and a projected inline join (clauses 4.21, 4.5.23.2,
//! 5.2.33, 6.3.7, 6.4.3.2 and 6.23.3.1).

use crate::fixtures::client;
use headers::Header;
use iri_rs::iri;
use ngsi_ld::{
    client::ClientError,
    http::{
        RequestError,
        headers::NgsiLdEntityMap,
        operation::{ArgumentError, InlineJoin, QueryMethod, RepresentationSpelling},
    },
    model::{Attribute, AttributeName, EntityId, Instances, OneOrMany},
    query::{EntitySelector, IdPattern, Locality, SelectorType, projection, temporal_query},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Match,
    Mock,
    MockServer,
    Request,
    ResponseTemplate,
    matchers::{body_json, method, path},
};

/// Matches a request whose query string is exactly the one given, `None` for no query string.
struct QueryIs(Option<&'static str>);

// A matcher of the whole query string, which the matchers of `wiremock` do not offer.
impl Match for QueryIs {
    fn matches(&self, request: &Request) -> bool {
        request.url.query() == self.0
    }
}

#[tokio::test]
async fn a_post_query_sends_every_entity_selector_in_its_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entityOperations/query"))
        .and(QueryIs(Some("format=simplified")))
        .and(body_json(json!({
            "type": "Query",
            "entities": [
                {"type": "Vehicle", "idPattern": "^urn:ngsi-ld:Vehicle:A.*$|^urn:ngsi-ld:Vehicle:B.*$"},
                {"type": "Car", "id": "urn:ngsi-ld:Car:1"},
                {"type": "*"}
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": "urn:ngsi-ld:Car:1", "type": "Car"}])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let pattern: IdPattern = "^urn:ngsi-ld:Vehicle:A.*$|^urn:ngsi-ld:Vehicle:B.*$".parse().unwrap();
    let page = client
        .entities()
        .query_simplified()
        .entity_type("Vehicle")
        .id_pattern(pattern)
        .selectors([
            EntitySelector::new("Car".parse().unwrap()).with_ids(EntityId::from(iri!("urn:ngsi-ld:Car:1"))),
            EntitySelector::all(),
        ])
        .method(QueryMethod::Post)
        .send()
        .await
        .unwrap();
    assert_eq!(page.items().len(), 1);
}

#[tokio::test]
async fn a_get_query_of_every_type_sends_a_star_and_refuses_to_be_distributed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(QueryIs(Some("type=*&options=keyValues")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let page = client
        .entities()
        .query_simplified()
        .entity_type(SelectorType::All)
        .spelling(RepresentationSpelling::default().in_options().key_values())
        .send()
        .await
        .unwrap();
    assert_eq!(page.items().first(), None);
    let error = client
        .entities()
        .query()
        .entity_type(SelectorType::All)
        .locality(Locality::Distributed)
        .send()
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::AllTypesNotLocal))));
    let error = client.entities().query().entity_type("Vehicle").selectors(["Car"]).send().await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Request(RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString))
    ));
    let error = client.entities().query().selectors(["Car;Bus"]).send().await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Request(RequestError::Argument(ArgumentError::InvalidArgument { .. }))
    ));
}

#[tokio::test]
async fn a_retrieve_writes_normalized_explicitly_and_reads_a_narrowed_linked_entity() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(QueryIs(Some("pick=isParked%7Bname%7D&join=inline&format=normalized")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "isParked": {
                "type": "Relationship",
                "object": "urn:ngsi-ld:OffStreetParking:Downtown1",
                "entity": {"name": {"type": "Property", "value": "Top Parking"}}
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let entity = client
        .entities()
        .retrieve_projected(&id)
        .pick(projection::linked("isParked", projection::attributes(["name"])))
        .join(InlineJoin::Inline)
        .spelling(RepresentationSpelling::default().explicit_normalized())
        .send()
        .await
        .unwrap()
        .into_body();
    let is_parked: AttributeName = "isParked".parse().unwrap();
    let Some(Attribute::Relationship(relationship)) = entity.attribute(&is_parked).map(Instances::first) else {
        panic!("not a Relationship");
    };
    let parking = relationship.output().entity.as_ref().and_then(OneOrMany::as_one).unwrap();
    assert_eq!((parking.id.as_ref(), parking.entity_type.as_ref()), (None, None));
    assert_eq!(parking.attributes.len(), 1);
}

#[tokio::test]
async fn the_temporal_query_purge_entity_map_and_registration_builders_take_every_type_and_whole_id_patterns() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(QueryIs(Some(
            "type=*&idPattern=%5Ea%24%7C%5Eb%24&timerel=after&timeAt=2018-08-01T12%3A00%3A00Z&options=temporalValues",
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(QueryIs(Some("type=*&idPattern=%5Ea%24%7C%5Eb%24")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entityMaps"))
        .and(QueryIs(Some("type=*&idPattern=%5Ea%24%7C%5Eb%24")))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header(NgsiLdEntityMap::name(), "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")
                .set_body_json(
                    json!({"id": "urn:ngsi-ld:EntityMap:1", "type": "EntityMap", "expiresAt": "2030-01-01T00:00:00Z", "entityMap": {}, "linkedMaps": {}}),
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/csourceRegistrations"))
        .and(QueryIs(Some("type=Vehicle&idPattern=%5Ea%24%7C%5Eb%24")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let pattern: IdPattern = "^a$|^b$".parse().unwrap();
    let page = client
        .temporal()
        .query_values(temporal_query::after("2018-08-01T12:00:00Z".parse().unwrap()))
        .entity_type(SelectorType::All)
        .id_pattern(pattern.clone())
        .spelling(RepresentationSpelling::default().in_options())
        .send()
        .await
        .unwrap();
    assert_eq!(page.items().first(), None);
    client
        .entities()
        .purge()
        .entity_type(SelectorType::All)
        .id_pattern(pattern.clone())
        .send()
        .await
        .unwrap();
    let created = client
        .entity_maps()
        .create()
        .entity_type(SelectorType::All)
        .id_pattern(pattern.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(created.body().id.to_string(), "urn:ngsi-ld:EntityMap:1");
    let registrations = client.registrations().query().entity_type("Vehicle").id_pattern(pattern).send().await.unwrap();
    assert_eq!(registrations.items().len(), 0);
}
