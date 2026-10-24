use crate::model::{Uri, UriError};
use iri_rs::iri;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;
use url::Url;

#[test]
fn a_urn_is_held_as_a_urn() {
    let uri: Uri = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    assert!(uri.as_urn().is_some());
    assert_eq!(uri.to_string(), "urn:ngsi-ld:Vehicle:A4567");
}

#[test]
fn an_ascii_url_is_held_as_a_url() {
    let uri: Uri = "https://example.org/vehicles/1".parse().unwrap();
    assert!(uri.as_url().is_some());
    assert_eq!(uri.as_str(), "https://example.org/vehicles/1");
}

#[test]
fn the_urn_prefix_case_does_not_distinguish_two_uris() {
    let upper: Uri = "URN:NGSI-LD:Vehicle:A4567".parse().unwrap();
    let lower: Uri = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    assert_eq!(upper, lower);
    let other: Uri = "urn:ngsi-ld:vehicle:A4567".parse().unwrap();
    assert_ne!(other, lower);
}

#[test]
fn a_non_ascii_iri_is_kept_verbatim() {
    let raw = "https://example.org/Ciudad/Par\u{ed}s";
    let uri: Uri = raw.parse().unwrap();
    assert!(uri.as_url().is_none() && uri.as_urn().is_none());
    assert_eq!(uri.as_str(), raw);
}

#[test]
fn whitespace_and_characters_outside_the_grammar_are_rejected() {
    for raw in [
        "Vehicle A4567",
        "urn:ngsi-ld:Vehicle:has space",
        " urn:ngsi-ld:Vehicle:1",
        "urn:ngsi-ld:Vehicle:a<b",
        "https://example.org/a%2",
    ] {
        assert_eq!(raw.parse::<Uri>(), Err(UriError), "{raw:?}");
    }
}

#[test]
fn a_compile_time_literal_gets_the_same_form_as_a_parsed_string() {
    assert_eq!(
        Uri::from(iri!("urn:ngsi-ld:Vehicle:A4567")),
        "urn:ngsi-ld:Vehicle:A4567".parse::<Uri>().unwrap()
    );
    assert_eq!(Uri::from(iri!("https://example.org/a")), "https://example.org/a".parse::<Uri>().unwrap());
}

#[test]
fn a_url_with_a_literal_space_in_an_opaque_path_is_rejected() {
    let url: Url = "urn:ngsi-ld:Vehicle:has space".parse().unwrap();
    assert_eq!(Uri::try_from(url), Err(UriError));
}

#[test]
fn a_uri_round_trips_through_serde() {
    let value = Json::from("urn:ngsi-ld:Vehicle:A4567");
    let uri: Uri = jstrict::from_value(value.clone()).unwrap();
    assert_eq!(jstrict::to_value(&uri).unwrap(), value);
}
