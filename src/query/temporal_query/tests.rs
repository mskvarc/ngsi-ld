use crate::{
    model::{DateTime, DocumentError, Duration, MemberError, evolution::AggregationMethod},
    query::{
        Problem,
        temporal_query::{
            self,
            AggregationMethods,
            AggregationMethodsError,
            TemporalProperty,
            TemporalQuery,
            TemporalQueryError,
            TemporalQueryMember,
            TimeRelation,
            builders::TemporalQueryBuilder,
        },
    },
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;
use strum::IntoEnumIterator;

/// What the kind of a refusal must be.
type Expectation<'a> = &'a dyn Fn(&TemporalQueryError) -> bool;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// The text of clause 4.11 example 2 in its JSON form.
const EXAMPLE_2: &str = r#"{"timerel":"between","timeAt":"2017-12-13T14:20:00Z","endTimeAt":"2017-12-13T14:40:00Z","timeproperty":"modifiedAt"}"#;

/// The temporal query of Annex C.5.14, with `lastN`, in its JSON form.
const AGGREGATED: &str =
    r#"{"timerel":"between","timeAt":"2018-08-01T12:00:00Z","endTimeAt":"2018-08-01T13:00:00Z","lastN":5,"aggrMethods":"max,avg","aggrPeriodDuration":"PT4M"}"#;

/// The query of example 2, built.
fn example_2() -> TemporalQuery {
    temporal_query::between(time("2017-12-13T14:20:00Z"), time("2017-12-13T14:40:00Z"))
        .time_property(TemporalProperty::ModifiedAt)
        .build()
        .unwrap()
}

/// The query of [`AGGREGATED`], built.
fn aggregated() -> TemporalQuery {
    temporal_query::between(time("2018-08-01T12:00:00Z"), time("2018-08-01T13:00:00Z"))
        .last_n(5)
        .aggregation_methods([AggregationMethod::Max, AggregationMethod::Avg])
        .aggregation_period("PT4M".parse().unwrap())
        .build()
        .unwrap()
}

/// The error reading `text`.
fn refusal(text: &str) -> DocumentError<TemporalQueryError> {
    TemporalQuery::parse(text).unwrap_err()
}

#[test]
fn before_compares_with_one_time_and_writes_no_end() {
    let query = temporal_query::before(time("2017-12-13T14:20:00Z")).build().unwrap();
    assert_eq!(query.relation(), TimeRelation::Before);
    assert_eq!(query.time_at(), &time("2017-12-13T14:20:00Z"));
    assert_eq!(query.end_time_at(), None);
    assert_eq!(query.render().to_string(), r#"{"timerel":"before","timeAt":"2017-12-13T14:20:00Z"}"#);
}

#[test]
fn between_with_a_time_property_writes_example_2() {
    let query = example_2();
    assert_eq!(query.relation(), TimeRelation::Between);
    assert_eq!(query.end_time_at(), Some(&time("2017-12-13T14:40:00Z")));
    assert_eq!(query.time_property(), Some(TemporalProperty::ModifiedAt));
    assert_eq!(query.render().to_string(), EXAMPLE_2);
}

#[test]
fn after_writes_no_default_time_property() {
    let query = temporal_query::after(time("2018-08-01T12:03:00Z")).build().unwrap();
    assert_eq!(query.relation(), TimeRelation::After);
    assert_eq!(query.time_property(), None);
    assert_eq!(query.render().to_string(), r#"{"timerel":"after","timeAt":"2018-08-01T12:03:00Z"}"#);
}

#[test]
fn the_relations_print_and_parse_as_clause_4_11_spells_them() {
    for (text, relation) in [
        ("before", TimeRelation::Before),
        ("after", TimeRelation::After),
        ("between", TimeRelation::Between),
    ] {
        assert_eq!(text.parse::<TimeRelation>(), Ok(relation));
        assert_eq!(relation.to_string(), text);
    }
    for text in ["Before", "during", ""] {
        assert!(text.parse::<TimeRelation>().is_err(), "{text}");
    }
}

#[test]
fn the_temporal_properties_print_and_parse_as_table_5_2_21_1_spells_them() {
    for (text, property) in [
        ("observedAt", TemporalProperty::ObservedAt),
        ("createdAt", TemporalProperty::CreatedAt),
        ("modifiedAt", TemporalProperty::ModifiedAt),
        ("deletedAt", TemporalProperty::DeletedAt),
    ] {
        assert_eq!(text.parse::<TemporalProperty>(), Ok(property));
        assert_eq!(property.to_string(), text);
    }
    assert!("expiresAt".parse::<TemporalProperty>().is_err());
}

#[test]
fn last_n_and_aggregation_are_written_after_the_times() {
    let query = aggregated();
    assert_eq!(query.last_n().map(NonZeroU64::get), Some(5));
    assert_eq!(
        query.aggregation_methods().map(AggregationMethods::as_slice),
        Some([AggregationMethod::Max, AggregationMethod::Avg].as_slice())
    );
    assert_eq!(query.aggregation_period().map(|period| period.to_string()), Some("PT4M".to_owned()));
    assert_eq!(query.render().to_string(), AGGREGATED);
}

#[test]
fn the_builder_reports_every_problem_together() {
    let error = temporal_query::before(time("2018-08-01T12:03:00Z"))
        .last_n(0)
        .aggregation_methods([])
        .build()
        .unwrap_err();
    assert_eq!(error.problems(), [Problem::ZeroLastN, Problem::EmptyAggregationMethods]);
}

#[test]
fn the_json_form_reads_back_what_the_builder_wrote() {
    for (text, built) in [(EXAMPLE_2, example_2()), (AGGREGATED, aggregated())] {
        let read = TemporalQuery::parse(text).unwrap();
        assert_eq!(read, built);
        assert_eq!(read.render().to_string(), text);
    }
}

#[test]
fn between_needs_its_end_and_only_between_has_one() {
    let example_3 = refusal(r#"{"timerel":"between","timeAt":"2017-12-13T14:20:00Z","timeproperty":"observedAt"}"#);
    assert!(matches!(
        example_3.kind,
        TemporalQueryError::Member(MemberError::MissingMember { member: "endTimeAt" })
    ));
    let before = refusal(r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","endTimeAt":"2018-08-01T12:05:00Z"}"#);
    assert!(matches!(before.kind, TemporalQueryError::EndTimeWithoutBetween));
    assert_eq!(before.path.to_string(), "endTimeAt");
}

#[test]
fn the_json_reader_refuses_every_member_out_of_shape() {
    let missing = |member: TemporalQueryMember| move |error: &TemporalQueryError| matches!(error, TemporalQueryError::Member(MemberError::MissingMember { member: found }) if *found == <&str>::from(member));
    let cases: [(&str, &str, Expectation<'_>); 13] = [
        (r#"{"timerel":"before"}"#, "", &missing(TemporalQueryMember::TimeAt)),
        (r#"{"timeAt":"2018-08-01T12:03:00Z"}"#, "", &missing(TemporalQueryMember::Timerel)),
        (
            r#"{"timerel":"during","timeAt":"2018-08-01T12:03:00Z"}"#,
            "timerel",
            &|error| matches!(error, TemporalQueryError::Member(MemberError::InvalidToken { member: "timerel", found }) if found == "during"),
        ),
        (r#"{"timerel":"before","timeAt":"yesterday"}"#, "timeAt", &|error| {
            matches!(error, TemporalQueryError::Member(MemberError::Temporal { .. }))
        }),
        (
            r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","timeproperty":"expiresAt"}"#,
            "timeproperty",
            &|error| matches!(error, TemporalQueryError::Member(MemberError::InvalidToken { member: "timeproperty", .. })),
        ),
        (r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","lastN":0}"#, "lastN", &|error| {
            matches!(error, TemporalQueryError::LastN)
        }),
        (r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","lastN":"5"}"#, "lastN", &|error| {
            matches!(error, TemporalQueryError::LastN)
        }),
        (
            r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","aggrMethods":"max,median"}"#,
            "aggrMethods",
            &|error| matches!(error, TemporalQueryError::AggregationMethods { .. }),
        ),
        (
            r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","aggrMethods":["max"]}"#,
            "aggrMethods",
            &|error| matches!(error, TemporalQueryError::Member(MemberError::WrongShape { member: "aggrMethods", .. })),
        ),
        (
            r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","aggrPeriodDuration":"-PT1H"}"#,
            "aggrPeriodDuration",
            &|error| matches!(error, TemporalQueryError::Member(MemberError::Duration { .. })),
        ),
        (
            r#"{"timerel":"before","timeAt":"2018-08-01T12:03:00Z","foo":1}"#,
            "",
            &|error| matches!(error, TemporalQueryError::Member(MemberError::UnexpectedMember { member }) if member == "foo"),
        ),
        (
            r#"{"timerel":"before","timerel":"after","timeAt":"2018-08-01T12:03:00Z"}"#,
            "",
            &|error| matches!(error, TemporalQueryError::Member(MemberError::DuplicateMember { member }) if member == "timerel"),
        ),
        ("[]", "", &|error| matches!(error, TemporalQueryError::Member(MemberError::NotAnObject))),
    ];
    for (text, path, expected) in cases {
        let error = refusal(text);
        assert!(expected(&error.kind), "{text}: {error:?}");
        assert_eq!(error.path.to_string(), path, "{text}");
    }
}

#[test]
fn aggregation_methods_print_and_parse_as_a_comma_separated_list() {
    assert_eq!("max,avg".parse::<AggregationMethods>().unwrap().to_string(), "max,avg");
    for method in AggregationMethod::iter() {
        let text = method.to_string();
        assert_eq!(text.parse::<AggregationMethods>().unwrap().as_slice(), [method]);
    }
    assert_eq!("".parse::<AggregationMethods>(), Err(AggregationMethodsError::EmptyItem));
    assert_eq!("max,,avg".parse::<AggregationMethods>(), Err(AggregationMethodsError::EmptyItem));
    assert_eq!(
        "max,median".parse::<AggregationMethods>(),
        Err(AggregationMethodsError::UnknownMethod { method: "median".to_owned() })
    );
}

#[test]
fn a_built_query_takes_validated_members_without_the_builder() {
    let period: Duration = "PT0S".parse().unwrap();
    let query = TemporalQuery::after(time("2018-08-01T12:03:00Z")).with_aggregation_period(period);
    assert_eq!(
        query.render().to_string(),
        r#"{"timerel":"after","timeAt":"2018-08-01T12:03:00Z","aggrPeriodDuration":"PT0S"}"#
    );
    let rebuilt = TemporalQueryBuilder::from(query).last_n(2).build().unwrap();
    assert_eq!(rebuilt.aggregation_period(), Some(period));
    assert_eq!(rebuilt.last_n().map(NonZeroU64::get), Some(2));
}
