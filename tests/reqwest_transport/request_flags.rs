//! The flags and inputs a client may state explicitly, over the client: `timeproperty` without a time window, the
//! entity map of Retrieve Temporal Evolution and Purge Entities, the temporal representation parameter,
//! `details=false`, `reload=false`, the members beside an added context, and Context Source Properties (clauses
//! 4.9, 5.7.3.3, 5.13.2.3, 6.3.12, Tables 6.4.3.3-2, 6.19.3.1-1, 6.25.3.1-1, 6.29.3.2-1 and 6.30.3.2-1).

use crate::{fixtures::client, links::context_link};
use headers::Header;
use http::header;
use indexmap::IndexMap;
use iri_rs::{IriRefBuf, iri};
use ngsi_ld::{
    client::ClientError,
    http::{
        RequestError,
        headers::NgsiLdEntityMap,
        operation::{ArgumentError, ContextDeletion, EntityMapRequest, RepresentationSpelling, RepresentationToken, SummaryDetails, SystemAttributes},
    },
    model::{Context, ContextId, EntityId, Value, evolution::AggregationMethod},
    query::temporal_query::{self, TemporalProperty},
    registration::{CSourceRegistration, RegistrationInfo},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use url::Url;
use wiremock::{
    Match,
    Mock,
    MockServer,
    Request,
    ResponseTemplate,
    matchers::{body_json, header, method, path},
};

/// `B`'s path below the API root.
const B_PATH: &str = "/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211";

/// The location of the entity map the requests name.
const MAP: &str = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1";

/// Matches a request whose query string is exactly the one given, `None` matching a request with none.
struct ExactQuery(Option<&'static str>);

// No derive: wiremock matchers are implemented by hand.
impl Match for ExactQuery {
    fn matches(&self, request: &Request) -> bool {
        request.url.query() == self.0
    }
}

/// `B`.
fn b() -> EntityId {
    iri!("urn:ngsi-ld:Vehicle:B9211").into()
}

/// The entity map at [`MAP`].
fn entity_map() -> NgsiLdEntityMap {
    NgsiLdEntityMap::new(IriRefBuf::new(MAP.to_owned()).unwrap()).unwrap()
}

#[tokio::test]
async fn retrieve_temporal_evolution_sends_the_time_property_the_entity_map_and_the_deprecated_keyword() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(ExactQuery(Some(
            "timeproperty=modifiedAt&lastN=2&entityMap=true&options=temporalValues,sysAttrs",
        )))
        .and(header(NgsiLdEntityMap::name(), MAP))
        .and(header(&header::LINK, context_link().to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "urn:ngsi-ld:Vehicle:B9211",
            "type": "Vehicle",
            "speed": {"type": "Property", "values": [[80, "2018-08-01T12:05:00Z"]]}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let request = EntityMapRequest::returned().reusing(entity_map());
    let retrieved = client(&server.uri())
        .temporal()
        .retrieve_values(&b())
        .time_property(TemporalProperty::ModifiedAt)
        .last_n(2)
        .entity_map(&request)
        .system_attributes(SystemAttributes::Include)
        .spelling(RepresentationSpelling::default().in_options())
        .send()
        .await
        .unwrap();
    assert!(retrieved.body().render().to_string().contains("urn:ngsi-ld:Vehicle:B9211"));
}

#[tokio::test]
async fn a_time_property_beside_a_temporal_query_is_refused_before_sending() {
    let server = MockServer::start().await;
    let error = client(&server.uri())
        .temporal()
        .retrieve(&b())
        .temporal_q(temporal_query::after("2018-08-01T12:00:00Z".parse().unwrap()))
        .time_property(TemporalProperty::CreatedAt)
        .send()
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            ClientError::Request(RequestError::Argument(ArgumentError::InstanceOptionsBesideTemporalQuery))
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn query_temporal_evolution_states_the_format_over_the_deprecated_keyword() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(ExactQuery(Some(
            "type=Vehicle&timerel=after&timeAt=2018-08-01T12%3A00%3A00Z&aggrMethods=min&format=temporalValues&options=aggregatedValues",
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let spelling = RepresentationSpelling::default().format_over_options(RepresentationToken::AggregatedValues);
    // `aggregatedValues` in `options` needs `aggrMethods` (Table 6.18.3.2-1), whatever `format` asks for.
    let error = client
        .temporal()
        .query_values(temporal_query::after("2018-08-01T12:00:00Z".parse().unwrap()))
        .entity_type("Vehicle")
        .spelling(spelling)
        .send()
        .await
        .unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::AggregationWithoutMethods))),
        "{error:?}"
    );
    let page = client
        .temporal()
        .query_values(temporal_query::after("2018-08-01T12:00:00Z".parse().unwrap()).aggregation_methods([AggregationMethod::Min]))
        .entity_type("Vehicle")
        .spelling(spelling)
        .send()
        .await
        .unwrap();
    assert_eq!(page.items().len(), 0);
}

#[tokio::test]
async fn purge_entities_names_its_entity_map_in_the_header() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(ExactQuery(Some("type=Vehicle")))
        .and(header(NgsiLdEntityMap::name(), MAP))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let map = entity_map();
    client(&server.uri())
        .entities()
        .purge()
        .entity_type("Vehicle")
        .entity_map(&map)
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn the_summary_requests_state_details_false_and_delete_states_reload_false() {
    let server = MockServer::start().await;
    let id: ContextId = "urn:ngsi-ld:context:c1".parse().unwrap();
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts"))
        .and(ExactQuery(Some("details=false")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["https://example.org/context.jsonld"])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1"))
        .and(ExactQuery(Some("details=false")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"@context": "https://example.org/context.jsonld"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1"))
        .and(ExactQuery(Some("reload=false")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/types"))
        .and(ExactQuery(Some("details=false")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "urn:ngsi-ld:EntityTypeList:1", "type": "EntityTypeList", "typeList": ["Vehicle"]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/attributes"))
        .and(ExactQuery(Some("details=false")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "urn:ngsi-ld:AttributeList:1", "type": "AttributeList", "attributeList": ["speed"]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let explicit = SummaryDetails::Explicit;
    let urls = client.contexts().list().details(explicit).send().await.unwrap();
    assert_eq!(urls, [Url::parse("https://example.org/context.jsonld").unwrap()]);
    let context = client.contexts().serve(&id).details(explicit).send().await.unwrap();
    assert_eq!(context, Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap()));
    client.contexts().delete(&id).deletion(ContextDeletion::RemoveExplicitly).send().await.unwrap();
    let types = client.entity_types().list().details(explicit).send().await.unwrap();
    assert_eq!(types.type_list.len(), 1);
    let attributes = client.attributes().list().details(explicit).send().await.unwrap();
    assert_eq!(attributes.attribute_list.len(), 1);
}

#[tokio::test]
async fn add_context_sends_the_members_beside_the_context() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/jsonldContexts"))
        .and(header(&header::CONTENT_TYPE, "application/ld+json"))
        .and(body_json(
            json!({"@context": "https://example.org/context.jsonld", "note": "discarded", "n": 1}),
        ))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, "/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1"))
        .expect(1)
        .mount(&server)
        .await;
    let context = Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap());
    let created = client(&server.uri())
        .contexts()
        .add(&context)
        .extra_members(&IndexMap::from([
            ("note".parse().unwrap(), Value::from("discarded")),
            ("n".parse().unwrap(), Value::from(1)),
        ]))
        .send()
        .await
        .unwrap();
    assert_eq!(created.id().map(|id| id.to_string()).as_deref(), Some("urn:ngsi-ld:context:c1"));
}

#[tokio::test]
async fn a_registration_sends_and_reads_its_context_source_properties() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/csourceRegistrations"))
        .and(body_json(json!({
            "type": "ContextSourceRegistration",
            "information": [{"entities": [{"type": "Vehicle"}]}],
            "endpoint": "http://my.csource.org:1026/",
            "csourceProperty1": "aValue",
            "jsonldContext": {"speed": "https://example.org/speed"}
        })))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, "/ngsi-ld/v1/csourceRegistrations/urn:ngsi-ld:ContextSourceRegistration:1"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/csourceRegistrations/urn:ngsi-ld:ContextSourceRegistration:1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "urn:ngsi-ld:ContextSourceRegistration:1",
            "type": "ContextSourceRegistration",
            "information": [{"entities": [{"type": "Vehicle"}]}],
            "endpoint": "http://my.csource.org:1026/",
            "csourceProperty1": "aValue",
            "status": "ok"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let registration = CSourceRegistration::builder("http://my.csource.org:1026")
        .information([RegistrationInfo::builder().entities(["Vehicle"]).build().unwrap()])
        .property("csourceProperty1", "aValue")
        .property(
            "jsonldContext",
            Value::Object([("speed".to_owned(), Value::from("https://example.org/speed"))].into_iter().collect()),
        )
        .build()
        .unwrap();
    let client = client(&server.uri());
    let created = client.registrations().register(&registration).send().await.unwrap();
    let id = created.id().unwrap();
    let retrieved = client.registrations().retrieve(&id).send().await.unwrap();
    let properties: Vec<(&str, String)> = retrieved.properties().iter().map(|(name, value)| (name.as_ref(), value.to_string())).collect();
    assert_eq!(properties, [("csourceProperty1", r#""aValue""#.to_owned())]);
}
