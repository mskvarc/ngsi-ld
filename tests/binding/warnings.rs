//! The `NGSILD-Warning` header of clause 6.3.17 on the answers of Retrieve Entity and Query Entities.

use crate::{
    fixtures::{context, response, vehicle, vehicle_body, vehicle_id},
    query_vehicles,
    retrieve_entity,
};
use headers::Header;
use http::HeaderValue;
use jstrict::Print;
use ngsi_ld::{
    http::{
        headers::{NgsiLdWarning, NgsiLdWarnings, OptionalHeader, WarningCode},
        operation::Operation,
    },
    model::Lossless,
    query::SelectorType,
};
use pretty_assertions::assert_eq;

#[test]
fn a_page_keeps_every_warning_of_a_distributed_query() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Lossless::Normalized);
    let headers = [
        (NgsiLdWarnings::name(), "110 - Response is Stale"),
        (NgsiLdWarnings::name(), "199 - Miscellaneous Warning"),
    ];
    let page = query.decode(&context(), response(200, &headers, "[]")).unwrap();
    assert_eq!(
        page.warnings(),
        &OptionalHeader::Valid(
            NgsiLdWarnings::new(NgsiLdWarning::new(WarningCode::ResponseIsStale)).and(NgsiLdWarning::new(WarningCode::MiscellaneousWarning))
        )
    );
}

#[test]
fn an_unreadable_warning_is_kept_raw_and_the_entity_is_still_read() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized);
    let body = vehicle_body().compact_print().to_string();
    let warned = retrieve
        .decode(&context(), response(200, &[(NgsiLdWarnings::name(), "112 - Disconnected Operation")], &body))
        .unwrap();
    assert_eq!(
        warned.warnings(),
        &OptionalHeader::Malformed(vec![HeaderValue::from_static("112 - Disconnected Operation")])
    );
    assert_eq!(warned.into_body().id, vehicle().id);
}

#[test]
fn an_answer_without_warnings_has_none() {
    let id = vehicle_id();
    let body = vehicle_body().compact_print().to_string();
    let retrieved = retrieve_entity(&id, Lossless::Normalized)
        .decode(&context(), response(200, &[], &body))
        .unwrap();
    assert!(retrieved.warnings().is_absent());
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let page = query_vehicles(Some(&entity_type), None, Lossless::Normalized)
        .decode(&context(), response(200, &[], "[]"))
        .unwrap();
    assert!(page.warnings().is_absent());
}
