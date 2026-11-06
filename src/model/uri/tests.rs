use crate::model::{Uri, UriError};
use iri_rs::iri;
use jstrict::Value as Json;
use pretty_assertions::{assert_eq, assert_ne};
use std::hash::{DefaultHasher, Hash, Hasher};
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
fn a_non_ascii_iri_is_held_as_an_iri() {
    let raw = "https://example.org/Ciudad/Par\u{ed}s";
    let uri: Uri = raw.parse().unwrap();
    assert!(uri.as_url().is_none() && uri.as_urn().is_none());
    assert_eq!(uri.as_str(), raw);
}

// RFC 3986 section 6.2.2 (RFC 3987 section 5.3.2): scheme and host case, the case of percent-encoding hex digits,
// percent-encoded unreserved characters and dot segments do not distinguish two IRIs.
#[test]
fn syntax_based_equivalent_non_ascii_iris_are_one_uri() {
    let canonical: Uri = "https://example.org/Ciudad/Par\u{ed}s/~%2F".parse().unwrap();
    for raw in [
        "HTTPS://Example.ORG/Ciudad/Par\u{ed}s/~%2F",
        "https://example.org/Ciudad/Par\u{ed}s/%7e%2f",
        "https://example.org/Pueblo/../Ciudad/./Par\u{ed}s/~%2F",
    ] {
        let uri: Uri = raw.parse().unwrap();
        assert_eq!(uri, canonical, "{raw:?}");
        assert_eq!(uri.as_str(), canonical.as_str(), "{raw:?}");
        assert_eq!(hash_of(&uri), hash_of(&canonical), "{raw:?}");
    }
}

#[test]
fn non_ascii_iris_differing_beyond_syntax_based_normalisation_are_distinct() {
    let canonical: Uri = "https://example.org/Ciudad/Par\u{ed}s".parse().unwrap();
    for raw in [
        "https://example.org/ciudad/Par\u{ed}s",
        "https://example.org/Ciudad/Par\u{ed}s/",
        "https://example.org:443/Ciudad/Par\u{ed}s",
    ] {
        assert_ne!(raw.parse::<Uri>().unwrap(), canonical, "{raw:?}");
    }
}

#[test]
fn a_verbatim_non_ascii_iri_keeps_its_spelling_and_equals_the_normal_form() {
    let raw = "HTTPS://Example.ORG/Ciudad/Par\u{ed}s/%7e";
    let verbatim = Uri::verbatim(raw).unwrap();
    assert_eq!(verbatim.as_str(), raw);
    let parsed: Uri = raw.parse().unwrap();
    assert_eq!(parsed.as_str(), "https://example.org/Ciudad/Par\u{ed}s/~");
    assert_eq!(verbatim, parsed);
    assert_eq!(hash_of(&verbatim), hash_of(&parsed));
}

fn hash_of(uri: &Uri) -> u64 {
    let mut hasher = DefaultHasher::new();
    uri.hash(&mut hasher);
    hasher.finish()
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
