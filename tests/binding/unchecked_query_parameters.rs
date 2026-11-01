//! Query parameters through the unchecked path, as the negative tests of the query languages and parameters need:
//! a parameter set to text as written, a parameter removed so the operation's own refusal is bypassed, and a parameter
//! the specification does not define.

use crate::fixtures::endpoint;
use ngsi_ld::{
    http::{
        QueryParameter,
        RequestError,
        UncheckedRequest,
        operation::{Operation, QueryEntities},
    },
    model::{EntityId, Lossless},
    query::{Locality, SelectorType, projection::Projection, scope_query::ScopeQuery},
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

/// The unchecked request of the query of buildings.
fn unchecked(entity_type: &SelectorType) -> UncheckedRequest {
    UncheckedRequest::try_from_operation(&query(entity_type), &endpoint()).unwrap()
}

/// The decoded values of the parameter `name` of `request`, in order.
fn sent(request: &UncheckedRequest, name: &str) -> Vec<String> {
    let query = request.as_request().uri().query().unwrap_or_default();
    form_urlencoded::parse(query.as_bytes())
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .collect()
}

#[test]
fn each_invalid_value_of_the_test_suite_is_escaped_as_a_typed_value_and_decodes_to_the_text_as_written() {
    let buildings = buildings();
    let cases = [
        (QueryParameter::ScopeQ, "(/Madrid", "scopeQ=(/Madrid"),
        (QueryParameter::ScopeQ, "/Madrid;", "scopeQ=/Madrid;"),
        (QueryParameter::ScopeQ, "/Madrid,,/CompanyA", "scopeQ=/Madrid%2C%2C/CompanyA"),
        (QueryParameter::ScopeQ, "/Madrid/#/Gardens", "scopeQ=/Madrid/%23/Gardens"),
        (QueryParameter::Q, "attr==", "q=attr%3D%3D"),
        (QueryParameter::Q, "attr[", "q=attr%5B"),
        (QueryParameter::Q, r#";attr=="value""#, "q=%3Battr%3D%3D%22value%22"),
        (QueryParameter::Pick, "id;name", "pick=id%3Bname"),
        (QueryParameter::Pick, "id,locatedAt{name", "pick=id,locatedAt%7Bname"),
        (QueryParameter::Pick, "id,,name", "pick=id,,name"),
        (QueryParameter::Attrs, "id,type", "attrs=id,type"),
        (
            QueryParameter::Id,
            "thisisaninvaliduri1,thisisaninvaliduri2",
            "id=thisisaninvaliduri1,thisisaninvaliduri2",
        ),
        (
            QueryParameter::IdPattern,
            "invalid_entity_id_pattern**",
            "idPattern=invalid_entity_id_pattern%2A%2A",
        ),
        (QueryParameter::Options, "keyValues,invalid", "options=keyValues,invalid"),
        (QueryParameter::Georel, "within", "georel=within"),
        (QueryParameter::Limit, "-5", "limit=-5"),
        (QueryParameter::Offset, "-3", "offset=-3"),
        (QueryParameter::Details, "other", "details=other"),
    ];
    for (name, text, expected) in cases {
        let request = unchecked(&buildings).query_parameter(name, text).unwrap();
        assert_eq!(request.as_request().uri().query(), Some(format!("type=Building&{expected}").as_str()), "{name}");
        assert_eq!(sent(&request, name.into()), [text], "{name}");
    }
}

#[test]
fn the_invalid_scope_queries_are_refused_by_the_typed_path() {
    for text in ["(/Madrid", "/Madrid;", "/Madrid,,/CompanyA", "/Madrid/#/Gardens"] {
        assert!(text.parse::<ScopeQuery>().is_err(), "{text} is a valid scope query");
    }
}

#[test]
fn a_typed_value_written_as_text_is_escaped_the_same_through_either_path() {
    let buildings = buildings();
    let scope: ScopeQuery = "(/Madrid/+/Gardens;/CompanyA),/CompanyB/#".parse().unwrap();
    let pick: Projection = "id,type,locatedAt{name}".parse().unwrap();
    let typed = QueryEntities {
        scope_q: Some(&scope),
        pick: Some(&pick),
        ..query(&buildings)
    };
    let unchecked = unchecked(&buildings)
        .query_parameter(QueryParameter::ScopeQ, &scope.to_string())
        .unwrap()
        .query_parameter(QueryParameter::Pick, &pick.to_string())
        .unwrap();
    assert_eq!(unchecked.as_request().uri(), typed.to_request(&endpoint()).unwrap().uri());
}

#[test]
fn a_parameter_the_operation_sent_is_replaced_in_place() {
    let buildings = buildings();
    let madrid: ScopeQuery = "/Madrid".parse().unwrap();
    let typed = QueryEntities {
        scope_q: Some(&madrid),
        limit: Some(5),
        ..query(&buildings)
    };
    let request = UncheckedRequest::try_from_operation(&typed, &endpoint())
        .unwrap()
        .query_parameter(QueryParameter::ScopeQ, "(/Madrid")
        .unwrap();
    assert_eq!(request.as_request().uri().query(), Some("type=Building&scopeQ=(/Madrid&limit=5"));
}

#[test]
fn a_parameter_is_added_last_after_the_local_flag() {
    let buildings = buildings();
    let local = QueryEntities {
        locality: Locality::Local,
        ..query(&buildings)
    };
    let request = UncheckedRequest::try_from_operation(&local, &endpoint())
        .unwrap()
        .query_parameter(QueryParameter::ScopeQ, "/Madrid;")
        .unwrap();
    assert_eq!(request.as_request().uri().query(), Some("type=Building&local=true&scopeQ=/Madrid;"));
}

#[test]
fn a_second_parameter_alongside_a_typed_one_sends_both() {
    let buildings = buildings();
    let pick: Projection = "id,type,name".parse().unwrap();
    let typed = QueryEntities {
        pick: Some(&pick),
        ..query(&buildings)
    };
    let request = UncheckedRequest::try_from_operation(&typed, &endpoint())
        .unwrap()
        .query_parameter(QueryParameter::Omit, "name")
        .unwrap();
    assert_eq!(request.as_request().uri().query(), Some("type=Building&pick=id,type,name&omit=name"));
}

#[test]
fn removing_the_type_sends_a_query_the_operation_refuses_to_build() {
    let buildings = buildings();
    let ids: [EntityId; 2] = ["urn:ngsi-ld:Building:1".parse().unwrap(), "urn:ngsi-ld:Building:2".parse().unwrap()];
    let bounded = QueryEntities {
        ids: &ids,
        ..query(&buildings)
    };
    let unbounded = QueryEntities { entity_type: None, ..bounded };
    assert!(unbounded.to_request(&endpoint()).is_err());
    let request = UncheckedRequest::try_from_operation(&bounded, &endpoint())
        .unwrap()
        .remove_query_parameter(QueryParameter::Type)
        .unwrap();
    assert_eq!(
        request.as_request().uri(),
        "http://broker.test/ngsi-ld/v1/entities?id=urn%3Angsi-ld%3ABuilding%3A1,urn%3Angsi-ld%3ABuilding%3A2"
    );
}

#[test]
fn removing_the_last_parameter_leaves_no_query_and_removing_an_absent_one_changes_nothing() {
    let buildings = buildings();
    let bare = unchecked(&buildings).remove_query_parameter(QueryParameter::Type).unwrap();
    assert_eq!(bare.as_request().uri(), "http://broker.test/ngsi-ld/v1/entities");
    let unchanged = unchecked(&buildings).remove_query_parameter(QueryParameter::Q).unwrap();
    assert_eq!(unchanged.as_request().uri().query(), Some("type=Building"));
}

#[test]
fn a_parameter_the_specification_does_not_define_is_added_escaped() {
    let buildings = buildings();
    let request = unchecked(&buildings).undefined_query_parameter("additionalParam", "type=Building").unwrap();
    assert_eq!(request.as_request().uri().query(), Some("type=Building&additionalParam=type%3DBuilding"));
    assert_eq!(sent(&request, "additionalParam"), ["type=Building"]);
}

#[test]
fn a_defined_parameter_is_refused_as_an_undefined_one() {
    let buildings = buildings();
    let refusal = unchecked(&buildings).undefined_query_parameter("scopeQ", "/Madrid");
    assert!(
        matches!(
            refusal,
            Err(RequestError::DefinedQueryParameter {
                parameter: QueryParameter::ScopeQ
            })
        ),
        "{refusal:?}"
    );
}
