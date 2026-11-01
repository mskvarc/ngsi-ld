//! Set Attribute Value (ETSI TS 104 175 clause 10.2.6 and ETSI TS 104 176 clause 7.6.3.2): the request it builds, the
//! values it refuses, and how each answer of Table 7.6.3.2-2 is decoded.

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, json, link_of, response, vehicle_id},
    links::context_link,
};
use headers::Header;
use http::{Method, StatusCode, header};
use indexmap::IndexMap;
use ngsi_ld::{
    http::{
        BatchOutcome,
        ContextDelivery,
        RequestError,
        ResponseError,
        headers::NgsiLdSnapshot,
        operation::{ArgumentError, OnSnapshot, Operation, SetAttributeValue, ValueOnly, WithLocality},
    },
    model::{AttributeName, DatasetId, DateTime, EntityId, ErrorType, RawJson, Value},
    query::{Locality, type_selection::TypeSelection},
    snapshot::SnapshotId,
};
use pretty_assertions::assert_eq;

/// The attribute whose value the tests set.
fn speed() -> AttributeName {
    "speed".parse().unwrap()
}

/// Sets `name` of `id` to `value`, with no other parameter.
fn set<'a>(id: &'a EntityId, name: &'a AttributeName, value: impl Into<ValueOnly<'a>>) -> SetAttributeValue<'a> {
    SetAttributeValue::builder().id(id).name(name).value(value).build()
}

#[test]
fn the_request_is_a_put_of_the_value_resource_with_the_bare_value_as_json_and_the_context_link() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let request = set(&id, &name, &value).to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PUT);
    assert_eq!(
        request.uri(),
        "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value"
    );
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(request.body().as_ref(), b"30");
}

#[test]
fn every_parameter_of_table_7_6_3_2_1_is_sent_with_its_encoding() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let vehicles: TypeSelection = "Vehicle".parse().unwrap();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let observed: DateTime = "2024-05-01T10:00:00Z".parse().unwrap();
    let set = SetAttributeValue {
        entity_type: Some(&vehicles),
        dataset_id: Some(&gps),
        observed_at: Some(&observed),
        ..set(&id, &name, &value)
    };
    assert_eq!(
        set.to_request(&endpoint()).unwrap().uri().query(),
        Some("type=Vehicle&datasetId=urn%3Angsi-ld%3Adataset%3Agps&observedAt=2024-05-01T10%3A00%3A00Z")
    );
}

#[test]
fn the_value_of_the_default_instance_is_set_with_none() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let default: DatasetId = "@none".parse().unwrap();
    let set = SetAttributeValue {
        dataset_id: Some(&default),
        ..set(&id, &name, &value)
    };
    assert_eq!(set.to_request(&endpoint()).unwrap().uri().query(), Some("datasetId=%40none"));
}

#[test]
fn a_text_an_object_or_an_array_value_is_sent_as_it_is() {
    let id = vehicle_id();
    let name = speed();
    let target = Value::from("urn:ngsi-ld:Person:1");
    let map = Value::Object(IndexMap::from([
        ("fr".to_owned(), Value::from("Grand Place")),
        ("nl".to_owned(), Value::from("Grote Markt")),
    ]));
    let list = Value::from(vec![300, 300, 120]);
    let bodies = [&target, &map, &list].map(|value| set(&id, &name, value).to_request(&endpoint()).unwrap().into_body());
    assert_eq!(
        bodies.map(|body| json(&body)),
        [
            json(br#""urn:ngsi-ld:Person:1""#),
            json(br#"{"fr":"Grand Place","nl":"Grote Markt"}"#),
            json(b"[300,300,120]")
        ]
    );
}

#[test]
fn the_json_of_a_json_property_is_sent_verbatim_with_its_nulls() {
    let id = vehicle_id();
    let name = speed();
    let ticket: RawJson = r#"{"id":"85a6cc52","paid":null}"#.parse().unwrap();
    let request = set(&id, &name, &ticket).to_request(&endpoint()).unwrap();
    assert_eq!(request.body().as_ref(), br#"{"id":"85a6cc52","paid":null}"#);
}

#[test]
fn an_endpoint_delivering_the_context_in_the_body_still_sends_the_value_as_json_with_the_link() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let request = set(&id, &name, &value).to_request(&endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(request.body().as_ref(), b"30");
}

#[test]
fn a_value_holding_the_ngsi_ld_null_anywhere_is_refused() {
    let id = vehicle_id();
    let name = speed();
    let alone = Value::from("urn:ngsi-ld:null");
    let in_array = Value::from(vec!["urn:ngsi-ld:Person:1", "urn:ngsi-ld:null"]);
    let in_object = Value::Object(IndexMap::from([("street".to_owned(), Value::from("urn:ngsi-ld:null"))]));
    let in_json: RawJson = r#"[{"paid":{"by":"urn:ngsi-ld:null"}}]"#.parse().unwrap();
    let values = [
        ValueOnly::from(&alone),
        ValueOnly::from(&in_array),
        ValueOnly::from(&in_object),
        ValueOnly::from(&in_json),
    ];
    for value in values {
        let refusal = set(&id, &name, value).to_request(&endpoint());
        assert!(matches!(refusal, Err(RequestError::Argument(ArgumentError::NullInValue))), "{refusal:?}");
    }
}

#[test]
fn local_scope_and_a_snapshot_wrap_the_request() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let set = set(&id, &name, &value);
    let local = WithLocality::new(Locality::Local, &set).to_request(&endpoint()).unwrap();
    assert_eq!(local.uri().query(), Some("local=true"));
    let snapshot: SnapshotId = "urn:ngsi-ld:Snapshot:1".parse().unwrap();
    let on_snapshot = OnSnapshot::new(&snapshot, &set).to_request(&endpoint()).unwrap();
    assert_eq!(on_snapshot.headers()[NgsiLdSnapshot::name()], "urn:ngsi-ld:Snapshot:1");
}

#[test]
fn a_204_answer_is_done() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    assert_eq!(set(&id, &name, &value).decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
}

#[test]
fn a_207_answer_is_the_result_of_each_registration_it_was_forwarded_to() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let body = r#"{"success":[],"errors":[{"entityId":"urn:ngsi-ld:Vehicle:A4567","error":{"type":"https://uri.etsi.org/ngsi-ld/errors/Conflict"},"registrationId":"urn:ngsi-ld:ContextSourceRegistration:r1"}]}"#;
    let outcome = set(&id, &name, &value).decode(&context(), response(207, &[], body)).unwrap();
    let BatchOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.errors[0].entity_id, id);
    assert_eq!(result.errors[0].error.error_type, Some(ErrorType::Conflict));
    assert_eq!(
        result.errors[0].registration_id.as_ref().map(ToString::to_string).as_deref(),
        Some("urn:ngsi-ld:ContextSourceRegistration:r1")
    );
}

#[test]
fn a_404_answer_is_a_resource_not_found_problem() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let body = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"Attribute not found"}"#;
    let error = set(&id, &name, &value).decode(&context(), response(404, &[], body)).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Problem { status: StatusCode::NOT_FOUND, details, .. } if details.error_type == Some(ErrorType::ResourceNotFound)),
        "{error:?}"
    );
}

#[test]
fn a_success_status_the_table_does_not_list_is_unexpected() {
    let id = vehicle_id();
    let name = speed();
    let value = Value::from(30);
    let error = set(&id, &name, &value).decode(&context(), response(200, &[], "30")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { status: StatusCode::OK, .. }), "{error:?}");
}
