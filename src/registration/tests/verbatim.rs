//! Registrations whose endpoint is written as the caller gave it.

use crate::{
    model::{Input, UrlDraft},
    registration::{CSourceRegistration, RegistrationFragment, tests::vehicle_speed},
};
use pretty_assertions::assert_eq;

#[test]
fn a_registration_endpoint_given_verbatim_is_written_without_a_trailing_slash() {
    let registration = CSourceRegistration::<Input>::builder(UrlDraft::verbatim("http://localhost:8080"))
        .information([vehicle_speed()])
        .build()
        .unwrap();
    assert_eq!(
        registration.render().to_string(),
        r#"{"type":"ContextSourceRegistration","information":[{"entities":[{"type":"Vehicle"}],"propertyNames":["speed"]}],"endpoint":"http://localhost:8080"}"#
    );
    assert_eq!(registration.endpoint().as_str(), "http://localhost:8080/");
}

#[test]
fn registrations_differing_only_in_the_spelling_of_the_endpoint_are_equal() {
    let verbatim = CSourceRegistration::<Input>::builder(UrlDraft::verbatim("http://localhost:8080"))
        .information([vehicle_speed()])
        .build()
        .unwrap();
    let canonical = CSourceRegistration::<Input>::builder("http://localhost:8080")
        .information([vehicle_speed()])
        .build()
        .unwrap();
    assert_eq!(verbatim, canonical);
    assert!(canonical.render().to_string().contains(r#""endpoint":"http://localhost:8080/""#));
}

#[test]
fn a_registration_fragment_endpoint_given_verbatim_is_written_as_given() {
    let fragment = RegistrationFragment::builder()
        .endpoint(UrlDraft::verbatim("http://localhost:8080"))
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"ContextSourceRegistration","endpoint":"http://localhost:8080"}"#
    );
    assert_eq!(fragment.endpoint().map(url::Url::as_str), Some("http://localhost:8080/"));
}
