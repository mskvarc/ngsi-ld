//! The batch operations (clauses 5.6.7 to 5.6.10, 5.6.20, 6.14 to 6.17 and 6.31), and Merge Entity with a fragment
//! that names its entity.

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, json, link_of, response},
    links::context_link,
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, StatusCode, header};
use iri_rs::iri;
use ngsi_ld::{
    http::{
        BatchCreateOutcome,
        BatchOutcome,
        BodyError,
        ContextDelivery,
        RequestError,
        ResponseError,
        UncheckedDocument,
        UncheckedRequest,
        UpsertOutcome,
        headers::NgsiLdTenant,
        operation::{ArgumentError, BatchCreate, BatchDelete, BatchMerge, BatchUpdate, BatchUpsert, MergeEntity, Operation, Overwrite, UpsertMode},
    },
    model::{Complete, Entity, EntityFragment, EntityId, ErrorType, Input, Lossless, Tenant},
};
use pretty_assertions::assert_eq;

/// `urn:ngsi-ld:Vehicle:A4567`.
const A: &str = "urn:ngsi-ld:Vehicle:A4567";
/// `urn:ngsi-ld:Vehicle:B9211`.
const B: &str = "urn:ngsi-ld:Vehicle:B9211";

/// The entity `id` of type `Vehicle` with the Property `speed` of value `speed`.
fn vehicle(id: &str, speed: i64) -> Entity<Input> {
    Entity::builder().id(id).entity_type("Vehicle").property("speed", speed).build().unwrap()
}

/// The normalized text of `vehicle(id, speed)`.
fn vehicle_text(id: &str, speed: i64) -> String {
    format!(r#"{{"id":"{id}","type":"Vehicle","speed":{{"type":"Property","value":{speed}}}}}"#)
}

/// A fragment of `A`: it stops and is no longer parked.
fn stopped() -> EntityFragment<Complete> {
    EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
        .property("speed", 0)
        .delete("isParked")
        .build()
        .unwrap()
}

/// The normalized text of `stopped()`.
const STOPPED: &str = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":0},"isParked":"urn:ngsi-ld:null"}"#;

/// Parses an entity id.
fn id(text: &str) -> EntityId {
    text.parse().unwrap()
}

/// A batch create of `entities` in normalized form.
fn create(entities: &[Entity<Input>]) -> BatchCreate<'_> {
    BatchCreate::builder().entities(entities).build()
}

/// Asserts the request is a POST to `entityOperations/{resource}` with the given query string.
fn assert_batch_target(request: &Request<Bytes>, resource: &str, query: Option<&str>) {
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/entityOperations/{resource}"));
    assert_eq!(request.uri().query(), query);
}

#[test]
fn a_batch_create_posts_every_entity_in_order_with_the_json_headers() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let request = create(&entities).to_request(&endpoint()).unwrap();
    assert_batch_target(&request, "create", None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
    let expected = format!("[{},{}]", vehicle_text(A, 80), vehicle_text(B, 60));
    assert_eq!(request.body(), &Bytes::from(expected));
}

#[test]
fn two_instances_of_one_entity_are_both_sent_oldest_first() {
    let entities = [vehicle(A, 80), vehicle(A, 90)];
    let request = create(&entities).to_request(&endpoint()).unwrap();
    let expected = format!("[{},{}]", vehicle_text(A, 80), vehicle_text(A, 90));
    assert_eq!(request.body(), &Bytes::from(expected));
}

#[test]
fn a_concise_batch_create_writes_every_entity_concise() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let request = BatchCreate::builder()
        .entities(&entities)
        .form(Lossless::Concise)
        .build()
        .to_request(&endpoint())
        .unwrap();
    let expected = format!(r#"[{{"id":"{A}","type":"Vehicle","speed":80}},{{"id":"{B}","type":"Vehicle","speed":60}}]"#);
    assert_eq!(request.body(), &Bytes::from(expected));
}

#[test]
fn under_ld_json_body_every_element_carries_the_context_last() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let request = create(&entities).to_request(&endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let context = r#","@context":"https://example.org/context.jsonld"}"#;
    let with_context = |text: String| format!("{}{context}", text.strip_suffix('}').unwrap());
    let expected = format!("[{},{}]", with_context(vehicle_text(A, 80)), with_context(vehicle_text(B, 60)));
    assert_eq!(request.body(), &Bytes::from(expected));
}

#[test]
fn an_empty_batch_is_refused_before_sending_by_every_batch_operation() {
    let endpoint = endpoint();
    let refusals = [
        create(&[]).to_request(&endpoint).map(drop),
        BatchUpsert::builder().entities(&[]).build().to_request(&endpoint).map(drop),
        BatchUpdate::builder().fragments(&[]).build().to_request(&endpoint).map(drop),
        BatchMerge::builder().fragments(&[]).build().to_request(&endpoint).map(drop),
        BatchDelete { ids: &[] }.to_request(&endpoint).map(drop),
    ];
    for refusal in refusals {
        assert!(matches!(refusal, Err(RequestError::Argument(ArgumentError::EmptyBatch))), "{refusal:?}");
    }
}

#[test]
fn a_batch_upsert_sends_no_options_by_default_and_options_update_in_update_mode() {
    let entities = [vehicle(A, 80)];
    let upsert = BatchUpsert::builder().entities(&entities).build();
    let request = upsert.to_request(&endpoint()).unwrap();
    assert_batch_target(&request, "upsert", None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(request.body(), &Bytes::from(format!("[{}]", vehicle_text(A, 80))));
    let request = BatchUpsert {
        mode: UpsertMode::Update,
        ..upsert
    }
    .to_request(&endpoint())
    .unwrap();
    assert_batch_target(&request, "upsert", Some("options=update"));
}

#[test]
fn a_batch_update_sends_the_fragments_and_options_no_overwrite_when_keeping() {
    let fragments = [stopped()];
    let update = BatchUpdate::builder().fragments(&fragments).build();
    let request = update.to_request(&endpoint()).unwrap();
    assert_batch_target(&request, "update", None);
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(request.body(), &Bytes::from(format!("[{STOPPED}]")));
    let request = BatchUpdate {
        overwrite: Overwrite::Keep,
        ..update
    }
    .to_request(&endpoint())
    .unwrap();
    assert_batch_target(&request, "update", Some("options=noOverwrite"));
}

#[test]
fn a_batch_merge_sends_the_fragments_in_order_with_no_parameter() {
    let slowed = EntityFragment::for_entity(B, "Vehicle").property("speed", 10).build().unwrap();
    let fragments = [stopped(), slowed];
    let request = BatchMerge::builder().fragments(&fragments).build().to_request(&endpoint()).unwrap();
    assert_batch_target(&request, "merge", None);
    let expected = format!("[{STOPPED},{}]", vehicle_text(B, 10));
    assert_eq!(request.body(), &Bytes::from(expected));
}

#[test]
fn a_batch_delete_sends_the_ids_as_json_with_the_link_header() {
    let ids = [id(A), id(B)];
    let request = BatchDelete { ids: &ids }.to_request(&endpoint()).unwrap();
    assert_batch_target(&request, "delete", None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
    assert_eq!(request.body(), &Bytes::from(format!(r#"["{A}","{B}"]"#)));
}

#[test]
fn a_batch_delete_under_ld_json_body_is_still_json_with_the_link_header() {
    let ids = [id(A), id(B)];
    let request = BatchDelete { ids: &ids }.to_request(&endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(request.body(), &Bytes::from(format!(r#"["{A}","{B}"]"#)));
}

#[test]
fn a_repeated_id_is_sent_twice_in_order() {
    let ids = [id(A), id(A)];
    let request = BatchDelete { ids: &ids }.to_request(&endpoint()).unwrap();
    assert_eq!(request.body(), &Bytes::from(format!(r#"["{A}","{A}"]"#)));
}

#[test]
fn a_batch_upsert_on_a_tenant_sends_the_tenant_header() {
    let entities = [vehicle(A, 80)];
    let tenant: Tenant = "t1".parse().unwrap();
    let request = BatchUpsert::builder()
        .entities(&entities)
        .build()
        .to_request(&endpoint().with_tenant(tenant))
        .unwrap();
    assert_eq!(header_value(&request, NgsiLdTenant::name()), Some("t1"));
}

#[test]
fn merge_entity_accepts_a_fragment_that_names_its_entity() {
    let fragment = stopped();
    let target = id(A);
    let request = MergeEntity::builder().id(&target).fragment(&fragment).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(request.body(), &Bytes::from(STOPPED));
}

#[test]
fn a_201_to_a_batch_create_lists_the_created_ids_in_the_order_received() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let outcome = create(&entities).decode(&context(), response(201, &[], &format!(r#"["{B}","{A}"]"#))).unwrap();
    assert_eq!(outcome, BatchCreateOutcome::Created(vec![id(B), id(A)]));
}

#[test]
fn a_207_to_a_batch_create_reports_each_entity_in_error() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let body = format!(r#"{{"success":["{A}"],"errors":[{{"entityId":"{B}","error":{{"type":"https://uri.etsi.org/ngsi-ld/errors/AlreadyExists"}}}}]}}"#);
    let outcome = create(&entities).decode(&context(), response(207, &[], &body)).unwrap();
    let BatchCreateOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.success, [id(A)]);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].entity_id, id(B));
    assert_eq!(result.errors[0].error.error_type, Some(ErrorType::AlreadyExists));
}

#[test]
fn a_201_whose_body_is_not_a_list_of_ids_is_a_body_error() {
    let entities = [vehicle(A, 80)];
    for body in [r#"{"a":1}"#, r#"["not a uri"]"#] {
        let error = create(&entities).decode(&context(), response(201, &[], body)).unwrap_err();
        assert!(
            matches!(&error, ResponseError::Body { status: StatusCode::CREATED, source, .. } if matches!(**source, BodyError::Shape(_))),
            "{body}: {error:?}"
        );
    }
}

#[test]
fn a_204_to_a_batch_create_is_an_unexpected_status() {
    let entities = [vehicle(A, 80)];
    let error = create(&entities).decode(&context(), response(204, &[], "")).unwrap_err();
    assert!(
        matches!(
            error,
            ResponseError::UnexpectedStatus {
                status: StatusCode::NO_CONTENT,
                ..
            }
        ),
        "{error:?}"
    );
}

#[test]
fn a_batch_upsert_answers_created_updated_or_partial() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let upsert = BatchUpsert::builder().entities(&entities).build();
    let created = upsert.decode(&context(), response(201, &[], &format!(r#"["{B}"]"#))).unwrap();
    assert_eq!(created, UpsertOutcome::Created(vec![id(B)]));
    assert_eq!(upsert.decode(&context(), response(204, &[], "")).unwrap(), UpsertOutcome::Updated);
    let body = format!(r#"{{"success":["{B}"],"errors":[{{"entityId":"{A}","error":{{"type":"https://uri.etsi.org/ngsi-ld/errors/BadRequestData"}}}}]}}"#);
    let partial = upsert.decode(&context(), response(207, &[], &body)).unwrap();
    let UpsertOutcome::Partial(result) = partial else {
        panic!("expected a partial result, got {partial:?}")
    };
    assert_eq!(result.errors[0].entity_id, id(A));
}

#[test]
fn batch_update_merge_and_delete_answer_204_done_and_207_partial() {
    let fragments = [stopped()];
    let ids = [id(A), id(B)];
    let update = BatchUpdate::builder().fragments(&fragments).build();
    let merge = BatchMerge::builder().fragments(&fragments).build();
    let delete = BatchDelete { ids: &ids };
    assert_eq!(update.decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
    assert_eq!(merge.decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
    assert_eq!(delete.decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
    let body = format!(r#"{{"success":["{A}"],"errors":[{{"entityId":"{B}","error":{{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound"}}}}]}}"#);
    let outcome = delete.decode(&context(), response(207, &[], &body)).unwrap();
    let BatchOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.errors[0].entity_id, id(B));
    assert_eq!(result.errors[0].error.error_type, Some(ErrorType::ResourceNotFound));
}

#[test]
fn a_batch_error_keeps_the_registration_of_a_failed_distributed_operation() {
    let ids = [id(A)];
    let body = format!(
        r#"{{"success":[],"errors":[{{"entityId":"{A}","error":{{"type":"https://uri.etsi.org/ngsi-ld/errors/Conflict"}},"registrationId":"urn:ngsi-ld:ContextSourceRegistration:r1"}}]}}"#
    );
    let outcome = BatchDelete { ids: &ids }.decode(&context(), response(207, &[], &body)).unwrap();
    let BatchOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(
        result.errors[0].registration_id.as_ref().map(ToString::to_string).as_deref(),
        Some("urn:ngsi-ld:ContextSourceRegistration:r1")
    );
}

#[test]
fn a_400_to_a_batch_create_is_a_bad_request_data_problem() {
    let entities = [vehicle(A, 80)];
    let body = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/BadRequestData","title":"Bad request data"}"#;
    let error = create(&entities)
        .decode(&context(), response(400, &[(&header::CONTENT_TYPE, "application/json")], body))
        .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Problem { status: StatusCode::BAD_REQUEST, details, .. } if details.error_type == Some(ErrorType::BadRequestData)),
        "{error:?}"
    );
}

#[test]
fn a_200_to_a_batch_delete_is_an_unexpected_status() {
    let ids = [id(A)];
    let error = BatchDelete { ids: &ids }.decode(&context(), response(200, &[], "")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { status: StatusCode::OK, .. }), "{error:?}");
}

#[test]
fn an_empty_array_and_a_null_element_are_sent_through_the_unchecked_path() {
    let entities = [vehicle(A, 80)];
    let unchecked = || UncheckedRequest::from_operation(&create(&entities), &endpoint()).unwrap();
    let empty = unchecked().body(Bytes::from_static(b"[]")).into_request();
    assert_eq!(empty.body().as_ref(), b"[]");
    assert_eq!(header_value(&empty, &header::CONTENT_LENGTH), Some("2"));
    let null = unchecked().body(Bytes::from_static(b"[null]")).into_request();
    assert_eq!(null.body().as_ref(), b"[null]");
    assert_eq!(header_value(&null, &header::CONTENT_LENGTH), Some("6"));
}

#[test]
fn documents_replace_a_batch_body_keeping_the_context_of_each_element() {
    let entities = [vehicle(A, 80), vehicle(B, 60)];
    let documents = [
        UncheckedDocument::from_entity(&entities[0], Lossless::Normalized),
        UncheckedDocument::from_entity(&entities[1], Lossless::Normalized).remove_member("type"),
    ];
    let request = UncheckedRequest::from_operation(&create(&entities), &endpoint())
        .unwrap()
        .documents(&documents)
        .into_request();
    let untyped = format!(r#"{{"id":"{B}","speed":{{"type":"Property","value":60}}}}"#);
    assert_eq!(request.body(), &Bytes::from(format!("[{},{untyped}]", vehicle_text(A, 80))));
    let request = UncheckedRequest::from_operation(&create(&entities), &endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap()
        .documents(&documents)
        .into_request();
    let body = json(request.body());
    let elements = body.as_array().unwrap();
    assert_eq!(elements.len(), 2);
    for element in elements {
        let object = element.as_object().unwrap();
        assert_eq!(
            object.get_unique("@context").unwrap().and_then(|context| context.as_str()),
            Some("https://example.org/context.jsonld")
        );
    }
    assert!(elements[1].as_object().unwrap().get("type").next().is_none());
}
