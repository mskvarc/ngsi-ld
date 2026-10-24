//! One alternative service an `Alt-Svc` header advertises (RFC 7838 section 3).

use crate::{
    http::headers::{
        field_list::{Unterminated, split, trim_ows},
        host_port::{canonical_host, parse_port, split_host_port},
        quoted_string::{MalformedParameter, MalformedQuotedString, parameter_value, parse_parameter, write_quoted},
    },
    model::{Token, TokenError},
};
use bon::bon;
use std::{fmt, num::IntErrorKind, str::FromStr, time::Duration};
use url::Host;

/// The parameter giving how long an alternative stays fresh (RFC 7838 section 3.1).
const MAX_AGE: &str = "ma";

/// The parameter asking an alternative be kept across network changes (RFC 7838 section 3.1).
const PERSIST: &str = "persist";

/// The one value of `persist` RFC 7838 section 3.1 defines.
const PERSIST_VALUE: &str = "1";

/// The number of seconds a `delta-seconds` too large to represent is taken as: the greatest the representation
/// holds, which RFC 9111 section 1.2.2 allows in place of 2^31 and which keeps a larger value never shorter than a
/// smaller one.
const OVERFLOWING_SECONDS: u64 = u64::MAX;

/// Why a string is not the authority of an alternative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AltAuthorityError {
    /// The host is not a host name, an IPv4 address or a bracketed IPv6 address.
    #[error("the host is not a host name or an IP address")]
    Host(#[source] url::ParseError),
    /// An IPv6 address is not closed by `]`, or is followed by text other than a port.
    #[error("an IPv6 host must be closed by ] and followed by a port")]
    Ipv6Literal,
    /// The authority has no port.
    #[error("an alternative authority must name a port")]
    MissingPort,
    /// The text after `:` is not a port.
    #[error("the port is not a decimal number from 0 to 65535")]
    Port,
}

/// Where an alternative service listens (RFC 7838 section 3): `[ uri-host ] ":" port`, the host omitted when it is
/// the host of the origin.
///
/// The host is held as a URL holds it, so the authority always renders as a header can carry it and reads back the
/// same.
///
/// ```
/// use ngsi_ld::http::headers::AltAuthority;
/// use url::Host;
///
/// assert_eq!(AltAuthority::same_host(443).to_string(), ":443");
/// let authority: AltAuthority = "alt.example.org:8443".parse()?;
/// assert_eq!(authority.port(), 8443);
/// assert_eq!(
///     AltAuthority::try_new(Host::Domain("Alt.Example.org".to_owned()), 443)?.to_string(),
///     "alt.example.org:443"
/// );
/// assert!(AltAuthority::try_new(Host::Domain("a b".to_owned()), 443).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AltAuthority {
    /// `uri-host`, absent for the host of the origin.
    host: Option<Host>,
    /// `port`.
    port: u16,
}

impl AltAuthority {
    /// The authority `host:port`, a domain read as a URL reads it: lowercased, with a Unicode label encoded in
    /// Punycode, and as an IPv4 address when it is one.
    ///
    /// # Errors
    ///
    /// Returns [`AltAuthorityError::Host`] when `host` is a domain no URL holds, such as one with a space.
    pub fn try_new(host: Host, port: u16) -> Result<AltAuthority, AltAuthorityError> {
        let host = canonical_host(host).map_err(AltAuthorityError::Host)?;
        Ok(AltAuthority { host: Some(host), port })
    }

    /// The authority on `port` of the host of the origin.
    #[must_use]
    pub const fn same_host(port: u16) -> AltAuthority {
        AltAuthority { host: None, port }
    }

    /// The host, absent for the host of the origin.
    #[must_use]
    pub const fn host(&self) -> Option<&Host> {
        self.host.as_ref()
    }

    /// The port.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }
}

// The host is optional and the port always introduced by `:`, which a derive cannot express.
impl fmt::Display for AltAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(host) = &self.host {
            host.fmt(f)?;
        }
        write!(f, ":{}", self.port)
    }
}

// The host may be empty and the port is required after `:`, which a derive cannot express.
impl FromStr for AltAuthority {
    type Err = AltAuthorityError;

    fn from_str(text: &str) -> Result<AltAuthority, AltAuthorityError> {
        let (host, port) = split_host_port(text).ok_or(AltAuthorityError::Ipv6Literal)?;
        let port = parse_port(port.ok_or(AltAuthorityError::MissingPort)?).ok_or(AltAuthorityError::Port)?;
        if host.is_empty() {
            return Ok(AltAuthority::same_host(port));
        }
        let host = Host::parse(host).map_err(AltAuthorityError::Host)?;
        Ok(AltAuthority { host: Some(host), port })
    }
}

/// Whether a client keeps an alternative when its network changes (RFC 7838 section 3.1).
///
/// The enum is exhaustive: section 3.1 defines `persist=1` as the one alternative to the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Persistence {
    /// The client drops the alternative when its network changes: the default, written without `persist`.
    #[default]
    ClearedOnNetworkChange,
    /// `persist=1`: the client keeps the alternative across network changes.
    Persistent,
}

/// Why a string is not an `Alt-Svc` alternative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AlternativeError {
    /// The alternative is not `protocol-id="authority"` followed by `;`-separated `name=value` parameters.
    #[error("an alternative is a protocol, =, a quoted authority, and parameters")]
    Syntax,
    /// The protocol id is not a token.
    #[error("the protocol id is not an HTTP token")]
    Protocol(#[from] TokenError),
    /// The authority is malformed.
    #[error("the alternative authority is malformed")]
    Authority(#[from] AltAuthorityError),
    /// The `ma` parameter is not a number of seconds.
    #[error("the ma parameter is not a number of seconds")]
    MaxAge,
}

/// An alternative service (RFC 7838 section 3): the protocol, as its percent-encoded ALPN id such as `h3`, `h2`
/// or `http%2F1.1`, the authority it is reached at, how long the advertisement stays fresh (`ma`), and whether it
/// survives a network change (`persist`).
///
/// `ma` is a whole number of seconds; a fraction of a second given to the builder is dropped. Reading an alternative
/// ignores a parameter RFC 7838 does not define and a `persist` value other than `1`, as section 3 and 3.1 require.
/// RFC 7838 does not forbid a parameter to repeat: the first `ma` is kept and a later one ignored, and any
/// `persist=1` makes the alternative persistent. An `ma` too large to represent is taken as `u64::MAX` seconds, the
/// greatest integer the representation holds, as RFC 9111 section 1.2.2 allows of `delta-seconds`.
///
/// ```
/// use ngsi_ld::http::headers::{AltAuthority, Alternative, Persistence};
/// use std::time::Duration;
///
/// let alternative = Alternative::builder()
///     .protocol("h3".parse()?)
///     .authority(AltAuthority::same_host(443))
///     .max_age(Duration::from_secs(86_400))
///     .persistence(Persistence::Persistent)
///     .build();
/// assert_eq!(alternative.to_string(), "h3=\":443\"; ma=86400; persist=1");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alternative {
    /// `protocol-id`.
    protocol: Token,
    /// `alt-authority`.
    authority: AltAuthority,
    /// `ma`, in whole seconds.
    max_age: Option<Duration>,
    /// `persist`.
    persistence: Persistence,
}

#[bon]
impl Alternative {
    /// The alternative of `protocol` at `authority`.
    #[builder]
    pub fn new(protocol: Token, authority: AltAuthority, max_age: Option<Duration>, #[builder(default)] persistence: Persistence) -> Alternative {
        Alternative {
            protocol,
            authority,
            max_age: max_age.map(|max_age| Duration::from_secs(max_age.as_secs())),
            persistence,
        }
    }

    /// The percent-encoded ALPN protocol id.
    #[must_use]
    pub const fn protocol(&self) -> &Token {
        &self.protocol
    }

    /// Where the alternative is reached.
    #[must_use]
    pub const fn authority(&self) -> &AltAuthority {
        &self.authority
    }

    /// How long the advertisement stays fresh, when given; RFC 7838 section 3.1 makes the default 24 hours.
    #[must_use]
    pub const fn max_age(&self) -> Option<Duration> {
        self.max_age
    }

    /// Whether the alternative survives a network change.
    #[must_use]
    pub const fn persistence(&self) -> Persistence {
        self.persistence
    }
}

// The authority is quoted and the parameters optional, which a derive cannot express.
impl fmt::Display for Alternative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}=", self.protocol)?;
        write_quoted(f, &self.authority.to_string())?;
        if let Some(max_age) = self.max_age {
            write!(f, "; {MAX_AGE}={}", max_age.as_secs())?;
        }
        match self.persistence {
            Persistence::ClearedOnNetworkChange => Ok(()),
            Persistence::Persistent => write!(f, "; {PERSIST}={PERSIST_VALUE}"),
        }
    }
}

/// Reads `delta-seconds` (RFC 9111 section 1.2.2): decimal digits only, a value too large to represent taken as the
/// greatest one represented.
fn parse_seconds(text: &str) -> Result<Duration, AlternativeError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AlternativeError::MaxAge);
    }
    match text.parse::<u64>() {
        Ok(seconds) => Ok(Duration::from_secs(seconds)),
        Err(error) if *error.kind() == IntErrorKind::PosOverflow => Ok(Duration::from_secs(OVERFLOWING_SECONDS)),
        Err(_other) => Err(AlternativeError::MaxAge),
    }
}

// A quoted authority followed by optional `;`-separated parameters, which a derive cannot express.
impl FromStr for Alternative {
    type Err = AlternativeError;

    fn from_str(text: &str) -> Result<Alternative, AlternativeError> {
        let pieces = split(text, b';').map_err(|Unterminated| AlternativeError::Syntax)?;
        let (alternative, parameters) = pieces.split_first().ok_or(AlternativeError::Syntax)?;
        let (protocol, authority) = trim_ows(alternative).split_once('=').ok_or(AlternativeError::Syntax)?;
        if !authority.starts_with('"') {
            return Err(AlternativeError::Syntax);
        }
        let authority = parameter_value(authority).map_err(|MalformedQuotedString| AlternativeError::Syntax)?;
        let (mut max_age, mut persistence) = (None, None);
        for parameter in parameters {
            let (name, value) = parse_parameter(trim_ows(parameter)).map_err(|MalformedParameter| AlternativeError::Syntax)?;
            if name.eq_ignore_ascii_case(MAX_AGE) {
                if max_age.is_none() {
                    max_age = Some(parse_seconds(&value)?);
                }
            } else if name.eq_ignore_ascii_case(PERSIST) && value == PERSIST_VALUE {
                persistence = Some(Persistence::Persistent);
            }
        }
        Ok(Alternative::builder()
            .protocol(protocol.parse()?)
            .authority(authority.parse()?)
            .maybe_max_age(max_age)
            .maybe_persistence(persistence)
            .build())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{AltAuthority, AltAuthorityError, Alternative, AlternativeError, Persistence},
        model::TokenError,
    };
    use pretty_assertions::assert_eq;
    use std::{net::Ipv4Addr, time::Duration};
    use url::Host;

    fn parse(text: &str) -> Result<String, AlternativeError> {
        text.parse::<Alternative>().map(|alternative| alternative.to_string())
    }

    #[test]
    fn an_alternative_with_each_parameter_round_trips() {
        for text in [
            "h3=\":443\"",
            "h2=\"alt.example.com:8000\"; ma=3600",
            "http%2F1.1=\"192.0.2.1:80\"; persist=1",
            "h3=\"[2001:db8::1]:443\"; ma=0; persist=1",
        ] {
            assert_eq!(parse(text), Ok(text.to_owned()), "{text}");
        }
    }

    #[test]
    fn the_builder_drops_a_fraction_of_a_second_and_defaults_the_persistence() {
        let alternative = Alternative::builder()
            .protocol("h2".parse().unwrap())
            .authority(AltAuthority::try_new(Host::Ipv4(Ipv4Addr::LOCALHOST), 8443).unwrap())
            .max_age(Duration::from_millis(1500))
            .build();
        assert_eq!(alternative.max_age(), Some(Duration::from_secs(1)));
        assert_eq!(alternative.persistence(), Persistence::ClearedOnNetworkChange);
        assert_eq!(alternative.authority().host(), Some(&Host::Ipv4(Ipv4Addr::LOCALHOST)));
        assert_eq!(alternative.protocol().as_ref(), "h2");
        assert_eq!(alternative.to_string(), "h2=\"127.0.0.1:8443\"; ma=1");
    }

    #[test]
    fn unknown_parameters_and_other_persist_values_are_ignored() {
        assert_eq!(parse("h3=\":443\";ma=60;foo=\"bar\";persist=0"), Ok("h3=\":443\"; ma=60".to_owned()));
        assert_eq!(parse("h3=\":443\" ; MA=60 ; Persist=1"), Ok("h3=\":443\"; ma=60; persist=1".to_owned()));
    }

    #[test]
    fn a_malformed_alternative_is_rejected_with_its_reason() {
        assert_eq!(parse("h3"), Err(AlternativeError::Syntax));
        assert_eq!(parse("h3=:443"), Err(AlternativeError::Syntax));
        assert_eq!(parse("h3=\":443"), Err(AlternativeError::Syntax));
        assert_eq!(parse("h3=\":443\"; ma"), Err(AlternativeError::Syntax));
        assert_eq!(parse("h3=\":443\";"), Err(AlternativeError::Syntax));
        assert_eq!(parse("h 3=\":443\""), Err(AlternativeError::Protocol(TokenError::InvalidCharacter)));
        assert_eq!(
            parse("h3=\"alt.example.com\""),
            Err(AlternativeError::Authority(AltAuthorityError::MissingPort))
        );
        assert_eq!(parse("h3=\":http\""), Err(AlternativeError::Authority(AltAuthorityError::Port)));
        assert_eq!(parse("h3=\"[::1\""), Err(AlternativeError::Authority(AltAuthorityError::Ipv6Literal)));
        assert!(matches!(
            parse("h3=\"exa mple:1\""),
            Err(AlternativeError::Authority(AltAuthorityError::Host(_)))
        ));
        assert_eq!(parse("h3=\":443\"; ma=-1"), Err(AlternativeError::MaxAge));
    }

    #[test]
    fn a_repeated_parameter_keeps_its_first_occurrence() {
        assert_eq!(parse("h3=\":443\"; ma=1; ma=2"), Ok("h3=\":443\"; ma=1".to_owned()));
        assert_eq!(parse("h3=\":443\"; ma=1; ma=x"), Ok("h3=\":443\"; ma=1".to_owned()));
        assert_eq!(parse("h3=\":443\"; persist=1; persist=1"), Ok("h3=\":443\"; persist=1".to_owned()));
        assert_eq!(parse("h3=\":443\"; persist=0; persist=1"), Ok("h3=\":443\"; persist=1".to_owned()));
    }

    #[test]
    fn an_overflowing_max_age_is_taken_as_the_greatest_one_represented() {
        let max_age = |text: &str| text.parse::<Alternative>().map(|alternative| alternative.max_age());
        assert_eq!(max_age("h3=\":443\"; ma=18446744073709551614"), Ok(Some(Duration::from_secs(u64::MAX - 1))));
        assert_eq!(max_age("h3=\":443\"; ma=18446744073709551615"), Ok(Some(Duration::from_secs(u64::MAX))));
        assert_eq!(max_age("h3=\":443\"; ma=18446744073709551616"), Ok(Some(Duration::from_secs(u64::MAX))));
        assert_eq!(max_age("h3=\":443\"; ma=99999999999999999999999999"), Ok(Some(Duration::from_secs(u64::MAX))));
    }

    #[test]
    fn a_larger_max_age_is_never_read_as_a_shorter_one() {
        let seconds = |text: &str| {
            format!("h3=\":443\"; ma={text}")
                .parse::<Alternative>()
                .ok()
                .and_then(|alternative| alternative.max_age())
        };
        let read = ["2147483648", "18446744073709551615", "18446744073709551616", "1".repeat(40).as_str()].map(seconds);
        assert!(read.windows(2).all(|pair| pair[0] <= pair[1]), "{read:?}");
    }

    #[test]
    fn a_constructed_authority_holds_its_host_as_a_url_does() {
        let domain = |name: &str| Host::Domain(name.to_owned());
        let authority = AltAuthority::try_new(domain("caf\u{e9}.example"), 443).unwrap();
        assert_eq!(authority.to_string(), "xn--caf-dma.example:443");
        assert_eq!(authority.to_string().parse::<AltAuthority>(), Ok(authority));
        assert!(matches!(AltAuthority::try_new(domain("a b"), 443), Err(AltAuthorityError::Host(_))));
    }
}
