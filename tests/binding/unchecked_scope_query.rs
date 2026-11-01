//! A scope query sent as written through the unchecked path, as a negative test of the Scope Query Language (clause
//! 4.19) needs: the syntactically invalid queries a broker must answer with `BadRequestData`.

use crate::fixtures::endpoint;
use ngsi_ld::{
    http::{
        UncheckedRequest,
        operation::{Operation, QueryEntities},
    },
    model::Lossless,
    query::{Locality, SelectorType, scope_query::ScopeQuery},
};
use pretty_assertions::assert_eq;
use url::form_urlencoded;

/// The entity type the queries select.
fn buildings() -> SelectorType {
    "Building".parse().unwrap()
}

/// Queries `entity_type` in normalized representation, with no other parameter.
fn query(entity_type: &SelectorType) -> QueryEntities<'_, Lossless> {
    QueryEntities::builder().entity_type(entity_type).format(Lossless::Normalized).build()
}

/// The decoded value of the `scopeQ` parameter of `request`.
fn sent_scope_query(request: &UncheckedRequest) -> Option<String> {
    let query = request.as_request().uri().query()?;
    form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == "scopeQ")
        .map(|(_, value)| value.into_owned())
}

#[test]
fn each_invalid_scope_query_is_sent_escaped_as_a_parsed_one_and_decodes_to_the_text_as_written() {
    let buildings = buildings();
    let cases = [
        ("(/Madrid", "type=Building&scopeQ=(/Madrid"),
        ("/Madrid;", "type=Building&scopeQ=/Madrid;"),
        ("/Madrid,,/CompanyA", "type=Building&scopeQ=/Madrid%2C%2C/CompanyA"),
        ("/Madrid/#/Gardens", "type=Building&scopeQ=/Madrid/%23/Gardens"),
    ];
    for (text, expected) in cases {
        assert!(text.parse::<ScopeQuery>().is_err(), "{text} is a valid scope query");
        let request = UncheckedRequest::try_from_operation(&query(&buildings), &endpoint())
            .unwrap()
            .scope_query(text)
            .unwrap();
        assert_eq!(request.as_request().uri().query(), Some(expected));
        assert_eq!(sent_scope_query(&request).as_deref(), Some(text));
    }
}

#[test]
fn a_valid_scope_query_is_escaped_the_same_through_either_path() {
    let buildings = buildings();
    let parsed: ScopeQuery = "(/Madrid/+/Gardens;/CompanyA),/CompanyB/#".parse().unwrap();
    let typed = QueryEntities {
        scope_q: Some(&parsed),
        ..query(&buildings)
    };
    let unchecked = UncheckedRequest::try_from_operation(&query(&buildings), &endpoint())
        .unwrap()
        .scope_query(&parsed.to_string())
        .unwrap();
    assert_eq!(unchecked.as_request().uri(), typed.to_request(&endpoint()).unwrap().uri());
}

#[test]
fn a_scope_query_the_operation_sent_is_replaced_in_place() {
    let buildings = buildings();
    let madrid: ScopeQuery = "/Madrid".parse().unwrap();
    let typed = QueryEntities {
        scope_q: Some(&madrid),
        limit: Some(5),
        ..query(&buildings)
    };
    let request = UncheckedRequest::try_from_operation(&typed, &endpoint())
        .unwrap()
        .scope_query("(/Madrid")
        .unwrap();
    assert_eq!(request.as_request().uri().query(), Some("type=Building&scopeQ=(/Madrid&limit=5"));
}

#[test]
fn a_scope_query_is_added_last_after_the_local_flag() {
    let buildings = buildings();
    let local = QueryEntities {
        locality: Locality::Local,
        ..query(&buildings)
    };
    let request = UncheckedRequest::try_from_operation(&local, &endpoint())
        .unwrap()
        .scope_query("/Madrid;")
        .unwrap();
    assert_eq!(request.as_request().uri().query(), Some("type=Building&local=true&scopeQ=/Madrid;"));
}
