//! The documents and helpers the temporal operation tests share.

use crate::fixtures::endpoint;
use bytes::Bytes;
use http::Request;
use iri_rs::iri;
use ngsi_ld::{
    http::operation::Operation,
    model::{AttributeName, DateTime, EntityId, Input, InstanceId, Property, evolution::TemporalEntity},
    query::temporal_query::TemporalQuery,
};
use std::str;

/// `t1`.
pub(crate) const T1: &str = "2018-08-01T12:03:00Z";
/// `t2`.
pub(crate) const T2: &str = "2018-08-01T12:05:00Z";
/// `t3`.
pub(crate) const T3: &str = "2018-08-01T12:07:00Z";

/// The parameters of `window`, encoded.
pub(crate) const WINDOW: &str = "timerel=between&timeAt=2018-08-01T12%3A03%3A00Z&endTimeAt=2018-08-01T12%3A07%3A00Z";

/// The path of `B` below the API root.
pub(crate) const B_PATH: &str = "/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211";

/// The normalized text of `evolution`.
pub(crate) const EVOLUTION: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":["#,
    r#"{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"},"#,
    r#"{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z"}]}"#
);

/// `B`.
pub(crate) fn b() -> EntityId {
    iri!("urn:ngsi-ld:Vehicle:B9211").into()
}

/// A time.
pub(crate) fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// `speed`.
pub(crate) fn speed() -> AttributeName {
    "speed".parse().unwrap()
}

/// `I`.
pub(crate) fn instance_id() -> InstanceId {
    "urn:ngsi-ld:attribute:instance:1".parse().unwrap()
}

/// `window`: `between(t1, t3)`.
pub(crate) fn window() -> TemporalQuery {
    TemporalQuery::between(time(T1), time(T3))
}

/// `evolution`.
pub(crate) fn evolution() -> TemporalEntity<Input> {
    TemporalEntity::builder()
        .id(b())
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(time(T1)))
        .property("speed", Property::builder(80).observed_at(time(T2)))
        .build()
        .unwrap()
}

/// The request `operation` builds on the default endpoint.
pub(crate) fn request(operation: &impl Operation) -> Request<Bytes> {
    operation.to_request(&endpoint()).unwrap()
}

/// The query string of the request `operation` builds.
pub(crate) fn query_string(operation: &impl Operation) -> Option<String> {
    request(operation).uri().query().map(str::to_owned)
}

/// The text of a request body.
pub(crate) fn body_text(request: &Request<Bytes>) -> &str {
    str::from_utf8(request.body()).unwrap()
}
