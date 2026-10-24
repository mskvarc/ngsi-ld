//! The `host` a `Forwarded` element passes on (RFC 7239 section 5.3).

use crate::http::headers::host_port::{canonical_host, parse_port, split_host_port};
use std::{fmt, str::FromStr};
use url::Host;

/// Why a string is not the `host` of a `Forwarded` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ForwardedHostError {
    /// The host is not a host name, an IPv4 address or a bracketed IPv6 address.
    #[error("the host is not a host name or an IP address")]
    Host(#[source] url::ParseError),
    /// An IPv6 address is not closed by `]`, or is followed by text other than a port.
    #[error("an IPv6 host must be closed by ] and followed by nothing but a port")]
    Ipv6Literal,
    /// The text after `:` is not a port.
    #[error("the port is not a decimal number from 0 to 65535")]
    Port,
}

/// The `Host` a client sent to a proxy, which the proxy passes on (RFC 7239 section 5.3): `uri-host [ ":" port ]`.
///
/// The host is held as a URL holds it, so it always renders as a header can carry it and reads back the same.
///
/// ```
/// use ngsi_ld::http::headers::ForwardedHost;
/// use url::Host;
///
/// let host: ForwardedHost = "Broker.Example.org:8443".parse()?;
/// assert_eq!(host.to_string(), "broker.example.org:8443");
/// assert_eq!(host.port(), Some(8443));
/// assert_eq!(
///     ForwardedHost::new(Host::Domain("caf\u{e9}.example".to_owned()))?.to_string(),
///     "xn--caf-dma.example"
/// );
/// assert!(ForwardedHost::new(Host::Domain("a b".to_owned())).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForwardedHost {
    /// `uri-host`.
    host: Host,
    /// `port`, when written.
    port: Option<u16>,
}

impl ForwardedHost {
    /// The host `host`, without a port, a domain read as a URL reads it: lowercased, with a Unicode label encoded in
    /// Punycode, and as an IPv4 address when it is one.
    ///
    /// # Errors
    ///
    /// Returns [`ForwardedHostError::Host`] when `host` is a domain no URL holds, such as one with a space.
    pub fn new(host: Host) -> Result<ForwardedHost, ForwardedHostError> {
        let host = canonical_host(host).map_err(ForwardedHostError::Host)?;
        Ok(ForwardedHost { host, port: None })
    }

    /// Adds `port`.
    #[must_use]
    pub fn with_port(self, port: u16) -> ForwardedHost {
        ForwardedHost { port: Some(port), ..self }
    }

    /// The host.
    #[must_use]
    pub const fn host(&self) -> &Host {
        &self.host
    }

    /// The port, when written.
    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }
}

impl TryFrom<Host> for ForwardedHost {
    type Error = ForwardedHostError;

    fn try_from(host: Host) -> Result<ForwardedHost, ForwardedHostError> {
        ForwardedHost::new(host)
    }
}

// The port is optional and introduced by `:`, which a derive cannot express.
impl fmt::Display for ForwardedHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.host.fmt(f)?;
        self.port.map_or(Ok(()), |port| write!(f, ":{port}"))
    }
}

// The port is optional and introduced by `:`, which a derive cannot express.
impl FromStr for ForwardedHost {
    type Err = ForwardedHostError;

    fn from_str(text: &str) -> Result<ForwardedHost, ForwardedHostError> {
        let (host, port) = split_host_port(text).ok_or(ForwardedHostError::Ipv6Literal)?;
        let host = ForwardedHost {
            host: Host::parse(host).map_err(ForwardedHostError::Host)?,
            port: None,
        };
        match port {
            Some(port) => Ok(host.with_port(parse_port(port).ok_or(ForwardedHostError::Port)?)),
            None => Ok(host),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{ForwardedHost, ForwardedHostError};
    use pretty_assertions::assert_eq;
    use std::net::Ipv4Addr;
    use url::Host;

    fn parse(text: &str) -> Result<String, ForwardedHostError> {
        text.parse::<ForwardedHost>().map(|host| host.to_string())
    }

    #[test]
    fn a_name_or_address_with_or_without_a_port_round_trips() {
        for text in ["example.com", "example.com:8080", "192.0.2.1:80", "[2001:db8::1]", "[2001:db8::1]:443"] {
            assert_eq!(parse(text), Ok(text.to_owned()), "{text}");
        }
        let host = ForwardedHost::try_from(Host::Ipv4(Ipv4Addr::LOCALHOST)).unwrap().with_port(1026);
        assert_eq!((host.host(), host.port()), (&Host::Ipv4(Ipv4Addr::LOCALHOST), Some(1026)));
    }

    #[test]
    fn a_constructed_host_is_held_as_a_url_holds_it() {
        let domain = |name: &str| Host::Domain(name.to_owned());
        let host = ForwardedHost::new(domain("Caf\u{e9}.Example")).unwrap().with_port(8080);
        assert_eq!(host.to_string(), "xn--caf-dma.example:8080");
        assert_eq!(host.to_string().parse::<ForwardedHost>(), Ok(host));
        for name in ["a b", "a/b", ""] {
            assert!(matches!(ForwardedHost::new(domain(name)), Err(ForwardedHostError::Host(_))), "{name}");
        }
    }

    #[test]
    fn a_malformed_host_or_port_is_rejected() {
        for text in ["", ":80", "exa mple.com", "[2001:db8:1]"] {
            assert!(matches!(text.parse::<ForwardedHost>(), Err(ForwardedHostError::Host(_))), "{text}");
        }
        for text in ["[2001:db8::1", "[::1]x"] {
            assert_eq!(parse(text), Err(ForwardedHostError::Ipv6Literal), "{text}");
        }
        for text in ["example.com:", "example.com:http", "example.com:1:2"] {
            assert_eq!(parse(text), Err(ForwardedHostError::Port), "{text}");
        }
    }
}
