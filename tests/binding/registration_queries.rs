//! Query Context Source Registrations (clauses 5.10.2 and 6.8.3.2): the query
//! string, the refusals before sending, the following pages and the pages read.

use crate::{
    fixtures::{context, endpoint, header_value, link_of, response},
    links::{context_link, link_field, link_value},
    registration_fixtures::{R, RETRIEVED, query_string, registration_error, vehicles},
};
use headers::Header;
use http::{Method, header};
use ngsi_ld::{
    http::{
        BodyError,
        Page,
        RequestError,
        ResponseContext,
        ResponseError,
        headers::{NgsiLdResultsCount, NgsiLdTenant, RelationType},
        operation::{ArgumentError, Operation, QueryRegistrations, ResultsCount, SystemAttributes},
    },
    model::{AttributeName, EntityId, MemberError, Output, Tenant},
    query::{
        IdPattern,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        q::{self, Query},
        scope_query::ScopeQuery,
        temporal_query::{TemporalProperty, TemporalQuery},
    },
    registration::CSourceRegistration,
    subscription::ReadErrorKind,
};
use pretty_assertions::assert_eq;
use std::{num::NonZeroU64, str};

#[test]
fn a_query_by_type_gets_the_collection_and_asks_for_json() {
    let entity_type = vehicles();
    let query = QueryRegistrations {
        entity_type: Some(&entity_type),
        ..QueryRegistrations::default()
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/csourceRegistrations");
    assert_eq!(request.uri().query(), Some("type=Vehicle"));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert!(request.body().is_empty());
}

#[test]
fn a_query_writes_every_parameter_of_its_table_in_a_fixed_order() {
    let entity_type = vehicles();
    let ids: [EntityId; 1] = ["urn:ngsi-ld:Vehicle:A4567".parse().unwrap()];
    let id_pattern: IdPattern = "urn:ngsi-ld:Vehicle:.*".parse().unwrap();
    let attrs: [AttributeName; 2] = ["speed".parse().unwrap(), "isParked".parse().unwrap()];
    let q: Query = q::attr("speed").gt(50).build().unwrap();
    let csf: Query = q::attr("mode").eq("inclusive").build().unwrap();
    let geo_q = GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}"#).unwrap();
    let temporal_q = TemporalQuery::between("2024-01-01T00:00:00Z".parse().unwrap(), "2024-02-01T00:00:00Z".parse().unwrap())
        .with_time_property(TemporalProperty::ModifiedAt);
    let scope_q: ScopeQuery = "/Madrid/#".parse().unwrap();
    let lang: LanguageFilter = "en".parse().unwrap();
    let query = QueryRegistrations::builder()
        .entity_type(&entity_type)
        .ids(&ids)
        .id_pattern(&id_pattern)
        .attrs(&attrs)
        .q(&q)
        .csf(&csf)
        .geo_q(&geo_q)
        .temporal_q(&temporal_q)
        .scope_q(&scope_q)
        .lang(&lang)
        .system_attributes(SystemAttributes::Include)
        .limit(10)
        .offset(20)
        .count(ResultsCount::Include)
        .build();
    assert_eq!(
        query_string(&query).as_deref(),
        Some(concat!(
            "type=Vehicle&id=urn%3Angsi-ld%3AVehicle%3AA4567&idPattern=urn%3Angsi-ld%3AVehicle%3A.%2A&attrs=speed,isParked",
            "&q=speed%3E50&georel=near;maxDistance==2000&geometry=Point&coordinates=%5B8%2C40%5D&scopeQ=/Madrid/%23",
            "&csf=mode%3D%3D%22inclusive%22&lang=en&timerel=between&timeAt=2024-01-01T00%3A00%3A00Z",
            "&endTimeAt=2024-02-01T00%3A00%3A00Z&timeproperty=modifiedAt&options=sysAttrs&limit=10&offset=20&count=true"
        ))
    );
}

#[test]
fn a_query_needs_a_type_attributes_a_q_or_a_geoquery() {
    let ids: [EntityId; 1] = ["urn:ngsi-ld:Vehicle:A4567".parse().unwrap()];
    let scope_q: ScopeQuery = "/Madrid/#".parse().unwrap();
    let temporal_q = TemporalQuery::after("2024-01-01T00:00:00Z".parse().unwrap());
    let unbounded = [
        QueryRegistrations::default(),
        QueryRegistrations {
            ids: &ids,
            ..QueryRegistrations::default()
        },
        QueryRegistrations {
            scope_q: Some(&scope_q),
            ..QueryRegistrations::default()
        },
        QueryRegistrations {
            temporal_q: Some(&temporal_q),
            ..QueryRegistrations::default()
        },
    ];
    for query in unbounded {
        let error = query.to_request(&endpoint()).unwrap_err();
        assert!(matches!(error, RequestError::Argument(ArgumentError::UnboundedRegistrationQuery)), "{error:?}");
    }
    let attrs: [AttributeName; 1] = ["speed".parse().unwrap()];
    let by_attribute = QueryRegistrations {
        attrs: &attrs,
        ..QueryRegistrations::default()
    };
    assert_eq!(query_string(&by_attribute).as_deref(), Some("attrs=speed"));
    let q: Query = q::attr("speed").gt(50).build().unwrap();
    let by_q = QueryRegistrations {
        q: Some(&q),
        ..QueryRegistrations::default()
    };
    assert_eq!(query_string(&by_q).as_deref(), Some("q=speed%3E50"));
    let geo_q = GeoQuery::parse(r#"{"georel":"within","geometry":"Point","coordinates":[8,40]}"#).unwrap();
    let by_geoquery = QueryRegistrations {
        geo_q: Some(&geo_q),
        ..QueryRegistrations::default()
    };
    assert_eq!(
        query_string(&by_geoquery).as_deref(),
        Some("georel=within&geometry=Point&coordinates=%5B8%2C40%5D")
    );
}

#[test]
fn a_query_refuses_instance_options_and_a_zero_limit_without_count() {
    let entity_type = vehicles();
    let last_two = TemporalQuery::after("2024-01-01T00:00:00Z".parse().unwrap()).with_last_n(NonZeroU64::MIN.saturating_add(1));
    let query = QueryRegistrations {
        entity_type: Some(&entity_type),
        temporal_q: Some(&last_two),
        ..QueryRegistrations::default()
    };
    let error = query.to_request(&endpoint()).unwrap_err();
    assert!(
        matches!(error, RequestError::Argument(ArgumentError::InstanceOptionsOnRegistrations)),
        "{error:?}"
    );
    let zero = QueryRegistrations {
        entity_type: Some(&entity_type),
        limit: Some(0),
        ..QueryRegistrations::default()
    };
    let error = zero.to_request(&endpoint()).unwrap_err();
    assert!(matches!(error, RequestError::Argument(ArgumentError::ZeroLimitWithoutCount)), "{error:?}");
    let counted = QueryRegistrations {
        count: ResultsCount::Include,
        ..zero
    };
    assert_eq!(query_string(&counted).as_deref(), Some("type=Vehicle&limit=0&count=true"));
}

#[test]
fn the_next_page_of_registrations_stays_on_the_tenant_and_asks_for_json() {
    let tenant: Tenant = "t1".parse().unwrap();
    let entity_type = vehicles();
    let query = QueryRegistrations {
        entity_type: Some(&entity_type),
        limit: Some(1),
        ..QueryRegistrations::default()
    };
    let request = query.to_request(&endpoint().with_tenant(tenant)).unwrap();
    let page = query
        .decode(
            &ResponseContext::new(&request),
            response(
                200,
                &[(
                    &header::LINK,
                    &link_field([link_value("/ngsi-ld/v1/csourceRegistrations?type=Vehicle&limit=1&offset=1", RelationType::Next)]),
                )],
                &format!("[{RETRIEVED}]"),
            ),
        )
        .unwrap();
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(next.method(), Method::GET);
    assert_eq!(
        next.uri().to_string(),
        "http://broker.test/ngsi-ld/v1/csourceRegistrations?type=Vehicle&limit=1&offset=1"
    );
    assert_eq!(header_value(&next, &header::ACCEPT), Some("application/json"));
    assert_eq!(header_value(&next, NgsiLdTenant::name()), Some("t1"));
}

#[test]
fn a_query_reads_a_page_of_registrations_with_its_links_and_total() {
    let entity_type = vehicles();
    let query = QueryRegistrations {
        entity_type: Some(&entity_type),
        ..QueryRegistrations::default()
    };
    let second = RETRIEVED.replace("csr1", "csr2");
    let page: Page<CSourceRegistration<Output>> = query
        .decode(
            &context(),
            response(
                200,
                &[
                    (
                        &header::LINK,
                        &link_field([link_value("/ngsi-ld/v1/csourceRegistrations?type=Vehicle&offset=2", RelationType::Next)]),
                    ),
                    (NgsiLdResultsCount::name(), "3"),
                ],
                &format!("[{RETRIEVED},{second}]"),
            ),
        )
        .unwrap();
    let ids: Vec<&str> = page.items().iter().map(|registration| registration.id().as_str()).collect();
    assert_eq!(ids, [R, "urn:ngsi-ld:ContextSourceRegistration:csr2"]);
    assert_eq!(page.total(), Some(3));
    assert_eq!(
        page.next().map(ToString::to_string).as_deref(),
        Some("http://broker.test/ngsi-ld/v1/csourceRegistrations?type=Vehicle&offset=2")
    );
}

#[test]
fn a_query_answer_that_is_not_a_list_of_registrations_fails() {
    let entity_type = vehicles();
    let query = QueryRegistrations {
        entity_type: Some(&entity_type),
        ..QueryRegistrations::default()
    };
    let error = query.decode(&context(), response(200, &[], RETRIEVED)).unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::NotAnArray)),
        "{error:?}"
    );
    let without_id = RETRIEVED.replacen(&format!(r#""id":"{R}","#), "", 1);
    let error = query
        .decode(&context(), response(200, &[], &format!("[{RETRIEVED},{without_id}]")))
        .unwrap_err();
    let error = registration_error(error);
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "id" })),
        "{error:?}"
    );
    assert_eq!(error.path.to_string(), "1");
}
