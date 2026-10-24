//! The documents and helpers the context source registration tests share.

use crate::fixtures::endpoint;
use bytes::Bytes;
use http::Request;
use ngsi_ld::{
    http::{
        BodyError,
        ResponseError,
        operation::{Operation, QueryRegistrations},
    },
    model::{DateTime, DocumentError, Input},
    query::type_selection::TypeSelection,
    registration::{CSourceRegistration, Problem as RegistrationProblem, RegistrationFragment, RegistrationId},
    subscription::ReadErrorKind,
};
use std::str;

/// `R`.
pub(crate) const R: &str = "urn:ngsi-ld:ContextSourceRegistration:csr1";

/// The text of `vehicle_source`: a context source providing the `speed` of every vehicle.
pub(crate) const VEHICLE_SOURCE: &str = r#"{"type":"ContextSourceRegistration","information":[{"entities":[{"type":"Vehicle"}],"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026/"}"#;

/// `R` as a broker returns it.
pub(crate) const RETRIEVED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","type":"ContextSourceRegistration","#,
    r#""information":[{"entities":[{"type":"Vehicle"}],"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026/","#,
    r#""status":"ok","timesSent":2,"createdAt":"2024-04-01T10:00:00Z"}"#
);

/// `vehicle_source`.
pub(crate) fn vehicle_source() -> CSourceRegistration<Input> {
    CSourceRegistration::parse(VEHICLE_SOURCE).unwrap()
}

/// `postpone`: a fragment moving the expiry.
pub(crate) fn postpone() -> RegistrationFragment {
    RegistrationFragment::builder()
        .expires_at("2030-01-01T00:00:00Z".parse::<DateTime>().unwrap())
        .build()
        .unwrap()
}

/// Parses a registration id.
pub(crate) fn id(text: &str) -> RegistrationId {
    text.parse().unwrap()
}

/// The text of a request body.
pub(crate) fn body_text(request: &Request<Bytes>) -> &str {
    str::from_utf8(request.body()).unwrap()
}

/// A query on the type `Vehicle` with no other parameter.
pub(crate) fn vehicles() -> TypeSelection {
    "Vehicle".parse().unwrap()
}

/// The query string of the request `query` builds.
pub(crate) fn query_string(query: &QueryRegistrations<'_>) -> Option<String> {
    query.to_request(&endpoint()).unwrap().uri().query().map(str::to_owned)
}

/// The reader error inside a body error naming a registration.
pub(crate) fn registration_error(error: ResponseError) -> DocumentError<ReadErrorKind<RegistrationProblem>> {
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Registration(error) = *source else {
        panic!("expected a registration error, got {source:?}")
    };
    error
}
