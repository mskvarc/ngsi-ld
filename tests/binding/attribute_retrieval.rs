//! Retrieve Attribute (ETSI TS 104 175 clause 10.4.4 and ETSI TS 104 176 clause 7.5.3.1): the request it builds, and
//! how each answer of Table 7.5.3.1-3 is decoded.

use crate::{
    fixtures::{context, endpoint, header_value, link_of, response, vehicle_id},
    links::context_link,
};
use headers::Header;
use http::{Method, StatusCode, header};
use iri_rs::IriRefBuf;
use ngsi_ld::{
    http::{
        BodyError,
        ResponseError,
        headers::{NgsiLdEntityMap, NgsiLdSnapshot, OptionalHeader, PreferenceApplied},
        operation::{EntityMapRequest, Head, InlineJoin, OnSnapshot, Operation, RetrieveAttribute, SystemAttributes, WithLocality},
    },
    model::{AttributeName, Context, DatasetId, EntityId, ErrorType, Lossless},
    query::{Locality, language_filter::LanguageFilter, type_selection::TypeSelection},
    snapshot::SnapshotId,
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU32;

/// The attribute the tests retrieve.
fn speed() -> AttributeName {
    "speed".parse().unwrap()
}

/// The location of the entity map the broker hands out.
const MAP: &str = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1";

/// The entity map the broker hands out.
fn entity_map() -> NgsiLdEntityMap {
    NgsiLdEntityMap::try_new(IriRefBuf::new(MAP).unwrap()).unwrap()
}

/// `speed`, with system attributes, as a broker returns it in normalized representation.
const SPEED: &str = r#"{"type":"Property","value":80,"unitCode":"KMH","createdAt":"2024-05-01T10:00:00Z","modifiedAt":"2024-05-01T10:05:00Z"}"#;

/// Retrieves `name` of `id`, with no other parameter.
fn retrieve<'a>(id: &'a EntityId, name: &'a AttributeName) -> RetrieveAttribute<'a> {
    RetrieveAttribute::builder().id(id).name(name).build()
}

#[test]
fn the_request_is_a_get_of_the_attribute_asking_for_json_with_the_context_link() {
    let id = vehicle_id();
    let name = speed();
    let request = retrieve(&id, &name).to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed");
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
}

#[test]
fn every_parameter_of_table_7_5_3_1_1_is_sent_with_its_encoding() {
    let id = vehicle_id();
    let name = speed();
    let vehicles: TypeSelection = "Vehicle".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let lang: LanguageFilter = "fr-CH,fr;q=0.9".parse().unwrap();
    let contained_by: [EntityId; 1] = ["urn:ngsi-ld:Building:1".parse().unwrap()];
    let map = EntityMapRequest::returned_for("PT1H".parse().unwrap());
    let retrieve = RetrieveAttribute {
        entity_type: Some(&vehicles),
        dataset_id: Some(&gps),
        lang: Some(&lang),
        join: Some(InlineJoin::Inline),
        join_level: NonZeroU32::new(2),
        contained_by: &contained_by,
        entity_map: Some(&map),
        system_attributes: SystemAttributes::Include,
        ..retrieve(&id, &name)
    };
    let request = retrieve.to_request(&endpoint()).unwrap();
    assert_eq!(
        request.uri().query(),
        Some(concat!(
            "type=Vehicle&datasetId=urn%3Angsi-ld%3Adataset%3Agps&lang=fr-CH,fr;q=0.9&join=inline&joinLevel=2&",
            "containedBy=urn%3Angsi-ld%3ABuilding%3A1&entityMap=true&entityMapLifetime=PT1H&options=sysAttrs"
        ))
    );
    assert_eq!(header_value(&request, NgsiLdEntityMap::name()), None);
}

#[test]
fn the_default_instance_is_asked_for_with_none_and_a_join_of_none_is_sent() {
    let id = vehicle_id();
    let name = speed();
    let default: DatasetId = "@none".parse().unwrap();
    let retrieve = RetrieveAttribute {
        dataset_id: Some(&default),
        join: Some(InlineJoin::None),
        ..retrieve(&id, &name)
    };
    assert_eq!(retrieve.to_request(&endpoint()).unwrap().uri().query(), Some("datasetId=%40none&join=%40none"));
}

#[test]
fn reusing_an_entity_map_sends_its_location_in_the_header_only() {
    let id = vehicle_id();
    let name = speed();
    let map = EntityMapRequest::reuse(entity_map());
    let retrieve = RetrieveAttribute {
        entity_map: Some(&map),
        ..retrieve(&id, &name)
    };
    let request = retrieve.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, NgsiLdEntityMap::name()), Some(MAP));
}

#[test]
fn local_scope_a_snapshot_and_head_wrap_the_retrieval() {
    let id = vehicle_id();
    let name = speed();
    let retrieve = retrieve(&id, &name);
    let local = WithLocality::new(Locality::Local, &retrieve).to_request(&endpoint()).unwrap();
    assert_eq!(local.uri().query(), Some("local=true"));
    let snapshot: SnapshotId = "urn:ngsi-ld:Snapshot:1".parse().unwrap();
    let on_snapshot = OnSnapshot::new(&snapshot, &retrieve).to_request(&endpoint()).unwrap();
    assert_eq!(on_snapshot.headers()[NgsiLdSnapshot::name()], "urn:ngsi-ld:Snapshot:1");
    let head = Head::new(&retrieve).to_request(&endpoint()).unwrap();
    assert_eq!(head.method(), Method::HEAD);
    assert_eq!(head.uri(), "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed");
}

#[test]
fn a_200_answer_is_the_attribute_with_its_output_members_and_the_entity_map() {
    let id = vehicle_id();
    let name = speed();
    let retrieved = retrieve(&id, &name)
        .decode(&context(), response(200, &[(NgsiLdEntityMap::name(), MAP)], SPEED))
        .unwrap();
    assert_eq!(retrieved.status(), StatusCode::OK);
    assert_eq!(retrieved.body().context, None);
    assert_eq!(retrieved.body().attribute.render(Lossless::Normalized).to_string(), SPEED);
    assert_eq!(retrieved.entity_map(), &OptionalHeader::Valid(entity_map()));
}

#[test]
fn an_answer_in_json_ld_keeps_its_context_apart_from_the_attribute() {
    let id = vehicle_id();
    let name = speed();
    let body = r#"{"type":"Property","value":80,"@context":"https://example.org/context.jsonld"}"#;
    let retrieved = retrieve(&id, &name).decode(&context(), response(200, &[], body)).unwrap();
    assert_eq!(
        retrieved.body().context.as_ref().and_then(Context::link_target).map(IriRefBuf::as_str),
        Some("https://example.org/context.jsonld")
    );
    assert_eq!(
        retrieved.body().attribute.render(Lossless::Normalized).to_string(),
        r#"{"type":"Property","value":80}"#
    );
}

#[test]
fn a_203_answer_exposes_the_applied_preference() {
    let id = vehicle_id();
    let name = speed();
    let retrieved = retrieve(&id, &name)
        .decode(&context(), response(203, &[(PreferenceApplied::name(), "ngsi-ld=1.8.1")], SPEED))
        .unwrap();
    assert_eq!(retrieved.status(), StatusCode::NON_AUTHORITATIVE_INFORMATION);
    assert_eq!(retrieved.preference_applied().map(ToString::to_string), Some("1.8.1".to_owned()));
}

#[test]
fn a_404_answer_is_a_resource_not_found_problem() {
    let id = vehicle_id();
    let name = speed();
    let body = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"Attribute not found"}"#;
    let error = retrieve(&id, &name).decode(&context(), response(404, &[], body)).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Problem { status: StatusCode::NOT_FOUND, details, .. } if details.error_type == Some(ErrorType::ResourceNotFound)),
        "{error:?}"
    );
}

#[test]
fn an_answer_of_several_instances_is_not_one_attribute() {
    let id = vehicle_id();
    let name = speed();
    let body = format!("[{SPEED},{SPEED}]");
    let error = retrieve(&id, &name).decode(&context(), response(200, &[], &body)).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::Document(_))),
        "{error:?}"
    );
}

#[test]
fn a_success_status_the_table_does_not_list_is_unexpected() {
    let id = vehicle_id();
    let name = speed();
    let error = retrieve(&id, &name).decode(&context(), response(204, &[], "")).unwrap_err();
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
