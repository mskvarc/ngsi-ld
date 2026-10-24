//! One element of a `Forwarded` header: what one proxy passed on (RFC 7239 section 4).

use crate::http::headers::{
    ForwardedHost,
    ForwardedHostError,
    Node,
    NodeError,
    Scheme,
    SchemeError,
    field_list::{Unterminated, split},
    quoted_string::{MalformedParameter, parse_parameter, write_token_or_quoted},
};
use bon::bon;
use std::{fmt, str::FromStr};

/// A parameter of a `Forwarded` element, written in lowercase.
///
/// RFC 7239 section 5 defines the parameters; their names are case-insensitive.
///
/// The enum is exhaustive over the parameters this crate reads: RFC 7239 section 5 defines these four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum ForwardedParameter {
    /// `by`: the interface where the request came in to the proxy (section 5.1).
    By,
    /// `for`: the client that sent the request to the proxy (section 5.2).
    For,
    /// `host`: the `Host` the proxy received (section 5.3).
    Host,
    /// `proto`: the protocol the proxy received the request in (section 5.4).
    Proto,
}

/// Why a `Forwarded` element could not be read or built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ForwardedElementError {
    /// The element has no parameter.
    #[error("a forwarded element needs at least one parameter")]
    Empty,
    /// A pair is not `token=value`, or a quoted value is not closed.
    #[error("a forwarded pair is not a token, =, and a token or quoted string")]
    Syntax,
    /// A parameter is not `by`, `for`, `host` or `proto`.
    #[error("a forwarded parameter is not by, for, host or proto")]
    UnknownParameter,
    /// A parameter occurs twice.
    #[error("the {0} parameter occurs more than once")]
    Duplicate(ForwardedParameter),
    /// A `by` or `for` value is not a node.
    #[error("a forwarded node is malformed")]
    Node(#[from] NodeError),
    /// The `host` value is malformed.
    #[error("the forwarded host is malformed")]
    Host(#[from] ForwardedHostError),
    /// The `proto` value is not a URI scheme.
    #[error("the forwarded protocol is not a URI scheme")]
    Proto(#[from] SchemeError),
}

/// One element of a `Forwarded` header: what one proxy learnt of a request.
///
/// RFC 7239 section 4 defines it. It holds `by`, `for`, `host` and `proto` parameters, at least one of them.
///
/// ```
/// use ngsi_ld::http::headers::{ForwardedElement, ForwardedElementError};
/// use std::net::Ipv4Addr;
///
/// let element = ForwardedElement::builder()
///     .forwarded_for(Ipv4Addr::new(192, 0, 2, 60))
///     .proto("http".parse()?)
///     .by(Ipv4Addr::new(203, 0, 113, 43))
///     .build()?;
/// assert_eq!(
///     element.to_string(),
///     "by=203.0.113.43;for=192.0.2.60;proto=http"
/// );
/// assert!(matches!(
///     ForwardedElement::builder().build(),
///     Err(ForwardedElementError::Empty)
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForwardedElement {
    /// `by`.
    by: Option<Node>,
    /// `for`.
    forwarded_for: Option<Node>,
    /// `host`.
    host: Option<ForwardedHost>,
    /// `proto`.
    proto: Option<Scheme>,
}

#[bon]
impl ForwardedElement {
    /// An element of the parameters given.
    ///
    /// # Errors
    ///
    /// Returns [`ForwardedElementError::Empty`] when no parameter is given.
    #[builder]
    pub fn new(
        #[builder(into)] by: Option<Node>,
        #[builder(into)] forwarded_for: Option<Node>,
        #[builder(into)] host: Option<ForwardedHost>,
        proto: Option<Scheme>,
    ) -> Result<ForwardedElement, ForwardedElementError> {
        if by.is_none() && forwarded_for.is_none() && host.is_none() && proto.is_none() {
            return Err(ForwardedElementError::Empty);
        }
        Ok(ForwardedElement {
            by,
            forwarded_for,
            host,
            proto,
        })
    }

    /// `by`: the interface where the request came in to the proxy.
    #[must_use]
    pub const fn by(&self) -> Option<&Node> {
        self.by.as_ref()
    }

    /// `for`: the client that sent the request to the proxy.
    #[must_use]
    pub const fn forwarded_for(&self) -> Option<&Node> {
        self.forwarded_for.as_ref()
    }

    /// `host`: the `Host` the proxy received.
    #[must_use]
    pub const fn host(&self) -> Option<&ForwardedHost> {
        self.host.as_ref()
    }

    /// `proto`: the protocol the proxy received the request in.
    #[must_use]
    pub const fn proto(&self) -> Option<&Scheme> {
        self.proto.as_ref()
    }
}

// The parameters are optional, `;`-separated, and quoted when they are no token, which a derive cannot express.
impl fmt::Display for ForwardedElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pairs = [
            self.by.as_ref().map(|node| (ForwardedParameter::By, node.to_string())),
            self.forwarded_for.as_ref().map(|node| (ForwardedParameter::For, node.to_string())),
            self.host.as_ref().map(|host| (ForwardedParameter::Host, host.to_string())),
            self.proto.as_ref().map(|proto| (ForwardedParameter::Proto, proto.to_string())),
        ];
        for (index, (name, value)) in pairs.into_iter().flatten().enumerate() {
            if index > 0 {
                f.write_str(";")?;
            }
            write!(f, "{name}=")?;
            write_token_or_quoted(f, &value)?;
        }
        Ok(())
    }
}

/// Sets `slot` to `value`, refusing a parameter already set.
fn set_once<T>(slot: &mut Option<T>, name: ForwardedParameter, value: T) -> Result<(), ForwardedElementError> {
    if slot.replace(value).is_some() {
        return Err(ForwardedElementError::Duplicate(name));
    }
    Ok(())
}

// The pairs are `;`-separated and each parameter is named at most once, which a derive cannot express.
impl FromStr for ForwardedElement {
    type Err = ForwardedElementError;

    /// Reads `[ forwarded-pair ] *( ";" [ forwarded-pair ] )` (RFC 7239 section 4); blank pairs are skipped.
    fn from_str(text: &str) -> Result<ForwardedElement, ForwardedElementError> {
        let mut by: Option<Node> = None;
        let mut forwarded_for: Option<Node> = None;
        let mut host: Option<ForwardedHost> = None;
        let mut proto: Option<Scheme> = None;
        let pairs = split(text, b';').map_err(|Unterminated| ForwardedElementError::Syntax)?;
        for pair in pairs.into_iter().filter(|pair| !pair.is_empty()) {
            let (name, value) = parse_parameter(pair).map_err(|MalformedParameter| ForwardedElementError::Syntax)?;
            let parameter = name.parse::<ForwardedParameter>().map_err(|_| ForwardedElementError::UnknownParameter)?;
            match parameter {
                ForwardedParameter::By => set_once(&mut by, parameter, value.parse()?)?,
                ForwardedParameter::For => set_once(&mut forwarded_for, parameter, value.parse()?)?,
                ForwardedParameter::Host => set_once(&mut host, parameter, value.parse()?)?,
                ForwardedParameter::Proto => set_once(&mut proto, parameter, value.parse()?)?,
            }
        }
        ForwardedElement::builder()
            .maybe_by(by)
            .maybe_forwarded_for(forwarded_for)
            .maybe_host(host)
            .maybe_proto(proto)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{ForwardedElement, ForwardedElementError, ForwardedHost, ForwardedHostError, ForwardedParameter, Node, NodeError};
    use pretty_assertions::assert_eq;
    use std::net::{Ipv6Addr, SocketAddr};

    fn parse(text: &str) -> Result<String, ForwardedElementError> {
        text.parse::<ForwardedElement>().map(|element| element.to_string())
    }

    #[test]
    fn every_parameter_round_trips_and_a_value_that_is_no_token_is_quoted() {
        let element = ForwardedElement::builder()
            .forwarded_for(SocketAddr::from((Ipv6Addr::LOCALHOST, 4711)))
            .by(Node::unknown())
            .host("example.com:8080".parse::<ForwardedHost>().unwrap())
            .proto("https".parse().unwrap())
            .build()
            .unwrap();
        let text = element.to_string();
        assert_eq!(text, "by=unknown;for=\"[::1]:4711\";host=\"example.com:8080\";proto=https");
        assert_eq!(text.parse::<ForwardedElement>(), Ok(element));
    }

    #[test]
    fn pairs_are_read_in_any_order_case_and_quoting() {
        assert_eq!(parse("For=\"192.0.2.43\";PROTO=HTTP"), Ok("for=192.0.2.43;proto=http".to_owned()));
        assert_eq!(parse("for=_hidden;;by=_SEVKISEK"), Ok("by=_SEVKISEK;for=_hidden".to_owned()));
        assert_eq!(parse("host=example.com"), Ok("host=example.com".to_owned()));
    }

    #[test]
    fn a_malformed_element_is_rejected_with_its_reason() {
        assert_eq!(parse(""), Err(ForwardedElementError::Empty));
        assert_eq!(parse(";"), Err(ForwardedElementError::Empty));
        assert_eq!(parse("for"), Err(ForwardedElementError::Syntax));
        assert_eq!(parse("for=\"192.0.2.43"), Err(ForwardedElementError::Syntax));
        assert_eq!(parse("for=[::1]"), Err(ForwardedElementError::Syntax));
        assert_eq!(parse("for = 192.0.2.43"), Err(ForwardedElementError::Syntax));
        assert_eq!(parse("secret=x"), Err(ForwardedElementError::UnknownParameter));
        assert_eq!(
            parse("for=192.0.2.43;for=192.0.2.44"),
            Err(ForwardedElementError::Duplicate(ForwardedParameter::For))
        );
        assert_eq!(parse("by=example.com"), Err(ForwardedElementError::Node(NodeError::Name)));
        assert_eq!(parse("host=\"a:b\""), Err(ForwardedElementError::Host(ForwardedHostError::Port)));
        assert!(matches!(parse("proto=1http"), Err(ForwardedElementError::Proto(_))));
    }
}
