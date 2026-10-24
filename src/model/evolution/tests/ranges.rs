//! Joining the answers of two time ranges (clause 6.3.10).

use crate::{
    model::{Lossless, Output, evolution::TemporalEntity},
    query::temporal_query::TemporalProperty,
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

/// Reads `text` as a response, normalized.
fn output(text: &str) -> TemporalEntity<Output> {
    TemporalEntity::<Output>::parse(text, Lossless::Normalized).unwrap()
}

#[test]
fn a_later_range_adds_its_instances_without_the_ones_at_the_boundary() {
    let mut first = output(concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:i:1"},"#,
        r#"{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z","instanceId":"urn:ngsi-ld:i:2"}],"#,
        r#""rpm":[{"type":"Property","value":3000,"observedAt":"2018-08-01T12:05:00Z"}]}"#
    ));
    let later = output(concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""speed":[{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z","instanceId":"urn:ngsi-ld:i:2"},"#,
        r#"{"type":"Property","value":100,"observedAt":"2018-08-01T12:07:00Z","instanceId":"urn:ngsi-ld:i:3"}],"#,
        r#""rpm":[{"type":"Property","value":3000,"observedAt":"2018-08-01T12:05:00Z"},"#,
        r#"{"type":"Property","value":2500,"observedAt":"2018-08-01T12:07:00Z"}],"#,
        r#""brandName":[{"type":"Property","value":"Mercedes","observedAt":"2018-08-01T12:07:00Z"}]}"#
    ));
    first.absorb(later);
    assert_eq!(
        first.render(Lossless::Concise).to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:i:1"},"#,
            r#"{"value":80,"observedAt":"2018-08-01T12:05:00Z","instanceId":"urn:ngsi-ld:i:2"},"#,
            r#"{"value":100,"observedAt":"2018-08-01T12:07:00Z","instanceId":"urn:ngsi-ld:i:3"}],"#,
            r#""rpm":[{"value":3000,"observedAt":"2018-08-01T12:05:00Z"},{"value":2500,"observedAt":"2018-08-01T12:07:00Z"}],"#,
            r#""brandName":[{"value":"Mercedes","observedAt":"2018-08-01T12:07:00Z"}]}"#
        )
    );
}

/// The values of `speed` once `older` is joined to the most recent answer with `lastN` `last_n` by `property`.
fn last_speeds(last_n: u64, property: TemporalProperty) -> String {
    let mut recent = output(concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""speed":[{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z","createdAt":"2018-08-01T12:05:01Z","instanceId":"urn:ngsi-ld:i:2"}]}"#
    ));
    let older = output(concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""speed":[{"type":"Property","value":100,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:i:3"},"#,
        r#"{"type":"Property","value":120,"observedAt":"2018-08-01T12:01:00Z","createdAt":"2018-08-01T12:01:01Z","instanceId":"urn:ngsi-ld:i:1"}],"#,
        r#""rpm":[{"type":"Property","value":3000,"observedAt":"2018-08-01T12:04:00Z"}]}"#
    ));
    recent.absorb_last(older, NonZeroU64::new(last_n).unwrap(), property);
    assert_eq!(recent.attribute(&"rpm".parse().unwrap()).unwrap().len(), 1);
    recent
        .attribute(&"speed".parse().unwrap())
        .unwrap()
        .iter()
        .map(|instance| instance.meta().output.instance_id.as_ref().unwrap().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn a_later_range_of_a_last_n_request_keeps_the_last_n_instances_of_each_attribute() {
    assert_eq!(last_speeds(2, TemporalProperty::ObservedAt), "urn:ngsi-ld:i:2,urn:ngsi-ld:i:3");
    assert_eq!(last_speeds(1, TemporalProperty::ObservedAt), "urn:ngsi-ld:i:2");
    assert_eq!(last_speeds(3, TemporalProperty::ObservedAt), "urn:ngsi-ld:i:2,urn:ngsi-ld:i:3,urn:ngsi-ld:i:1");
}

#[test]
fn the_last_n_instances_are_counted_by_the_temporal_property_and_one_without_it_is_the_least_recent() {
    assert_eq!(last_speeds(2, TemporalProperty::CreatedAt), "urn:ngsi-ld:i:2,urn:ngsi-ld:i:1");
}
