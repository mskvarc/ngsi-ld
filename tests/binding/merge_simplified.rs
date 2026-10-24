//! Merge Entity with a fragment in simplified representation (clauses 5.6.17 and 6.5.3.4).

use crate::{
    fixtures::{context, endpoint, endpoint_with, header_value, json, link_of, response, vehicle_id},
    links::context_link,
};
use http::{Method, header};
use langtag::LangTagBuf;
use ngsi_ld::{
    http::{
        BatchOutcome,
        ContextDelivery,
        operation::{MergeSimplifiedEntity, Operation},
    },
    model::{DateTime, EntityId, SimplifiedFragment},
    query::type_selection::TypeSelection,
};
use pretty_assertions::assert_eq;

/// A simplified fragment: a value, a language text and a deletion.
fn fragment() -> SimplifiedFragment {
    SimplifiedFragment::builder()
        .value("speed", 55)
        .value("street", "Grote Markt")
        .delete("isParked")
        .build()
        .unwrap()
}

/// The simplified merge of `fragment` into `id`, with no other parameter.
fn merge<'a>(id: &'a EntityId, fragment: &'a SimplifiedFragment) -> MergeSimplifiedEntity<'a> {
    MergeSimplifiedEntity::builder().id(id).fragment(fragment).build()
}

/// The body of `fragment`, without `@context`.
const BODY: &str = r#"{"speed": 55, "street": "Grote Markt", "isParked": "urn:ngsi-ld:null"}"#;

#[test]
fn a_simplified_merge_sends_format_simplified_and_the_bare_values_with_a_link_header() {
    let id = vehicle_id();
    let fragment = fragment();
    let request = merge(&id, &fragment).to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
    assert_eq!(request.uri().query(), Some("format=simplified"));
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(json(request.body()), json(BODY.as_bytes()));
}

#[test]
fn a_simplified_merge_under_ld_json_body_carries_the_context_last() {
    let id = vehicle_id();
    let fragment = fragment();
    let request = merge(&id, &fragment).to_request(&endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    assert_eq!(
        json(request.body()),
        json(br#"{"speed": 55, "street": "Grote Markt", "isParked": "urn:ngsi-ld:null", "@context": "https://example.org/context.jsonld"}"#)
    );
}

#[test]
fn a_simplified_merge_sends_type_observed_at_format_and_lang_in_that_order() {
    let id = vehicle_id();
    let fragment = fragment();
    let entity_type: TypeSelection = "Vehicle".parse().unwrap();
    let observed_at: DateTime = "2024-05-01T10:00:00Z".parse().unwrap();
    let lang: LangTagBuf = "nl".parse().unwrap();
    let request = MergeSimplifiedEntity {
        entity_type: Some(&entity_type),
        observed_at: Some(&observed_at),
        lang: Some(&lang),
        ..merge(&id, &fragment)
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(
        request.uri().query(),
        Some("type=Vehicle&observedAt=2024-05-01T10%3A00%3A00Z&format=simplified&lang=nl")
    );
}

#[test]
fn a_simplified_merge_answers_204_done_and_207_partial() {
    let id = vehicle_id();
    let fragment = fragment();
    let merge = merge(&id, &fragment);
    assert_eq!(merge.decode(&context(), response(204, &[], "")).unwrap(), BatchOutcome::Done);
    let outcome = merge
        .decode(&context(), response(207, &[], r#"{"success": ["urn:ngsi-ld:Vehicle:A4567"], "errors": []}"#))
        .unwrap();
    let BatchOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.success.len(), 1);
}
