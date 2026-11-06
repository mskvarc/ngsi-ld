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
        "https://example.org:8443/Ciudad/Par\u{ed}s",
    ] {
        assert_ne!(raw.parse::<Uri>().unwrap(), canonical, "{raw:?}");
    }
}

// RFC 3986 section 6.2.2: the same equivalences hold for an all-ASCII URL, which is written in its normal form.
#[test]
fn syntax_based_equivalent_ascii_urls_are_one_uri() {
    let canonical: Uri = "http://example.org/a/c/~b%2F".parse().unwrap();
    for raw in [
        "HTTP://Example.ORG/a/c/~b%2F",
        "http://example.org/a/c/%7eb%2f",
        "http://example.org/a/./b/../c/%7Eb%2F",
        "http://exa%6Dple.org/a/c/~b%2F",
    ] {
        let uri: Uri = raw.parse().unwrap();
        assert!(uri.as_url().is_some(), "{raw:?}");
        assert_eq!(uri, canonical, "{raw:?}");
        assert_eq!(uri.as_str(), "http://example.org/a/c/~b%2F", "{raw:?}");
        assert_eq!(hash_of(&uri), hash_of(&canonical), "{raw:?}");
    }
}

// RFC 3987 section 5.3.2.3: percent-encoding is aligned by mapping the IRI to a URI (section 3.1), so an escaped
// UTF-8 sequence and the character it encodes are one URI, each written as it was recognised.
#[test]
fn an_escaped_non_ascii_character_and_the_character_are_one_uri() {
    let spelled: Uri = "https://example.org/Ciudad/Par\u{ed}s".parse().unwrap();
    for raw in ["https://example.org/Ciudad/Par%C3%ADs", "https://example.org/Ciudad/Par%c3%ads"] {
        let escaped: Uri = raw.parse().unwrap();
        assert_eq!(escaped, spelled, "{raw:?}");
        assert_eq!(hash_of(&escaped), hash_of(&spelled), "{raw:?}");
        assert_eq!(escaped.as_str(), "https://example.org/Ciudad/Par%C3%ADs", "{raw:?}");
    }
    assert_eq!(spelled.as_str(), "https://example.org/Ciudad/Par\u{ed}s");
}

#[test]
fn a_non_ascii_host_and_its_idna_form_are_one_uri() {
    let spelled: Uri = "https://B\u{fc}cher.example/Par\u{ed}s".parse().unwrap();
    for raw in ["https://xn--bcher-kva.example/Par%C3%ADs", "https://b%C3%BCcher.example/Par%C3%ADs"] {
        let other: Uri = raw.parse().unwrap();
        assert_eq!(other, spelled, "{raw:?}");
        assert_eq!(hash_of(&other), hash_of(&spelled), "{raw:?}");
    }
}

// RFC 3986 section 6.2.3: the default port of the scheme does not distinguish two URLs.
#[test]
fn a_default_port_does_not_distinguish_two_uris() {
    assert_eq!(
        "https://example.org:443/a".parse::<Uri>().unwrap(),
        "https://example.org/a".parse::<Uri>().unwrap()
    );
    assert_eq!(
        "https://example.org:443/Par\u{ed}s".parse::<Uri>().unwrap(),
        "https://example.org/Par%C3%ADs".parse::<Uri>().unwrap()
    );
}

#[test]
fn an_iri_no_url_holds_is_compared_by_the_uri_it_maps_to() {
    let spelled: Uri = "HTTP://example.org:99999/Par\u{ed}s/%7e".parse().unwrap();
    assert!(spelled.as_url().is_none() && spelled.as_urn().is_none());
    assert_eq!(spelled.as_str(), "http://example.org:99999/Par\u{ed}s/~");
    let escaped: Uri = "http://example.org:99999/Par%c3%ads/~".parse().unwrap();
    assert_eq!(escaped, spelled);
    assert_eq!(hash_of(&escaped), hash_of(&spelled));
    assert_ne!("http://example.org:99998/Par\u{ed}s/~".parse::<Uri>().unwrap(), spelled);
}

#[test]
fn a_url_converts_to_the_form_its_text_parses_to() {
    let urn = Uri::try_from(Url::parse("urn:ngsi-ld:Vehicle:A4567").unwrap()).unwrap();
    assert!(urn.as_urn().is_some());
    assert_eq!(urn, "urn:ngsi-ld:Vehicle:A4567".parse::<Uri>().unwrap());
    let url = Uri::try_from(Url::parse("http://example.org/%7ea").unwrap()).unwrap();
    assert_eq!(url, "http://example.org/~a".parse::<Uri>().unwrap());
    assert_eq!(url.as_str(), "http://example.org/~a");
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
