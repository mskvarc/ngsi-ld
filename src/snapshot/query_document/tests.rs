//! The queries a snapshot holds.

use crate::{
    model::{
        AttributeName,
        BuildError,
        EntityId,
        MemberError,
        MemberProblem,
        evolution::{AggregationMethod, TemporalAttributeName},
    },
    query::{EntityMapReturn, EntitySelector, EntitySplitting, Join, temporal_query::TemporalQuery},
    snapshot::{EntityQuery, Problem, QueryDocument, QueryMember, SnapshotField, TemporalEvolutionQuery},
    subscription::ReadErrorKind,
};
use iri_rs::iri;
use pretty_assertions::assert_eq;
use std::num::NonZeroU32;

/// An entity query holding every member of Table 5.2.23-1, in canonical order.
const EVERY_MEMBER: &str = concat!(
    r#"{"type":"Query","entities":[{"type":"Vehicle"},{"type":"Car","id":"urn:ngsi-ld:Vehicle:A4567"}],"q":"speed>50","#,
    r#""geoQ":{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]},"scopeQ":"/Madrid","attrs":["speed"],"#,
    r#""pick":["speed","brandName"],"omit":["scope"],"csf":"brandName==\"Mercedes\"","containedBy":["urn:ngsi-ld:Building:1"],"#,
    r#""datasetId":["@none","urn:ngsi-ld:dataset:gps"],"expandValues":"gender","entityMap":true,"entityMapLifetime":"PT1H","#,
    r#""jsonKeys":"parkingTickets","join":"inline","joinLevel":2,"lang":"fr-CH,fr;q=0.9","#,
    r#""ordering":{"orderBy":["name","location;dist-asc"],"collation":"und-u-ks-identic","geometry":"LineString","coordinates":[[13.3986,52.5547],[13.4,52.6]]},"#,
    r#""splitEntities":false}"#
);

#[test]
fn an_entity_query_with_every_member_round_trips_typed() {
    let query = QueryDocument::<EntityQuery>::parse(EVERY_MEMBER).unwrap();
    assert_eq!(query.render().to_string(), EVERY_MEMBER);
    assert_eq!(query.entities().len(), 2);
    assert_eq!(query.join(), Some(Join::Inline));
    assert_eq!(query.join_level().map(NonZeroU32::get), Some(2));
    assert_eq!(query.entity_map(), EntityMapReturn::Returned);
    assert_eq!(query.split_entities(), EntitySplitting::Whole);
    assert_eq!(query.dataset_ids().map(<[_]>::len), Some(2));
    assert!(query.ordering().is_some_and(|ordering| ordering.origin().is_some()));
}

#[test]
fn a_temporal_query_carries_its_aggregation() {
    let text = r#"{"type":"Query","entities":[{"type":"Vehicle"}],"temporalQ":{"timerel":"before","timeAt":"2030-01-01T00:00:00Z"},"aggrParams":{"aggrMethods":"min,max","aggrPeriodDuration":"PT1H"}}"#;
    let query = QueryDocument::<TemporalEvolutionQuery>::parse(text).unwrap();
    assert_eq!(query.render().to_string(), text);
    assert_eq!(query.temporal_q().time_at().to_string(), "2030-01-01T00:00:00Z");
    assert_eq!(
        query.aggr_params().map(|params| params.methods().as_slice().to_vec()),
        Some(vec![AggregationMethod::Min, AggregationMethod::Max])
    );
    let error = QueryDocument::<TemporalEvolutionQuery>::parse(&text.replace(r#"{"aggrMethods":"min,max","aggrPeriodDuration":"PT1H"}"#, "{}")).unwrap_err();
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "aggrMethods" })),
        "{error:?}"
    );
    assert_eq!(error.path.to_string(), "aggrParams");
}

#[test]
fn a_query_builder_refuses_only_what_the_tables_forbid() {
    let problems = |build: Result<QueryDocument<EntityQuery>, BuildError<Problem>>| build.unwrap_err().problems().to_vec();
    let levelled = QueryDocument::<EntityQuery>::builder().join_level(2).build().unwrap();
    assert_eq!(levelled.join_level(), NonZeroU32::new(2));
    let empty = QueryDocument::<EntityQuery>::parse(r#"{"type":"Query","containedBy":[]}"#).unwrap_err();
    assert_eq!(empty.path.to_string(), "containedBy");
    let contained = QueryDocument::<EntityQuery>::builder()
        .contained_by([EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"))])
        .build()
        .unwrap();
    assert_eq!(contained.contained_by().len(), 1);
    let forwarded = QueryDocument::<EntityQuery>::parse(r#"{"type":"Query","containedBy":["urn:ngsi-ld:Vehicle:A4567"]}"#).unwrap();
    assert_eq!(forwarded, contained);
    assert_eq!(
        problems(QueryDocument::builder().entities(Vec::<EntitySelector>::new()).build()),
        [Problem::Member(MemberProblem::Empty {
            member: SnapshotField::Query(QueryMember::Entities)
        })]
    );
    let temporal = QueryDocument::temporal_builder(TemporalQuery::after("2030-01-01T00:00:00Z".parse().unwrap()))
        .join(Join::Flat)
        .join_level(1)
        .build()
        .unwrap();
    assert_eq!(temporal.join(), Some(Join::Flat));
}

#[test]
fn an_invalid_attribute_name_and_a_join_level_of_zero_are_named_by_their_member() {
    let error = QueryDocument::<EntityQuery>::builder().attrs(["1x"]).join_level(0).build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [
                Problem::Member(MemberProblem::NotPositive {
                    member: SnapshotField::Query(QueryMember::JoinLevel),
                    ..
                }),
                Problem::Member(MemberProblem::InvalidAttributeName {
                    member: SnapshotField::Query(QueryMember::Attrs),
                    index: 0,
                    ..
                }),
            ]
        ),
        "{error:?}"
    );
}

#[test]
fn only_a_temporal_query_names_the_scope_in_attrs() {
    let temporal = QueryDocument::temporal_builder(TemporalQuery::after("2018-08-01T12:00:00Z".parse().unwrap()))
        .attrs(["speed", "scope"])
        .build()
        .unwrap();
    assert_eq!(
        temporal.attrs(),
        [
            TemporalAttributeName::from("speed".parse::<AttributeName>().unwrap()),
            TemporalAttributeName::Scope
        ]
    );
    let text = temporal.render().to_string();
    assert_eq!(
        text,
        r#"{"type":"Query","temporalQ":{"timerel":"after","timeAt":"2018-08-01T12:00:00Z"},"attrs":["speed","scope"]}"#
    );
    assert_eq!(QueryDocument::<TemporalEvolutionQuery>::parse(&text).unwrap(), temporal);
    let error = QueryDocument::<EntityQuery>::builder().attrs(["speed", "scope"]).build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [Problem::Member(MemberProblem::InvalidAttributeName {
                member: SnapshotField::Query(QueryMember::Attrs),
                index: 1,
                ..
            })]
        ),
        "{error:?}"
    );
    assert!(QueryDocument::<EntityQuery>::parse(r#"{"type":"Query","attrs":["scope"]}"#).is_err());
}

#[test]
fn an_empty_dataset_id_list_round_trips_and_differs_from_none() {
    let text = r#"{"type":"Query","datasetId":[]}"#;
    let query = QueryDocument::<EntityQuery>::parse(text).unwrap();
    assert_eq!(query.dataset_ids().map(<[_]>::len), Some(0));
    assert_eq!(query.render().to_string(), text);
    assert_eq!(QueryDocument::<EntityQuery>::parse(r#"{"type":"Query"}"#).unwrap().dataset_ids(), None);
}

#[test]
fn an_entity_type_given_as_text_selects_that_type_and_is_checked_when_built() {
    let query = QueryDocument::builder().entities(["Vehicle"]).build().unwrap();
    assert_eq!(query.render().to_string(), r#"{"type":"Query","entities":[{"type":"Vehicle"}]}"#);
    let error = QueryDocument::builder().entities(["Vehicle", "Car;Bus"]).build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [Problem::Member(MemberProblem::InvalidSelectedType {
                member: SnapshotField::Query(QueryMember::Entities),
                index: 1,
                ..
            })]
        ),
        "{error:?}"
    );
}
