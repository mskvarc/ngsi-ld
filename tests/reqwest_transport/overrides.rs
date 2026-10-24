//! Clients derived from another: the `@context`, `Accept`, `Prefer`, tenant and further headers they state on the
//! wire, sharing the original's transport; and the head of an error read from the wire.

use crate::{
    fixtures::{client, vehicle},
    links::{CONTEXT, link_field, link_value, profiled_context_value},
};
use headers::{Allow, CacheControl, ContentEncoding, ContentType, ETag, Header, IfNoneMatch, StrictTransportSecurity, Vary};
use http::{HeaderName, HeaderValue, Method, StatusCode, Version, header};
use iri_rs::{IriBuf, IriRefBuf, iri};
use ngsi_ld::{
    client::ClientError,
    http::{
        ContextDelivery,
        EndpointError,
        ResponseError,
        ResponseHead,
        headers::{
            Accept,
            AltSvc,
            Link,
            LinkValue,
            Location,
            NgsiLdResultsCount,
            NgsiLdTenant,
            Prefer,
            ReceivedBy,
            ReceivedProtocol,
            RelationType,
            SpecVersion,
            Via,
            ViaEntry,
        },
    },
    model::{Context, ContextEntry, EntityId, InlineContext, NgsiLdMediaType},
    query::{Quality, Weighted},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, header_exists, headers, method, path, query_param},
};

#[tokio::test]
async fn a_derived_client_sends_its_whole_context_in_an_ld_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(header(&header::CONTENT_TYPE, "application/ld+json"))
        .and(body_json(json!({
            "id": "urn:ngsi-ld:Vehicle:A4567",
            "type": "Vehicle",
            "brandName": {"type": "Property", "value": "Mercedes"},
            "@context": ["https://example.org/fleet.jsonld", {"@vocab": "https://example.org/terms/"}]
        })))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let context = Context::from(vec![
        ContextEntry::from(IriRefBuf::new("https://example.org/fleet.jsonld").unwrap()),
        ContextEntry::from(r#"{"@vocab":"https://example.org/terms/"}"#.parse::<InlineContext>().unwrap()),
    ]);
    let fleet = client.with_context(context).unwrap();
    let ld_json = fleet.with_context_delivery(ContextDelivery::LdJsonBody);
    let vehicle = vehicle();
    assert!(ld_json.entities().create(&vehicle).send().await.unwrap().is_created());
    assert!(client.endpoint().context().is_some_and(|context| context.link_target().is_some()));
}

#[tokio::test]
async fn a_derived_client_states_its_accept_prefer_tenant_and_further_header() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(header(&header::ACCEPT, "application/ld+json"))
        .and(header(Prefer::name(), "ngsi-ld=1.9"))
        .and(header(NgsiLdTenant::name(), "fleet"))
        .and(header(&header::IF_NONE_MATCH, "*"))
        .and(header_exists(&header::LINK))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let derived = client
        .with_accept(NgsiLdMediaType::JsonLd)
        .with_prefer(Prefer::builder().version(SpecVersion::new(1, 9)).build().unwrap())
        .with_tenant("fleet".parse().unwrap())
        .with_header(IfNoneMatch::any())
        .unwrap();
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    assert!(derived.entities().delete(&id).send().await.unwrap().is_done());
    assert_eq!(derived.endpoint().accept(), Some(&Accept::new(NgsiLdMediaType::JsonLd)));
    assert_eq!(client.endpoint().accept(), None);
}

#[tokio::test]
async fn an_error_from_the_wire_keeps_its_head() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(405).insert_header(&header::ALLOW, "GET"))
        .expect(1)
        .mount(&server)
        .await;
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let error = client(&server.uri()).entities().delete(&id).send().await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Response(ResponseError::Status {
            status: StatusCode::METHOD_NOT_ALLOWED,
            ..
        })
    ));
    let head = error.head().unwrap();
    assert_eq!(head.version(), Version::HTTP_11);
    assert_eq!(head.typed::<Allow>().into_valid().unwrap().iter().collect::<Vec<_>>(), [Method::GET]);
}

#[tokio::test]
async fn a_derived_client_sends_a_weighted_accept_and_a_via_hop() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        // The matcher compares a comma-separated field element by element.
        .and(headers(&header::ACCEPT, vec!["application/json;q=0.5", "application/ld+json"]))
        .and(header(&header::VIA, "1.1 broker-a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle"})))
        .expect(1)
        .mount(&server)
        .await;
    let accept = Accept::try_from_entries([
        Weighted::new(NgsiLdMediaType::Json).with_quality(Quality::try_from_thousandths(500).unwrap()),
        Weighted::new(NgsiLdMediaType::JsonLd),
    ])
    .unwrap();
    let via = Via::try_new(ViaEntry::new(
        ReceivedProtocol::new("1.1".parse().unwrap()),
        ReceivedBy::new("broker-a".parse().unwrap()),
    ))
    .unwrap();
    let client = client(&server.uri()).with_accept(accept).with_header(via).unwrap();
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let retrieved = client.entities().retrieve(&id).send().await.unwrap();
    assert_eq!(retrieved.body().id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
}

/// A header of the client's own, which no NGSI-LD rule names.
const USER: HeaderName = HeaderName::from_static("x-user");

#[tokio::test]
async fn a_derived_client_sends_a_header_field_no_typed_header_models() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(header(&USER, "fleet-operator"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri())
        .with_header_field(USER, HeaderValue::from_static("fleet-operator"))
        .unwrap();
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    assert!(client.entities().delete(&id).send().await.unwrap().is_done());
    assert!(matches!(
        client.with_header_field(header::LINK, HeaderValue::from_static("<x>")),
        Err(EndpointError::ReservedHeader { .. })
    ));
}

#[tokio::test]
async fn a_client_with_heads_answers_every_call_with_the_status_and_headers_of_its_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle"}"#, "application/json")
                .insert_header(&header::ETAG, "\"v1\"")
                .insert_header(&header::VARY, "Accept")
                .insert_header(NgsiLdTenant::name(), "fleet")
                .insert_header(&header::CACHE_CONTROL, "max-age=60")
                .insert_header(&header::STRICT_TRANSPORT_SECURITY, "max-age=31536000")
                .insert_header(&header::ALT_SVC, "h3=\":443\"; ma=86400")
                .insert_header(&header::CONTENT_ENCODING, "identity"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header(&header::LOCATION, "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567")
                .insert_header(NgsiLdTenant::name(), "fleet"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204).insert_header(NgsiLdTenant::name(), "fleet"))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri()).with_heads();
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let tenant = |head: &ResponseHead| head.typed::<NgsiLdTenant>().into_valid().map(|tenant| tenant.tenant().to_string());

    let retrieved = client.entities().retrieve(&id).send().await.unwrap();
    assert_eq!(retrieved.status, StatusCode::OK);
    assert_eq!(retrieved.outcome.body().id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(
        retrieved
            .head
            .typed::<ContentType>()
            .into_valid()
            .map(|content_type| content_type.to_string())
            .as_deref(),
        Some("application/json")
    );
    assert!(retrieved.head.typed::<ETag>().is_valid());
    assert!(retrieved.head.typed::<Vary>().into_valid().is_some_and(|vary| vary.iter_strs().eq(["Accept"])));
    assert_eq!(tenant(&retrieved.head).as_deref(), Some("fleet"));
    assert!(
        retrieved
            .head
            .typed::<CacheControl>()
            .into_valid()
            .is_some_and(|cache| cache.max_age().is_some())
    );
    assert!(retrieved.head.typed::<StrictTransportSecurity>().is_valid());
    assert!(retrieved.head.typed::<AltSvc>().into_valid().is_some_and(|alt| alt.alternatives().len() == 1));
    assert!(retrieved.head.typed::<ContentEncoding>().is_valid());

    let vehicle = vehicle();
    let created = client.entities().create(&vehicle).send().await.unwrap();
    assert_eq!(created.status, StatusCode::CREATED);
    assert!(created.outcome.is_created());
    assert_eq!(
        created.head.typed::<Location>().valid().map(|location| location.as_iri_ref().as_str()),
        Some("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567")
    );
    assert_eq!(tenant(&created.head).as_deref(), Some("fleet"));

    let deleted = client.entities().delete(&id).send().await.unwrap();
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert!(deleted.outcome.is_done());
    assert_eq!(tenant(&deleted.head).as_deref(), Some("fleet"));
}

#[tokio::test]
async fn a_client_with_heads_walks_pages_with_the_head_of_each() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle"}]))
                .insert_header(NgsiLdResultsCount::name(), "2"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("limit", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle"}]))
                .insert_header(
                    &header::LINK,
                    &link_field([link_value("/ngsi-ld/v1/entities?type=Vehicle&limit=1&offset=1", RelationType::Next)]),
                )
                .insert_header(&header::ETAG, "\"page-1\""),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri()).with_heads();
    let first = client.entities().query().entity_type("Vehicle").limit(1).send().await.unwrap();
    let mut pages = client.pages(first);
    let one = pages.next().await.unwrap().unwrap();
    assert!(one.head.typed::<ETag>().is_valid());
    let two = pages.next().await.unwrap().unwrap();
    assert_eq!(two.outcome.items().len(), 1);
    assert_eq!(two.outcome.total(), Some(2));
    assert!(pages.next().await.is_none());
}

#[tokio::test]
async fn a_derived_client_sends_a_profile_beside_and_inside_the_context_link() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(headers(
            &header::LINK,
            vec![
                &link_field([profiled_context_value(CONTEXT, "https://uri.etsi.org/ngsi-ld/profile/version")]),
                &link_field([link_value("https://uri.etsi.org/ngsi-ld/v1.9", RelationType::Profile)]),
            ],
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let profile = LinkValue::builder(IriRefBuf::new("https://uri.etsi.org/ngsi-ld/v1.9").unwrap())
        .rel(RelationType::Profile)
        .build();
    let client = client(&server.uri())
        .with_context_profile(IriBuf::new("https://uri.etsi.org/ngsi-ld/profile/version").unwrap())
        .unwrap()
        .with_link(Link::try_new(profile).unwrap())
        .unwrap();
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    assert!(client.entities().delete(&id).send().await.unwrap().is_done());
}

#[test]
fn a_derived_client_returns_to_the_defaults_it_was_given_up() {
    let client = client("http://broker.test");
    let derived = client
        .with_accept(NgsiLdMediaType::JsonLd)
        .with_prefer(Prefer::builder().version(SpecVersion::new(1, 9)).build().unwrap())
        .with_tenant("fleet".parse().unwrap());
    assert!(derived.endpoint().accept().is_some() && derived.endpoint().prefer().is_some() && derived.tenant().is_some());
    let reset = derived.without_accept().without_prefer().without_tenant();
    assert_eq!(reset.endpoint().accept(), None);
    assert_eq!(reset.endpoint().prefer(), None);
    assert_eq!(reset.tenant(), None);
}
