//! The unchecked path: documents and headers sent as given, as a negative test needs.

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, json, link_of, response, vehicle},
    links::context_link,
};
use bytes::Bytes;
use http::header;
use jstrict::Value as Json;
use ngsi_ld::{
    http::{
        ContextDelivery,
        ResponseError,
        UncheckedDocument,
        UncheckedRequest,
        operation::{CreateEntity, Operation},
    },
    model::{ErrorType, Lossless, RawJson},
};
use pretty_assertions::assert_eq;

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
