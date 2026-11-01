//! Retrieve Attribute Value (ETSI TS 104 175 clause 10.4.4 and ETSI TS 104 176 clause 7.6.3.1): the request it builds,
//! and how each answer of Table 7.6.3.1-2 is decoded.

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
        operation::{EntityMapRequest, Head, OnSnapshot, Operation, RetrieveAttributeValue, WithLocality},
    },
    model::{AttributeName, DatasetId, EntityId, ErrorType},
    query::{Locality, language_filter::LanguageFilter, type_selection::TypeSelection},
    snapshot::SnapshotId,
};
use pretty_assertions::assert_eq;

/// The attribute whose value the tests retrieve.
fn speed() -> AttributeName {
    "speed".parse().unwrap()
}

/// The location of the entity map the broker hands out.
const MAP: &str = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1";

/// The entity map the broker hands out.
fn entity_map() -> NgsiLdEntityMap {
    NgsiLdEntityMap::try_new(IriRefBuf::new(MAP).unwrap()).unwrap()
}

/// Retrieves the value of `name` of `id`, with no other parameter.
fn retrieve<'a>(id: &'a EntityId, name: &'a AttributeName) -> RetrieveAttributeValue<'a> {
    RetrieveAttributeValue::builder().id(id).name(name).build()
}

#[test]
fn the_request_is_a_get_of_the_value_resource_asking_for_json_with_the_context_link() {
    let id = vehicle_id();
    let name = speed();
    let request = retrieve(&id, &name).to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(
        request.uri(),
        "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value"
    );
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
}

#[test]
fn every_parameter_of_table_7_6_3_1_1_is_sent_with_its_encoding() {
    let id = vehicle_id();
    let name = speed();
    let vehicles: TypeSelection = "Vehicle".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let lang: LanguageFilter = "fr-CH,fr;q=0.9".parse().unwrap();
    let map = EntityMapRequest::returned_for("PT1H".parse().unwrap());
    let retrieve = RetrieveAttributeValue {
        entity_type: Some(&vehicles),
        dataset_id: Some(&gps),
        lang: Some(&lang),
        entity_map: Some(&map),
        ..retrieve(&id, &name)
    };
    let request = retrieve.to_request(&endpoint()).unwrap();
    assert_eq!(
        request.uri().query(),
        Some("type=Vehicle&datasetId=urn%3Angsi-ld%3Adataset%3Agps&lang=fr-CH,fr;q=0.9&entityMap=true&entityMapLifetime=PT1H")
    );
    assert_eq!(header_value(&request, NgsiLdEntityMap::name()), None);
}

#[test]
fn the_value_of_the_default_instance_is_asked_for_with_none() {
    let id = vehicle_id();
    let name = speed();
    let default: DatasetId = "@none".parse().unwrap();
    let retrieve = RetrieveAttributeValue {
        dataset_id: Some(&default),
        ..retrieve(&id, &name)
    };
    assert_eq!(retrieve.to_request(&endpoint()).unwrap().uri().query(), Some("datasetId=%40none"));
}

#[test]
fn reusing_an_entity_map_sends_its_location_in_the_header_only() {
    let id = vehicle_id();
    let name = speed();
    let map = EntityMapRequest::reuse(entity_map());
    let retrieve = RetrieveAttributeValue {
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
    assert_eq!(head.uri(), "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value");
}

#[test]
fn a_200_answer_of_a_json_primitive_is_the_value_with_the_entity_map() {
    let id = vehicle_id();
    let name = speed();
    let retrieved = retrieve(&id, &name)
        .decode(&context(), response(200, &[(NgsiLdEntityMap::name(), MAP)], "80.50"))
        .unwrap();
    assert_eq!(retrieved.status(), StatusCode::OK);
    assert_eq!(retrieved.body().to_string(), "80.50");
    assert_eq!(retrieved.entity_map(), &OptionalHeader::Valid(entity_map()));
}

#[test]
fn an_object_or_an_array_value_is_kept_as_the_broker_wrote_it() {
    let id = vehicle_id();
    let name = speed();
    let values = [
        r#"{"fr":"Grand Place","nl":"Grote Markt"}"#,
        r#"[{"ticket":"85a6cc52","paid":null}]"#,
        r#"["urn:ngsi-ld:Person:1","urn:ngsi-ld:Person:2"]"#,
    ];
    let read = values.map(|body| {
        retrieve(&id, &name)
            .decode(&context(), response(200, &[], body))
            .unwrap()
            .into_body()
            .to_string()
    });
    assert_eq!(read, values);
}

#[test]
fn a_203_answer_exposes_the_applied_preference() {
    let id = vehicle_id();
    let name = speed();
    let retrieved = retrieve(&id, &name)
        .decode(&context(), response(203, &[(PreferenceApplied::name(), "ngsi-ld=1.8.1")], r#""Berlin""#))
        .unwrap();
    assert_eq!(retrieved.status(), StatusCode::NON_AUTHORITATIVE_INFORMATION);
    assert_eq!(retrieved.body().to_string(), r#""Berlin""#);
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
fn a_body_that_is_not_json_is_a_body_error() {
    let id = vehicle_id();
    let name = speed();
    let error = retrieve(&id, &name).decode(&context(), response(200, &[], "Berlin")).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::Json(_))),
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
