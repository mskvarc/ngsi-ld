//! A host followed by an optional port, as headers write an authority (RFC 3986 section 3.2).

use url::Host;

/// `host` as a URL holds it: a domain is read again by the URL host parser, which lowercases it, encodes a Unicode
/// label in Punycode, reads a domain that is an IPv4 address as that address, and refuses what no host holds; an IP
/// address is kept.
///
/// A [`Host`] built directly can hold a domain the parser would never produce, such as `café` or `a b`, which a
/// header cannot carry or would not read back as the same host; this gives the host a header writes and reads.
pub(crate) fn canonical_host(host: Host) -> Result<Host, url::ParseError> {
    match host {
        Host::Domain(domain) => Host::parse(&domain),
        address @ (Host::Ipv4(_) | Host::Ipv6(_)) => Ok(address),
    }
}

/// Reads a `port` (RFC 3986 section 3.2.3): decimal digits only, no sign, within the range of a TCP port.
pub(crate) fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Splits `host [ ":" port ]` into the host and the port text, an IPv6 literal kept in its brackets.
///
/// `None` when text follows the closing bracket of an IPv6 literal without a `:`.
pub(crate) fn split_host_port(text: &str) -> Option<(&str, Option<&str>)> {
    if text.starts_with('[') {
        let close = text.find(']')?;
        let (host, rest) = text.split_at(close + 1);
        return match rest.strip_prefix(':') {
            Some(port) => Some((host, Some(port))),
            None => rest.is_empty().then_some((host, None)),
        };
    }
    Some(text.split_once(':').map_or((text, None), |(host, port)| (host, Some(port))))
}

#[cfg(test)]
mod tests {
    use crate::http::headers::host_port::{canonical_host, parse_port, split_host_port};
    use pretty_assertions::assert_eq;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use url::Host;

    #[test]
    fn a_domain_is_read_as_a_url_reads_it_and_an_address_is_kept() {
        let domain = |name: &str| Host::Domain(name.to_owned());
        assert_eq!(canonical_host(domain("example.org")), Ok(domain("example.org")));
        assert_eq!(canonical_host(domain("Broker.Example.org")), Ok(domain("broker.example.org")));
        assert_eq!(canonical_host(domain("caf\u{e9}.example")), Ok(domain("xn--caf-dma.example")));
        assert_eq!(canonical_host(domain("192.0.2.1")), Ok(Host::Ipv4(Ipv4Addr::new(192, 0, 2, 1))));
        assert_eq!(canonical_host(Host::Ipv6(Ipv6Addr::LOCALHOST)), Ok(Host::Ipv6(Ipv6Addr::LOCALHOST)));
        for name in ["a b", "", "a/b", "a\nb"] {
            assert!(canonical_host(domain(name)).is_err(), "{name}");
        }
    }

    #[test]
    fn a_port_is_decimal_digits_within_range() {
        assert_eq!(parse_port("443"), Some(443));
        assert_eq!(parse_port("0080"), Some(80));
        for text in ["", "+80", "-1", "65536", "8o", " 80"] {
            assert_eq!(parse_port(text), None, "{text}");
        }
    }

    #[test]
    fn a_host_splits_from_its_port_and_an_ipv6_literal_keeps_its_brackets() {
        assert_eq!(split_host_port("example.org"), Some(("example.org", None)));
        assert_eq!(split_host_port("example.org:8080"), Some(("example.org", Some("8080"))));
        assert_eq!(split_host_port(":443"), Some(("", Some("443"))));
        assert_eq!(split_host_port("[2001:db8::1]"), Some(("[2001:db8::1]", None)));
        assert_eq!(split_host_port("[2001:db8::1]:4711"), Some(("[2001:db8::1]", Some("4711"))));
        assert_eq!(split_host_port("[2001:db8::1]x"), None);
        assert_eq!(split_host_port("[2001:db8::1"), None);
    }
}
