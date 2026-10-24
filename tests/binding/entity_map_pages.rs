//! The entity map on the following pages of a query (clause 4.3.6.7): a
//! page request names the map the broker said it used, or else the map the query named.

use crate::{
    fixtures::{endpoint, header_value, response},
    links::{link_field, link_value},
    query_vehicles,
};
use bytes::Bytes;
use headers::Header;
use http::{HeaderName, Request, header};
use ngsi_ld::{
    http::{
        Page,
        ResponseContext,
        headers::{NgsiLdEntityMap, RelationType},
        operation::{Operation, Simplified},
    },
    model::SimplifiedEntity,
    query::SelectorType,
};
use pretty_assertions::assert_eq;

/// The map the query names.
const QUERY_MAP: &str = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1";

/// The map the broker answers with.
const BROKER_MAP: &str = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:2";

/// The `Link` to the second page.
fn next() -> String {
    link_field([link_value("?type=Vehicle&format=simplified&offset=1", RelationType::Next)])
}

/// The page answering a query that named `query_map`, with `headers` on the response.
fn page(query_map: Option<&str>, headers: &[(&HeaderName, &str)]) -> Page<SimplifiedEntity> {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let query = query_vehicles(Some(&entity_type), None, Simplified);
    let builder = Request::get("http://broker.test/ngsi-ld/v1/entities?type=Vehicle&format=simplified");
    let builder = match query_map {
        Some(map) => builder.header(NgsiLdEntityMap::name(), map),
        None => builder,
    };
    let context = ResponseContext::new(&builder.body(Bytes::new()).unwrap());
    query.decode(&context, response(200, headers, "[]")).unwrap()
}

/// The `NGSILD-EntityMap` the request for the page after `page` sends.
fn next_map(page: &Page<SimplifiedEntity>) -> Option<String> {
    let request = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    header_value(&request, NgsiLdEntityMap::name()).map(str::to_owned)
}

#[test]
fn the_following_page_names_the_map_the_broker_used() {
    let page = page(Some(QUERY_MAP), &[(&header::LINK, &next()), (NgsiLdEntityMap::name(), BROKER_MAP)]);
    assert_eq!(next_map(&page).as_deref(), Some(BROKER_MAP));
}

#[test]
fn without_a_map_from_the_broker_the_following_page_names_the_map_of_the_query() {
    let page = page(Some(QUERY_MAP), &[(&header::LINK, &next())]);
    assert_eq!(next_map(&page).as_deref(), Some(QUERY_MAP));
    let unreadable = page_with_unreadable_map();
    assert_eq!(next_map(&unreadable).as_deref(), Some(QUERY_MAP));
}

#[test]
fn a_query_without_a_map_sends_none_on_its_following_pages() {
    let page = page(None, &[(&header::LINK, &next())]);
    assert_eq!(next_map(&page), None);
}

/// A page whose response names a map that cannot be read.
fn page_with_unreadable_map() -> Page<SimplifiedEntity> {
    page(Some(QUERY_MAP), &[(&header::LINK, &next()), (NgsiLdEntityMap::name(), "a b")])
}
