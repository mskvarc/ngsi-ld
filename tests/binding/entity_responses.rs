//! How the responses of the entity operations decode: outcomes, problems, pages and their headers.

use crate::{
    fixtures::{context, endpoint, header_value, link_of, response, vehicle, vehicle_id},
    links::{context_link, link_field, link_value},
    query_vehicles,
    retrieve_entity,
};
use headers::Header;
use http::{HeaderValue, StatusCode, header};
use iri_rs::IriRefBuf;
use ngsi_ld::{
    http::{
        BatchOutcome,
        BodyError,
        CreateOutcome,
        ResponseContext,
        ResponseError,
        headers::{LinkValue, NgsiLdResultsCount, NgsiLdTenant, OptionalHeader, PreferenceApplied, RelationType},
        operation::{CreateEntity, DeleteEntity, MergeEntity, Operation, QueryEntities, Simplified},
    },
    model::{APPLICATION_LD_JSON, EntityFragment, ErrorType, Lossless, RawJson, Tenant},
    query::SelectorType,
};
use pretty_assertions::assert_eq;
use std::fs;
use url::Url;

#[test]
fn a_201_with_location_is_created() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let outcome = create
        .decode(
            &context(),
            response(201, &[(&header::LOCATION, "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567")], ""),
        )
        .unwrap();
    let CreateOutcome::Created {
        location: OptionalHeader::Valid(location),
    } = outcome
    else {
        panic!("expected Created with a location, got {outcome:?}")
    };
    assert_eq!(location.as_str(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
}

#[test]
fn a_207_with_location_and_result_is_partially_created() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let body = r#"{"success": ["urn:ngsi-ld:Vehicle:A4567"], "errors": [{"entityId": "urn:ngsi-ld:Vehicle:A4567", "error": {"type": "https://uri.etsi.org/ngsi-ld/errors/InternalError"}, "registrationId": "urn:ngsi-ld:ContextSourceRegistration:r1"}]}"#;
    let outcome = create
        .decode(
            &context(),
            response(207, &[(&header::LOCATION, "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567")], body),
        )
        .unwrap();
    let CreateOutcome::PartiallyCreated {
        location: OptionalHeader::Valid(_),
        result,
    } = outcome
    else {
        panic!("expected PartiallyCreated, got {outcome:?}")
    };
    assert_eq!(result.success.len(), 1);
    assert_eq!(result.errors[0].error.error_type, Some(ErrorType::InternalError));
}

#[test]
fn a_409_already_exists_decodes_to_its_error_type() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let body = r#"{"type": "https://uri.etsi.org/ngsi-ld/errors/AlreadyExists", "title": "Already exists", "detail": "urn:ngsi-ld:Vehicle:A4567"}"#;
    let error = create
        .decode(&context(), response(409, &[(&header::CONTENT_TYPE, "application/json")], body))
        .unwrap_err();
    assert!(matches!(error, ResponseError::Problem { status: StatusCode::CONFLICT, details, .. } if details.error_type == Some(ErrorType::AlreadyExists)));
}

#[test]
fn a_404_keeps_the_problem_instance() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let body = r#"{"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Resource not found.", "detail": "urn:ngsi-ld:Device:widget001 was not found", "status": 404, "instance": "urn:ngsi-ld:Device:widget001"}"#;
    let error = retrieve.decode(&context(), response(404, &[], body)).unwrap_err();
    let ResponseError::Problem { details, .. } = error else {
        panic!("expected a problem, got {error:?}")
    };
    assert_eq!(details.error_type, Some(ErrorType::ResourceNotFound));
    assert_eq!(
        details.instance.map(|instance| instance.as_str().to_owned()),
        Some("urn:ngsi-ld:Device:widget001".to_owned())
    );
}

#[test]
fn an_unknown_problem_type_is_other_with_its_url() {
    let id = vehicle_id();
    let delete = DeleteEntity::builder().id(&id).build();
    let error = delete
        .decode(
            &context(),
            response(400, &[], r#"{"type": "https://broker.example/errors/Unusual", "title": "Unusual"}"#),
        )
        .unwrap_err();
    let ResponseError::Problem { details, .. } = error else {
        panic!("expected a problem, got {error:?}")
    };
    let Some(ErrorType::Other(url)) = details.error_type else {
        panic!("expected an unknown type")
    };
    assert_eq!(url.as_str(), "https://broker.example/errors/Unusual");
}

#[test]
fn a_bodiless_415_is_a_bare_status() {
    let id = vehicle_id();
    let error = DeleteEntity::builder().id(&id).build().decode(&context(), response(415, &[], "")).unwrap_err();
    assert!(matches!(
        error,
        ResponseError::Status {
            status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ..
        }
    ));
}

#[test]
fn a_page_has_its_items_and_links_resolved_against_the_request_url() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = QueryEntities {
        limit: Some(2),
        ..query_vehicles(Some(&entity_type), None, Simplified)
    };
    let request = query.to_request(&endpoint()).unwrap();
    let body = fs::read_to_string(format!("{}/tests/golden/etsi/EntityList_C.5.4.3.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let link = link_field([
        LinkValue::builder(IriRefBuf::new("/ngsi-ld/v1/entities/?type=Vehicle&format=simplified&limit=2&offset=2").unwrap())
            .rel(RelationType::Next)
            .media_type(APPLICATION_LD_JSON)
            .build(),
        link_value("?type=Vehicle&format=simplified&limit=2&offset=0", RelationType::Prev),
    ]);
    let page = query
        .decode(&ResponseContext::new(&request), response(200, &[(&header::LINK, &link)], &body))
        .unwrap();
    assert_eq!(
        page.items().iter().map(|vehicle| vehicle.id.as_str()).collect::<Vec<_>>(),
        ["urn:ngsi-ld:Vehicle:B9211", "urn:ngsi-ld:Vehicle:A456"]
    );
    assert_eq!(
        page.next().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/entities/?type=Vehicle&format=simplified&limit=2&offset=2")
    );
    // A relative reference resolves against the request URI (RFC 8288 section 3.2), not the API root.
    assert_eq!(
        page.prev().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&format=simplified&limit=2&offset=0")
    );
    assert_eq!(page.unread_links(), [] as [HeaderValue; 0]);
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(
        next.uri().to_string(),
        "http://broker.test/ngsi-ld/v1/entities/?type=Vehicle&format=simplified&limit=2&offset=2"
    );
    assert_eq!(link_of(&next), Some(context_link()));
}

#[test]
fn a_page_link_carrying_an_opaque_continuation_is_requested_exactly_as_the_broker_wrote_it() {
    // Clause 5.5.9.1 lets the pointers to the neighbouring pages be completely opaque, so a link may carry a parameter
    // the specification does not define.
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let request = query.to_request(&endpoint()).unwrap();
    let link = link_field([
        link_value(
            "/ngsi-ld/v1/entities?type=Vehicle&format=simplified&cursor=Zm9v.YmFy&limit=2",
            RelationType::Next,
        ),
        link_value("/ngsi-ld/v1/entities?type=Vehicle&format=simplified&cursor=YmFy&limit=2", RelationType::Prev),
    ]);
    let page = query
        .decode(&ResponseContext::new(&request), response(200, &[(&header::LINK, &link)], "[]"))
        .unwrap();
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(
        next.uri().to_string(),
        "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&format=simplified&cursor=Zm9v.YmFy&limit=2"
    );
    let prev = page.prev_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(
        prev.uri().to_string(),
        "http://broker.test/ngsi-ld/v1/entities?type=Vehicle&format=simplified&cursor=YmFy&limit=2"
    );
}

#[test]
fn the_results_count_header_is_the_total() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Lossless::Normalized);
    let page = query.decode(&context(), response(200, &[(NgsiLdResultsCount::name(), "42")], "[]")).unwrap();
    assert_eq!(page.total(), Some(42));
    assert_eq!(page.items(), []);
}

#[test]
fn a_malformed_results_count_fails_the_page_as_a_header_error_with_its_status() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Lossless::Normalized);
    let error = query
        .decode(&context(), response(200, &[(NgsiLdResultsCount::name(), "many")], "[]"))
        .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Header { status: StatusCode::OK, name, .. } if name == NgsiLdResultsCount::name()),
        "{error:?}"
    );
    assert_eq!(error.head().headers()[NgsiLdResultsCount::name()], "many");
}

#[test]
fn a_body_that_is_not_json_is_a_body_error_with_its_status() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let error = query_vehicles(Some(&entity_type), None, Lossless::Normalized)
        .decode(&context(), response(200, &[], "not json"))
        .unwrap_err();
    assert!(matches!(error, ResponseError::Body { status: StatusCode::OK, source, .. } if matches!(*source, BodyError::Json(_))));
}

#[test]
fn an_invalid_entity_of_a_page_is_reported_at_its_index() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let body = r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle"},{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"value":80}}]"#;
    let error = query_vehicles(Some(&entity_type), None, Lossless::Normalized)
        .decode(&context(), response(200, &[], body))
        .unwrap_err();
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Document(error) = *source else {
        panic!("expected a document error, got {source:?}")
    };
    assert_eq!(error.path.to_string(), "1.speed");
}

#[test]
fn a_204_merge_is_done() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 0).build().unwrap();
    let merge = MergeEntity::builder().id(&id).fragment(&fragment).build();
    assert_eq!(merge.decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
}

#[test]
fn a_203_page_exposes_the_applied_preference() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let page = query
        .decode(&context(), response(203, &[(PreferenceApplied::name(), "ngsi-ld=1.8.1")], "[]"))
        .unwrap();
    assert_eq!(page.preference_applied().map(ToString::to_string), Some("1.8.1".to_owned()));
}

#[test]
fn a_201_with_a_malformed_location_is_still_created_and_keeps_the_raw_header() {
    let vehicle = vehicle();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let outcome = create
        .decode(&context(), response(201, &[(&header::LOCATION, "/ngsi-ld/v1/entities/a b")], ""))
        .unwrap();
    assert_eq!(
        outcome,
        CreateOutcome::Created {
            location: OptionalHeader::Malformed(vec![HeaderValue::from_static("/ngsi-ld/v1/entities/a b")])
        }
    );
    let absent = create.decode(&context(), response(201, &[], "")).unwrap();
    assert_eq!(
        absent,
        CreateOutcome::Created {
            location: OptionalHeader::Absent
        }
    );
}

#[test]
fn a_malformed_link_value_is_kept_raw_and_the_readable_links_are_used() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let line = r#"garbage, </ngsi-ld/v1/entities?type=Vehicle&offset=2>; rel="next""#;
    let page = query.decode(&context(), response(200, &[(&header::LINK, line)], "[]")).unwrap();
    assert_eq!(
        page.next().map(Url::as_str),
        Some("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&offset=2")
    );
    assert_eq!(page.unread_links(), [HeaderValue::from_static(line)]);
}

#[test]
fn an_unreadable_preference_applied_does_not_fail_the_page() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let page = query
        .decode(&context(), response(203, &[(PreferenceApplied::name(), "ngsi-ld=latest")], "[]"))
        .unwrap();
    assert_eq!(page.preference_applied(), None);
    assert_eq!(page.items(), []);
}

#[test]
fn a_page_remembers_the_tenant_its_request_addressed_and_its_next_page_uses_it() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let tenant: Tenant = "t1".parse().unwrap();
    let request = query.to_request(&endpoint().with_tenant(tenant.clone())).unwrap();
    let link = link_field([link_value("/ngsi-ld/v1/entities?type=Vehicle&offset=2", RelationType::Next)]);
    let page = query
        .decode(&ResponseContext::new(&request), response(200, &[(&header::LINK, &link)], "[]"))
        .unwrap();
    assert_eq!(page.tenant(), Some(&tenant));
    let other: Tenant = "t2".parse().unwrap();
    for endpoint in [endpoint(), endpoint().with_tenant(other)] {
        let next = page.next_page().unwrap().to_request(&endpoint).unwrap();
        assert_eq!(header_value(&next, NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn a_problem_out_of_shape_still_decodes_with_its_raw_body() {
    let id = vehicle_id();
    let body = r#"{"type": "errors/TenantNotFound", "title": "Tenant not found", "status": "404", "instance": "not an iri"}"#;
    let error = DeleteEntity::builder()
        .id(&id)
        .build()
        .decode(&context(), response(404, &[], body))
        .unwrap_err();
    let ResponseError::Problem { details, body: raw, .. } = error else {
        panic!("expected a problem, got {error:?}")
    };
    assert_eq!(
        details.error_type.map(|error_type| error_type.to_string()),
        Some("errors/TenantNotFound".to_owned())
    );
    assert_eq!(details.status, Some(404));
    assert_eq!(details.instance, None);
    assert_eq!(*raw, body.parse::<RawJson>().unwrap());
}
