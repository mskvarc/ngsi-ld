//! The unchecked path: documents and headers sent as given, as a negative test needs.

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, json, link_of, response, vehicle},
    links::context_link,
    registration_fixtures::{R, id, postpone, vehicle_source},
};
use bytes::Bytes;
use http::{HeaderName, HeaderValue, Method, header};
use jstrict::Value as Json;
use ngsi_ld::{
    http::{
        ContextDelivery,
        ResponseError,
        UncheckedDocument,
        UncheckedRequest,
        operation::{
            BatchCreate,
            CreateEntity,
            CreateSubscription,
            Operation,
            QueryTemporalEvolution,
            RegisterContextSource,
            UpdateRegistration,
            UpdateSubscription,
        },
    },
    model::{ErrorType, Input, Lossless, RawJson},
    query::{SelectorType, temporal_query::TemporalQuery},
    subscription::{Activation, Subscription, SubscriptionFragment, SubscriptionId},
};
use pretty_assertions::assert_eq;
use std::str;

#[test]
fn an_entity_without_type_is_sent_with_the_headers_of_create() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let missing_type = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized).remove_member("type");
    let request = UncheckedRequest::try_from_operation(&create, &endpoint())
        .unwrap()
        .document(&missing_type)
        .into_request();
    let body = json(request.body());
    assert!(body.as_object().unwrap().get("type").next().is_none());
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
}

#[test]
fn a_link_header_alongside_an_ld_json_body_is_sent_and_the_rejection_decodes() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let endpoint = endpoint_with(ContextDelivery::LdJsonBody);
    let request = UncheckedRequest::try_from_operation(&create, &endpoint)
        .unwrap()
        .typed_header(context_link())
        .into_request();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(json(request.body()).as_object().unwrap().get("@context").next().is_some());
    let error = create
        .decode(
            &context(),
            response(
                400,
                &[],
                r#"{"type": "https://uri.etsi.org/ngsi-ld/errors/BadRequestData", "title": "Link header with ld+json"}"#,
            ),
        )
        .unwrap_err();
    assert!(matches!(error, ResponseError::Problem { details, .. } if details.error_type == Some(ErrorType::BadRequestData)));
}

#[test]
fn a_body_that_is_not_json_is_sent_as_given() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let request = UncheckedRequest::try_from_operation(&create, &endpoint())
        .unwrap()
        .body(Bytes::from_static(b"{not json"))
        .into_request();
    assert_eq!(request.body().as_ref(), b"{not json");
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some("9"));
}

#[test]
fn a_document_replacing_an_ld_json_body_keeps_its_context() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let missing_type = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized).remove_member("type");
    let request = UncheckedRequest::try_from_operation(&create, &endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap()
        .document(&missing_type)
        .into_request();
    let body = json(request.body());
    let object = body.as_object().unwrap();
    assert!(object.get("type").next().is_none());
    assert_eq!(object.get("@context").collect::<Vec<_>>(), [&Json::from("https://example.org/context.jsonld")]);
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
}

#[test]
fn a_subscription_without_its_notification_is_sent_with_the_headers_of_create() {
    let subscription: Subscription<Input> =
        Subscription::parse(r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#)
            .unwrap();
    let create = CreateSubscription { subscription: &subscription };
    let broken = UncheckedDocument::try_from(subscription.render()).unwrap().remove_member("notification");
    let request = UncheckedRequest::try_from_operation(&create, &endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap()
        .document(&broken)
        .into_request();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/subscriptions");
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(
        str::from_utf8(request.body()).unwrap(),
        r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"@context":"https://example.org/context.jsonld"}"#
    );
}

#[test]
fn a_subscription_fragment_with_a_member_of_the_wrong_kind_is_sent_with_the_headers_of_update() {
    let fragment = SubscriptionFragment::builder().activation(Activation::Paused).build().unwrap();
    let id: SubscriptionId = "urn:ngsi-ld:Subscription:mySubscription".parse().unwrap();
    let update = UpdateSubscription { id: &id, fragment: &fragment };
    let broken = UncheckedDocument::try_from(fragment.render()).unwrap().set_text("isActive", "no");
    let request = UncheckedRequest::try_from_operation(&update, &endpoint())
        .unwrap()
        .document(&broken)
        .into_request();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription");
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(str::from_utf8(request.body()).unwrap(), r#"{"type":"Subscription","isActive":"no"}"#);
}

#[test]
fn a_registration_without_its_endpoint_is_sent_with_the_headers_of_register() {
    let registration = vehicle_source();
    let register = RegisterContextSource { registration: &registration };
    let broken = UncheckedDocument::try_from(registration.render()).unwrap().remove_member("endpoint");
    let request = UncheckedRequest::try_from_operation(&register, &endpoint())
        .unwrap()
        .document(&broken)
        .into_request();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/csourceRegistrations");
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(
        str::from_utf8(request.body()).unwrap(),
        r#"{"type":"ContextSourceRegistration","information":[{"entities":[{"type":"Vehicle"}],"propertyNames":["speed"]}]}"#
    );
}

#[test]
fn a_registration_fragment_with_an_expiry_that_is_not_a_timestamp_is_sent_with_the_headers_of_update() {
    let fragment = postpone();
    let target = id(R);
    let update = UpdateRegistration {
        id: &target,
        fragment: &fragment,
    };
    let broken = UncheckedDocument::try_from(fragment.render()).unwrap().set_text("expiresAt", "tomorrow");
    let request = UncheckedRequest::try_from_operation(&update, &endpoint())
        .unwrap()
        .document(&broken)
        .into_request();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/csourceRegistrations/{R}"));
    assert_eq!(
        str::from_utf8(request.body()).unwrap(),
        r#"{"type":"ContextSourceRegistration","expiresAt":"tomorrow"}"#
    );
}

#[test]
fn a_document_sent_as_is_drops_the_context_and_one_with_its_own_context_keeps_it() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let unchecked = || UncheckedRequest::try_from_operation(&create, &endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    let document = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized);
    let without = unchecked().document_as_is(&document).into_request();
    assert!(json(without.body()).as_object().unwrap().get("@context").next().is_none());
    let own: RawJson = r#""https://example.org/other.jsonld""#.parse().unwrap();
    let with_own = unchecked().document(&document.set_member("@context", &own)).into_request();
    assert_eq!(
        json(with_own.body()).as_object().unwrap().get("@context").collect::<Vec<_>>(),
        [&Json::from("https://example.org/other.jsonld")]
    );
}

#[test]
fn a_temporal_query_without_its_temporal_query_is_sent_with_the_headers_of_the_query() {
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let since = TemporalQuery::after("2018-08-01T12:00:00Z".parse().unwrap());
    let query = QueryTemporalEvolution::builder()
        .entity_type(&vehicles)
        .temporal_q(&since)
        .format(Lossless::Normalized)
        .build();
    let without_timerel = "http://broker.test/ngsi-ld/v1/temporal/entities?type=Vehicle".parse().unwrap();
    let request = UncheckedRequest::try_from_operation(&query, &endpoint())
        .unwrap()
        .uri(without_timerel)
        .into_request();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().query(), Some("type=Vehicle"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
}

#[test]
fn a_preference_the_specification_does_not_define_is_sent_beside_the_typed_ones() {
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let since = TemporalQuery::after("2018-08-01T12:00:00Z".parse().unwrap());
    let query = QueryTemporalEvolution::builder()
        .entity_type(&vehicles)
        .temporal_q(&since)
        .format(Lossless::Normalized)
        .build();
    let request = UncheckedRequest::try_from_operation(&query, &endpoint())
        .unwrap()
        .append_header(HeaderName::from_static("prefer"), HeaderValue::from_static("respond-async"))
        .into_request();
    assert_eq!(header_value(&request, &HeaderName::from_static("prefer")), Some("respond-async"));
    assert_eq!(link_of(&request), Some(context_link()));
}

#[test]
fn a_batch_whose_documents_each_carry_a_context_of_their_own_is_sent_as_given() {
    let vehicle = vehicle();
    let entities = [vehicle.clone()];
    let create = BatchCreate::builder().entities(&entities).build();
    let first: RawJson = r#""https://example.org/first.jsonld""#.parse().unwrap();
    let second: RawJson = r#"["https://example.org/second.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context.jsonld"]"#
        .parse()
        .unwrap();
    let document = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized);
    let request = UncheckedRequest::try_from_operation(&create, &endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap()
        .documents(&[document.clone().set_member("@context", &first), document.set_member("@context", &second)])
        .into_request();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let body = json(request.body());
    let contexts: Vec<_> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|document| document.as_object().unwrap().get("@context").collect::<Vec<_>>())
        .collect();
    assert_eq!(
        contexts,
        [
            vec![&Json::from("https://example.org/first.jsonld")],
            vec![&json(
                br#"["https://example.org/second.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context.jsonld"]"#
            )]
        ]
    );
}
