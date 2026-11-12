//! The temporal pagination of ETSI TS 104 176 clause 6.4.7.3: `firstN` and `offsetN` in the requests, and the
//! `intervalafter` and `intervalbefore` links of the answers.

use crate::{
    fixtures::{endpoint, response},
    links::{link_field, link_value},
    temporal_fixtures::{WINDOW, b, query_string, window},
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, header};
use ngsi_ld::{
    http::{
        RequestError,
        ResponseContext,
        headers::{NgsiLdEntityMap, RelationType},
        operation::{
            ArgumentError,
            InstancePaging,
            NormalizedTemporal,
            Operation,
            ParameterError,
            QueryMethod,
            QueryTemporalEvolution,
            QueryTemporalEvolutionParameters,
            QueryTemporalEvolutionPostParameters,
            RetrieveTemporalEvolution,
            RetrieveTemporalEvolutionParameters,
            TemporalSelection,
        },
    },
    model::{Complete, EntityId},
    query::{SelectorType, temporal_query::TemporalQuery},
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

/// `B`'s evolution as a broker returns it: one `speed` instance.
const RETRIEVED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":["#,
    r#"{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:attribute:instance:1"}]}"#
);

/// The first two instances of each attribute, from the third on.
const SECOND_PAGE: InstancePaging = InstancePaging {
    first_n: NonZeroU64::new(2),
    offset_n: Some(2),
};

/// A retrieve of `id` with `window`, normalized, paged by `paging`.
fn retrieve<'a>(id: &'a EntityId, window: &'a TemporalQuery, paging: InstancePaging) -> RetrieveTemporalEvolution<'a, NormalizedTemporal, Complete> {
    RetrieveTemporalEvolution::builder()
        .id(id)
        .selection(TemporalSelection::Query(window))
        .instance_paging(paging)
        .format(NormalizedTemporal)
        .build()
}

/// A query of `Vehicle` with `window`, normalized, paged by `paging`, through `method`.
fn query<'a>(
    entity_type: &'a SelectorType,
    window: &'a TemporalQuery,
    paging: InstancePaging,
    method: QueryMethod,
) -> QueryTemporalEvolution<'a, NormalizedTemporal, Complete> {
    QueryTemporalEvolution::builder()
        .entity_type(entity_type)
        .temporal_q(window)
        .instance_paging(paging)
        .method(method)
        .format(NormalizedTemporal)
        .build()
}

/// The response context of `request`.
fn context_of(request: &Request<Bytes>) -> ResponseContext {
    ResponseContext::new(request)
}

#[test]
fn a_retrieve_sends_first_n_and_offset_n_after_the_temporal_selection() {
    let window = window();
    assert_eq!(
        query_string(&retrieve(&b(), &window, SECOND_PAGE)),
        Some(format!("{WINDOW}&firstN=2&offsetN=2"))
    );
    assert_eq!(query_string(&retrieve(&b(), &window, InstancePaging::default())), Some(WINDOW.to_owned()));
}

#[test]
fn a_get_and_a_post_query_send_first_n_and_offset_n_in_the_query_string() {
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let window = window();
    let get = query_string(&query(&vehicles, &window, SECOND_PAGE, QueryMethod::Get)).unwrap();
    assert_eq!(get, format!("type=Vehicle&{WINDOW}&firstN=2&offsetN=2"));
    let post = query_string(&query(&vehicles, &window, SECOND_PAGE, QueryMethod::Post));
    assert_eq!(post.as_deref(), Some("firstN=2&offsetN=2"));
}

#[test]
fn first_n_beside_a_last_n_is_refused_before_sending() {
    // ETSI TS 104 176 clause 6.4.7.3 pages ascending with firstN and descending with lastN, and gives both no meaning.
    let last_three = window().with_last_n(NonZeroU64::new(3).unwrap());
    let refused = retrieve(&b(), &last_three, SECOND_PAGE).to_request(&endpoint()).unwrap_err();
    assert!(matches!(refused, RequestError::Argument(ArgumentError::FirstNBesideLastN)), "{refused:?}");
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let refused = query(&vehicles, &last_three, SECOND_PAGE, QueryMethod::Get)
        .to_request(&endpoint())
        .unwrap_err();
    assert!(matches!(refused, RequestError::Argument(ArgumentError::FirstNBesideLastN)), "{refused:?}");
    let offset_only = InstancePaging {
        first_n: None,
        offset_n: Some(3),
    };
    assert!(retrieve(&b(), &last_three, offset_only).to_request(&endpoint()).is_ok());
}

#[test]
fn a_received_temporal_request_reads_first_n_and_offset_n() {
    let retrieve = RetrieveTemporalEvolutionParameters::parse(&format!("{WINDOW}&firstN=2&offsetN=4")).unwrap();
    assert_eq!(
        retrieve.instance_paging,
        InstancePaging {
            first_n: NonZeroU64::new(2),
            offset_n: Some(4),
        }
    );
    let get = QueryTemporalEvolutionParameters::parse(&format!("type=Vehicle&{WINDOW}&offsetN=1")).unwrap();
    assert_eq!(get.instance_paging.offset_n, Some(1));
    assert_eq!(get.instance_paging.first_n, None);
    let post = QueryTemporalEvolutionPostParameters::parse("firstN=5").unwrap();
    assert_eq!(post.instance_paging.first_n, NonZeroU64::new(5));
    for refused in ["firstN=0", "firstN=-1", "offsetN=x"] {
        assert!(
            matches!(QueryTemporalEvolutionPostParameters::parse(refused), Err(ParameterError::Integer { .. })),
            "{refused}"
        );
    }
}

#[test]
fn a_retrieve_answer_reads_its_interval_links_resolved_against_the_request() {
    let window = window();
    let id = b();
    let operation = retrieve(&id, &window, InstancePaging::default());
    let sent = operation.to_request(&endpoint()).unwrap();
    let links = link_field([
        link_value(&format!("?{WINDOW}&firstN=2&offsetN=2"), RelationType::IntervalAfter),
        link_value(&format!("?{WINDOW}&lastN=2&offsetN=2"), RelationType::IntervalBefore),
    ]);
    let answer = operation
        .decode(&context_of(&sent), response(200, &[(&header::LINK, &links)], RETRIEVED))
        .unwrap();
    let base = "http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211";
    assert_eq!(
        answer.interval_after().map(ToString::to_string),
        Some(format!("{base}?{WINDOW}&firstN=2&offsetN=2"))
    );
    assert_eq!(
        answer.interval_before().map(ToString::to_string),
        Some(format!("{base}?{WINDOW}&lastN=2&offsetN=2"))
    );
    let unlinked = operation.decode(&context_of(&sent), response(200, &[], RETRIEVED)).unwrap();
    assert_eq!((unlinked.interval_after(), unlinked.interval_before()), (None, None));
}

#[test]
fn the_interval_request_gets_the_link_naming_the_entity_map_the_answer_reports() {
    let window = window();
    let id = b();
    let operation = retrieve(&id, &window, SECOND_PAGE);
    let sent = operation.to_request(&endpoint()).unwrap();
    let map = "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:entitymap:1";
    let links = link_field([link_value(&format!("?{WINDOW}&firstN=2&offsetN=4"), RelationType::IntervalAfter)]);
    let answer = operation
        .decode(
            &context_of(&sent),
            response(200, &[(&header::LINK, &links), (NgsiLdEntityMap::name(), map)], RETRIEVED),
        )
        .unwrap();
    let following = answer.interval_after_request().unwrap();
    assert!(answer.interval_before_request().is_none());
    let request = following.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(
        request.uri().to_string(),
        format!("http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211?{WINDOW}&firstN=2&offsetN=4")
    );
    assert_eq!(request.uri().to_string(), following.url().to_string());
    assert_eq!(request.headers()[&header::ACCEPT], "application/json");
    assert_eq!(request.headers()[NgsiLdEntityMap::name()], map);
    let later = following.decode(&context_of(&request), response(200, &[], RETRIEVED)).unwrap();
    assert_eq!(later.body().id, b());
}

#[test]
fn a_query_page_reads_the_interval_link_of_the_binding_example() {
    // The link of ETSI TS 104 176 clause A.3.1.7: the next two instances, from the third on.
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let window = window();
    let paging = InstancePaging {
        first_n: NonZeroU64::new(2),
        offset_n: None,
    };
    let operation = query(&vehicles, &window, paging, QueryMethod::Get);
    let sent = operation.to_request(&endpoint()).unwrap();
    let target = format!("?type=Vehicle&{WINDOW}&firstN=2&offsetN=2");
    let links = link_field([link_value(&target, RelationType::IntervalAfter)]);
    let page = operation
        .decode(&context_of(&sent), response(200, &[(&header::LINK, &links)], &format!("[{RETRIEVED}]")))
        .unwrap();
    let expected = format!("http://broker.test/ngsi-ld/v1/temporal/entities{target}");
    assert_eq!(page.interval_after().map(ToString::to_string), Some(expected.clone()));
    assert_eq!(page.interval_before(), None);
    assert_eq!(page.next(), None);
    let following = page.interval_after_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(following.uri().to_string(), expected);
    assert!(page.interval_before_page().is_none());
}
