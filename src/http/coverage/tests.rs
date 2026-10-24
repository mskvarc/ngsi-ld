use crate::{
    http::{
        Coverage,
        headers::{ContentRange, OptionalHeader},
        operation::TemporalSelection,
    },
    model::DateTime,
    query::temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery},
};
use headers::Header;
use http::HeaderValue;
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

const T0: &str = "2018-08-01T12:00:00Z";
const T5: &str = "2018-08-01T12:05:00Z";
const T60: &str = "2018-08-01T13:00:00Z";

fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

fn partial(raw: &'static str) -> Coverage {
    let value = HeaderValue::from_static(raw);
    Coverage::Partial(OptionalHeader::Valid(ContentRange::decode(&mut [&value].into_iter()).unwrap()))
}

fn three() -> NonZeroU64 {
    NonZeroU64::new(3).unwrap()
}

#[test]
fn a_forwards_after_or_between_continues_from_the_most_recent_instance() {
    let coverage = partial("DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/*");
    let after = TemporalQuery::after(time(T0));
    assert_eq!(coverage.following_query(TemporalSelection::Query(&after)), Some(TemporalQuery::after(time(T5))));
    let between = TemporalQuery::between(time(T0), time(T60));
    assert_eq!(
        coverage.following_query(TemporalSelection::Query(&between)),
        Some(TemporalQuery::between(time(T5), time(T60)))
    );
}

#[test]
fn a_forwards_before_continues_between_the_most_recent_instance_and_its_bound() {
    let coverage = partial("DateTime 2018-08-01T11:00:00Z-2018-08-01T12:05:00Z/*");
    let before = TemporalQuery::before(time(T60));
    assert_eq!(
        coverage.following_query(TemporalSelection::Query(&before)),
        Some(TemporalQuery::between(time(T5), time(T60)))
    );
}

#[test]
fn a_backwards_query_continues_before_the_least_recent_instance_and_keeps_its_options() {
    let coverage = partial("DateTime 2018-08-01T13:00:00Z-2018-08-01T12:05:00Z/3");
    let before = TemporalQuery::before(time(T60))
        .with_last_n(three())
        .with_time_property(TemporalProperty::ModifiedAt);
    assert_eq!(
        coverage.following_query(TemporalSelection::Query(&before)),
        Some(
            TemporalQuery::before(time(T5))
                .with_time_property(TemporalProperty::ModifiedAt)
                .with_last_n(three())
        )
    );
    let between = TemporalQuery::between(time(T0), time(T60)).with_last_n(three());
    assert_eq!(
        coverage.following_query(TemporalSelection::Query(&between)),
        Some(TemporalQuery::between(time(T0), time(T5)).with_last_n(three()))
    );
    let after = TemporalQuery::after(time(T0)).with_last_n(three());
    assert_eq!(
        coverage.following_query(TemporalSelection::Query(&after)),
        Some(TemporalQuery::between(time(T0), time(T5)).with_last_n(three()))
    );
}

#[test]
fn an_unbounded_selection_continues_after_or_before_the_boundary_with_its_aggregation() {
    let methods: AggregationMethods = "max,avg".parse().unwrap();
    let forwards = TemporalSelection::Unbounded {
        time_property: None,
        last_n: None,
        aggregation_methods: Some(&methods),
        aggregation_period: Some("PT1M".parse().unwrap()),
    };
    assert_eq!(
        partial("DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/*").following_query(forwards),
        Some(
            TemporalQuery::after(time(T5))
                .with_aggregation_methods(methods.clone())
                .with_aggregation_period("PT1M".parse().unwrap())
        )
    );
    let backwards = TemporalSelection::Unbounded {
        time_property: None,
        last_n: Some(three()),
        aggregation_methods: None,
        aggregation_period: None,
    };
    assert_eq!(
        partial("DateTime 2018-08-01T13:00:00Z-2018-08-01T12:05:00Z/3").following_query(backwards),
        Some(TemporalQuery::before(time(T5)).with_last_n(three()))
    );
}

#[test]
fn a_whole_answer_or_one_without_a_readable_range_has_no_following_query() {
    let query = TemporalQuery::after(time(T0));
    for coverage in [
        Coverage::Whole,
        Coverage::Partial(OptionalHeader::Absent),
        Coverage::Partial(OptionalHeader::Malformed(vec![HeaderValue::from_static("bytes 0-1/2")])),
    ] {
        assert_eq!(coverage.following_query(TemporalSelection::Query(&query)), None);
    }
}

#[test]
fn a_boundary_that_makes_no_progress_or_leaves_the_range_has_no_following_query() {
    let between = TemporalQuery::between(time(T0), time(T60));
    for raw in [
        // Ends where the request began.
        "DateTime 2018-08-01T12:00:00Z-2018-08-01T12:00:00Z/*",
        // Ends at or past the far end.
        "DateTime 2018-08-01T12:00:00Z-2018-08-01T13:00:00Z/*",
        "DateTime 2018-08-01T12:00:00Z-2018-08-01T14:00:00Z/*",
        // Ends before the request began.
        "DateTime 2018-08-01T11:00:00Z-2018-08-01T11:30:00Z/*",
    ] {
        assert_eq!(partial(raw).following_query(TemporalSelection::Query(&between)), None, "{raw}");
    }
    let backwards = TemporalQuery::before(time(T60)).with_last_n(three());
    assert_eq!(
        partial("DateTime 2018-08-01T14:00:00Z-2018-08-01T13:30:00Z/3").following_query(TemporalSelection::Query(&backwards)),
        None
    );
}
