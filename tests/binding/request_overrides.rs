//! What an endpoint states on every request besides the operation's own parameters: any `@context` (clause 6.3.5),
//! an `Accept` replacing the operation's (clause 6.3.4) and further headers; and the head every error keeps.

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, link_of, response, vehicle, vehicle_id},
    links::profiled_context_value,
    query_vehicles,
    retrieve_entity,
};
use headers::{Allow, Header, IfNoneMatch, RetryAfter};
use http::{HeaderName, HeaderValue, Method, StatusCode, header};
use iri_rs::{IriBuf, IriRefBuf};
use jstrict::Value as Json;
use ngsi_ld::{
    http::{
        ContextDelivery,
        ContextDeliveryError,
        Endpoint,
        EndpointError,
        RequestError,
        ResponseContext,
        ResponseError,
        headers::{Accept, Link, NgsiLdTenant},
        operation::{BatchDelete, CreateEntity, DeleteEntity, Operation, Simplified},
    },
    model::{CORE_CONTEXT, Context, ContextEntry, InlineContext, Lossless, NgsiLdMediaType},
    query::SelectorType,
};
use pretty_assertions::assert_eq;
use std::str;
use url::Url;

/// A context of a reference and an inline definition, which no `Link` header can carry.
fn mixed_context() -> Context {
    Context::from(vec![
        ContextEntry::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap()),
        ContextEntry::from(r#"{"@vocab":"https://example.org/terms/"}"#.parse::<InlineContext>().unwrap()),
    ])
}

/// The test endpoint with `context`, delivering it as `delivery` says.
fn with_context(context: Context, delivery: ContextDelivery) -> Endpoint {
    endpoint_with(delivery).with_context(context).unwrap()
}

#[test]
fn a_context_that_is_not_one_reference_travels_whole_in_an_ld_json_body() {
    let endpoint = with_context(mixed_context(), ContextDelivery::LdJsonBody);
    let vehicle = vehicle();
    let request = CreateEntity::builder().entity(&vehicle).build().to_request(&endpoint).unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(header_value(&request, &header::LINK), None);
    let body: Json = str::from_utf8(request.body()).unwrap().parse().unwrap();
    assert_eq!(
        body.as_object().unwrap().get_unique("@context").unwrap().unwrap(),
        &r#"["https://example.org/context.jsonld",{"@vocab":"https://example.org/terms/"}]"#.parse::<Json>().unwrap()
    );
}

#[test]
fn a_context_that_is_not_one_reference_cannot_travel_in_a_link_header() {
    let id = vehicle_id();
    let get = retrieve_entity(&id, Lossless::Normalized).to_request(&with_context(mixed_context(), ContextDelivery::LdJsonBody));
    assert!(matches!(get, Err(RequestError::Context(ContextDeliveryError::NotLinkable))));
    let vehicle = vehicle();
    let create = CreateEntity::builder()
        .entity(&vehicle)
        .build()
        .to_request(&with_context(mixed_context(), ContextDelivery::JsonWithLink));
    assert!(matches!(create, Err(RequestError::Context(ContextDeliveryError::NotLinkable))));
}

#[test]
fn a_batch_delete_holds_no_term_so_it_goes_without_a_context_that_no_link_can_carry() {
    let ids = [vehicle_id()];
    let request = BatchDelete { ids: &ids }
        .to_request(&with_context(mixed_context(), ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(header_value(&request, &header::LINK), None);
}

#[test]
fn a_relative_context_reference_in_a_context_of_several_entries_cannot_be_given_to_an_endpoint() {
    assert!(matches!(
        endpoint().with_context(Context::from(vec![
            ContextEntry::from(IriRefBuf::new("contexts/fleet.jsonld").unwrap()),
            ContextEntry::from(r#"{"speed": "https://example.org/speed"}"#.parse::<InlineContext>().unwrap()),
        ])),
        Err(EndpointError::RelativeContextReference { .. })
    ));
}

#[test]
fn a_single_relative_context_reference_is_sent_in_the_link_header_for_the_broker_to_resolve() {
    let link = Link::json_ld_context(IriRefBuf::new("/ngsi-ld/v1/jsonldContexts/fleet").unwrap()).unwrap();
    let endpoint = endpoint()
        .with_context(Context::from(IriRefBuf::new("/ngsi-ld/v1/jsonldContexts/fleet").unwrap()))
        .unwrap();
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build().to_request(&endpoint).unwrap();
    assert_eq!(header_value(&create, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&create).as_ref(), Some(&link));
    let get = retrieve_entity(&vehicle_id(), Lossless::Normalized).to_request(&endpoint).unwrap();
    assert_eq!(link_of(&get), Some(link));
}

#[test]
fn a_single_relative_context_reference_is_refused_in_an_ld_json_body() {
    let endpoint = endpoint()
        .with_context(Context::from(IriRefBuf::new("/ngsi-ld/v1/jsonldContexts/fleet").unwrap()))
        .unwrap()
        .with_context_delivery(ContextDelivery::LdJsonBody);
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build().to_request(&endpoint);
    assert!(matches!(
        create,
        Err(RequestError::Context(ContextDeliveryError::RelativeInBody { reference })) if reference.as_str() == "/ngsi-ld/v1/jsonldContexts/fleet"
    ));
    let ids = [vehicle_id()];
    let delete = BatchDelete { ids: &ids }.to_request(&endpoint).unwrap();
    assert_eq!(header_value(&delete, &header::CONTENT_TYPE), Some("application/json"));
}

#[test]
fn an_endpoint_accept_replaces_the_operation_accept_and_reaches_requests_that_read_no_body() {
    let endpoint = endpoint().with_accept(Some(Accept::new(NgsiLdMediaType::JsonLd)));
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized).to_request(&endpoint).unwrap();
    assert_eq!(header_value(&retrieve, &header::ACCEPT), Some("application/ld+json"));
    let delete = DeleteEntity::builder().id(&id).build().to_request(&endpoint).unwrap();
    assert_eq!(header_value(&delete, &header::ACCEPT), Some("application/ld+json"));
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified).to_request(&endpoint).unwrap();
    assert_eq!(header_value(&query, &header::ACCEPT), Some("application/ld+json"));
}

#[test]
fn an_endpoint_header_reaches_every_request_and_a_pipeline_header_is_refused() {
    let endpoint = endpoint().with_header(IfNoneMatch::any()).unwrap();
    let id = vehicle_id();
    let request = DeleteEntity::builder().id(&id).build().to_request(&endpoint).unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(header_value(&request, &header::IF_NONE_MATCH), Some("*"));
    assert!(matches!(
        endpoint.with_header(NgsiLdTenant::try_new("fleet".parse().unwrap()).unwrap()),
        Err(EndpointError::ReservedHeader { name }) if name == NgsiLdTenant::name()
    ));
}

/// A header of the client's own, which no NGSI-LD rule names.
const USER: HeaderName = HeaderName::from_static("x-user");

#[test]
fn an_endpoint_header_field_reaches_every_request() {
    let endpoint = endpoint().with_header_field(USER, HeaderValue::from_static("fleet-operator")).unwrap();
    let id = vehicle_id();
    let delete = DeleteEntity::builder().id(&id).build().to_request(&endpoint).unwrap();
    assert_eq!(header_value(&delete, &USER), Some("fleet-operator"));
    let retrieve = retrieve_entity(&id, Lossless::Normalized).to_request(&endpoint).unwrap();
    assert_eq!(header_value(&retrieve, &USER), Some("fleet-operator"));
}

#[test]
fn an_error_without_a_body_keeps_the_headers_of_its_head() {
    let id = vehicle_id();
    let error = DeleteEntity::builder()
        .id(&id)
        .build()
        .decode(&context(), response(405, &[(&header::ALLOW, "GET, POST")], ""))
        .unwrap_err();
    assert!(matches!(
        error,
        ResponseError::Status {
            status: StatusCode::METHOD_NOT_ALLOWED,
            ..
        }
    ));
    let allow = error.head().typed::<Allow>().into_valid().unwrap();
    assert_eq!(allow.iter().collect::<Vec<_>>(), [Method::GET, Method::POST]);
}

#[test]
fn a_problem_keeps_the_headers_of_its_head() {
    let id = vehicle_id();
    let problem = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/InternalError","title":"Busy"}"#;
    let error = retrieve_entity(&id, Lossless::Normalized)
        .decode(
            &context(),
            response(503, &[(&header::RETRY_AFTER, "120"), (NgsiLdTenant::name(), "fleet")], problem),
        )
        .unwrap_err();
    assert_eq!(error.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(error.head().typed::<RetryAfter>().is_valid());
    assert_eq!(
        error
            .head()
            .typed::<NgsiLdTenant>()
            .valid()
            .map(|tenant| tenant.tenant().to_string())
            .as_deref(),
        Some("fleet")
    );
}

#[test]
fn a_page_reads_its_first_and_last_links() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let request = query.to_request(&endpoint()).unwrap();
    let link = r#"<?type=Vehicle&offset=0>; rel="first", <?type=Vehicle&offset=40>; rel="last""#;
    let page = query
        .decode(&ResponseContext::new(&request), response(200, &[(&header::LINK, link)], "[]"))
        .unwrap();
    assert_eq!(
        page.first().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=0")
    );
    assert_eq!(
        page.last().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=40")
    );
}

#[test]
fn a_context_profile_is_sent_on_the_context_link_and_refused_where_no_link_travels() {
    let profile = IriBuf::new("https://uri.etsi.org/ngsi-ld/profile/version").unwrap();
    let id = vehicle_id();
    let bare = Endpoint::builder()
        .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
        .build()
        .unwrap()
        .with_context_profile(profile.clone())
        .unwrap();
    let retrieve = retrieve_entity(&id, Lossless::Normalized).to_request(&bare).unwrap();
    assert_eq!(
        link_of(&retrieve),
        Some(Link::try_new(profiled_context_value(CORE_CONTEXT.as_str(), "https://uri.etsi.org/ngsi-ld/profile/version")).unwrap())
    );
    let ld_json = endpoint_with(ContextDelivery::LdJsonBody).with_context_profile(profile).unwrap();
    let vehicle = vehicle();
    assert!(matches!(
        CreateEntity::builder().entity(&vehicle).build().to_request(&ld_json),
        Err(RequestError::Context(ContextDeliveryError::ProfileWithoutLink))
    ));
}
