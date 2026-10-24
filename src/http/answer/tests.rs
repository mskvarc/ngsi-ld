//! The responses a context source writes, read back with the decoders a broker reads them with.

use crate::{
    http::{
        BatchEntityError,
        BatchOperationResult,
        CreateOutcome,
        Endpoint,
        NotUpdatedDetails,
        ResponseContext,
        ResponseError,
        UpdateOutcome,
        UpdateResult,
        UpsertOutcome,
        answer::{self, error_status},
        headers::{ContentType, Link, Location, NgsiLdResultsCount},
        operation::{AppendAttributes, BatchUpsert, CreateEntity, Operation, QueryEntities},
    },
    model::{Entity, EntityFragment, EntityId, ErrorType, Input, Lossless, NgsiLdMediaType, ProblemDetails, Representation},
    query::SelectorType,
};
use headers::HeaderMapExt;
use http::{Request, StatusCode};
use iri_rs::{IriRefBuf, iri};
use jstrict::Value as Json;
use pretty_assertions::assert_eq;
use std::str;
use url::Url;

/// The endpoint the forwarding broker sends its requests through.
fn endpoint() -> Endpoint {
    Endpoint::builder()
        .base_url(Url::parse("http://source.test/ngsi-ld/v1/").unwrap())
        .build()
        .unwrap()
}

/// A vehicle the context source holds.
fn vehicle() -> Entity<Input> {
    Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", 80)
        .build()
        .unwrap()
}

#[test]
fn a_list_is_a_json_array_with_the_context_link_and_reads_back_as_a_page() {
    let vehicle = vehicle();
    let mut response = answer::list([vehicle.render(Representation::Normalized)], &Link::core_context().unwrap());
    response.headers_mut().typed_insert(NgsiLdResultsCount::new(1));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().typed_get::<ContentType>(), Some(ContentType::NgsiLd(NgsiLdMediaType::Json)));
    assert_eq!(response.headers().typed_get::<Link>(), Some(Link::core_context().unwrap()));
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let query = QueryEntities::builder().entity_type(&vehicles).format(Lossless::Normalized).build();
    let request = query.to_request(&endpoint()).unwrap();
    let page = query.decode(&ResponseContext::new(&request), response).unwrap();
    assert_eq!(page.total(), Some(1));
    assert_eq!(
        page.into_items().into_iter().map(|entity| entity.into_input().unwrap()).collect::<Vec<_>>(),
        [vehicle]
    );
}

#[test]
fn an_empty_list_is_an_empty_json_array() {
    let response = answer::list([], &Link::core_context().unwrap());
    assert_eq!(response.body().as_ref(), b"[]");
}

#[test]
fn a_document_carries_its_context_link() {
    let response = answer::document(&vehicle().render(Representation::Concise), &Link::core_context().unwrap());
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().typed_get::<Link>().is_some());
    assert_eq!(response.body().as_ref(), br#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":80}"#);
}

#[test]
fn a_creation_carries_its_location_and_reads_back_as_created() {
    let vehicle = vehicle();
    let location = Location::new(IriRefBuf::new("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567".to_owned()).unwrap()).unwrap();
    let response = answer::created(location);
    assert!(response.body().is_empty());
    assert!(response.headers().typed_get::<Link>().is_none());
    let create = CreateEntity::builder().entity(&vehicle).build();
    let request = create.to_request(&endpoint()).unwrap();
    let CreateOutcome::Created { location } = create.decode(&ResponseContext::new(&request), response).unwrap() else {
        panic!("created");
    };
    assert_eq!(location.valid().map(IriRefBuf::as_str), Some("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"));
}

#[test]
fn no_content_is_an_empty_204_that_reads_back_as_done() {
    let response = answer::no_content();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.headers().is_empty());
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let fragment = EntityFragment::builder().property("color", "red").build().unwrap();
    let append = AppendAttributes::builder().id(&id).fragment(&fragment).build();
    let request = append.to_request(&endpoint()).unwrap();
    assert_eq!(append.decode(&ResponseContext::new(&request), response).unwrap(), UpdateOutcome::Done);
}

#[test]
fn an_update_result_is_a_207_without_link_and_reads_back_as_a_partial_outcome() {
    let result = UpdateResult {
        updated: vec!["https://example.org/color".parse().unwrap()],
        not_updated: vec![NotUpdatedDetails {
            attribute_name: "https://example.org/speed".parse().unwrap(),
            reason: "attribute exists".to_owned(),
            registration_id: None,
        }],
    };
    let response = answer::update_result(&result);
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);
    assert!(response.headers().typed_get::<Link>().is_none());
    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    let fragment = EntityFragment::builder().property("color", "red").build().unwrap();
    let append = AppendAttributes::builder().id(&id).fragment(&fragment).build();
    let request = append.to_request(&endpoint()).unwrap();
    assert_eq!(
        append.decode(&ResponseContext::new(&request), response).unwrap(),
        UpdateOutcome::Partial(result)
    );
}

#[test]
fn a_batch_result_is_a_207_and_reads_back_as_a_partial_outcome() {
    let result = BatchOperationResult {
        success: vec![iri!("urn:ngsi-ld:Vehicle:A4567").into()],
        errors: vec![BatchEntityError {
            entity_id: iri!("urn:ngsi-ld:Vehicle:B9211").into(),
            error: ProblemDetails::builder()
                .error_type(ErrorType::BadRequestData)
                .title("Bad request data")
                .detail("speed is not a number")
                .build(),
            registration_id: None,
        }],
    };
    let response = answer::batch_result(&result);
    assert_eq!(response.status(), StatusCode::MULTI_STATUS);
    assert!(response.headers().typed_get::<Link>().is_none());
    let entities = [vehicle()];
    let upsert = BatchUpsert::builder().entities(&entities).build();
    let request = upsert.to_request(&endpoint()).unwrap();
    assert_eq!(
        upsert.decode(&ResponseContext::new(&request), response).unwrap(),
        UpsertOutcome::Partial(result)
    );
}

/// The problem of a missing vehicle, with `status` as its `status` member.
fn vehicle_not_found(status: Option<u16>) -> ProblemDetails {
    ProblemDetails::builder()
        .error_type(ErrorType::ResourceNotFound)
        .title("Resource not found.")
        .detail("urn:ngsi-ld:Vehicle:B9211 was not found")
        .maybe_status(status)
        .build()
}

#[test]
fn a_problem_is_answered_with_its_status_and_no_link_and_reads_back_as_the_problem() {
    let response = answer::problem(StatusCode::NOT_FOUND, vehicle_not_found(Some(404)));
    assert_eq!(response.headers().typed_get::<ContentType>(), Some(ContentType::NgsiLd(NgsiLdMediaType::Json)));
    assert!(response.headers().typed_get::<Link>().is_none());
    let query = QueryEntities::builder().format(Lossless::Normalized).build();
    let request = Request::new(());
    let Err(ResponseError::Problem { status, details, .. }) = query.decode(&ResponseContext::new(&request), response) else {
        panic!("a problem");
    };
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(*details, vehicle_not_found(Some(404)));
}

#[test]
fn the_status_member_of_a_problem_is_the_status_of_its_answer() {
    for held in [None, Some(500)] {
        let response = answer::problem(StatusCode::NOT_FOUND, vehicle_not_found(held));
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body: Json = str::from_utf8(response.body()).unwrap().parse().unwrap();
        assert_eq!(ProblemDetails::from_json(&body), vehicle_not_found(Some(404)), "{held:?}");
    }
}

#[test]
fn every_error_type_of_table_6_3_2_1_maps_to_its_status() {
    let table = [
        (ErrorType::AlreadyExists, StatusCode::CONFLICT),
        (ErrorType::BadRequestData, StatusCode::BAD_REQUEST),
        (ErrorType::InternalError, StatusCode::INTERNAL_SERVER_ERROR),
        (ErrorType::InvalidRequest, StatusCode::BAD_REQUEST),
        (ErrorType::LdContextNotAvailable, StatusCode::GATEWAY_TIMEOUT),
        (ErrorType::NoMultiTenantSupport, StatusCode::NOT_IMPLEMENTED),
        (ErrorType::NonexistentTenant, StatusCode::NOT_FOUND),
        (ErrorType::OperationNotSupported, StatusCode::UNPROCESSABLE_ENTITY),
        (ErrorType::ResourceNotFound, StatusCode::NOT_FOUND),
        (ErrorType::TooComplexQuery, StatusCode::FORBIDDEN),
        (ErrorType::TooManyResults, StatusCode::FORBIDDEN),
    ];
    for (error_type, status) in table {
        assert_eq!(error_status(&error_type), Some(status), "{error_type}");
    }
}

#[test]
fn a_type_outside_table_6_3_2_1_has_no_status() {
    assert_eq!(error_status(&ErrorType::Conflict), None);
    assert_eq!(error_status(&"https://example.org/errors/Busy".parse().unwrap()), None);
}
