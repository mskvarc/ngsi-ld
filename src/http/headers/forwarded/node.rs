//! The node a `Forwarded` element names in `for` and `by` (RFC 7239 section 6).

use crate::http::headers::host_port::{parse_port, split_host_port};
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    str::FromStr,
};

/// The node name standing for a node a proxy does not know (RFC 7239 section 6.2).
const UNKNOWN: &str = "unknown";

/// Why a string is not an obfuscated identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ObfuscatedIdentifierError {
    /// The text is not `_` followed by letters, digits, `.`, `_` and `-`.
    #[error("an obfuscated identifier is _ followed by letters, digits, ., _ and -")]
    Syntax,
}

/// Checks an obfuscated identifier: `"_" 1*( ALPHA / DIGIT / "." / "_" / "-" )` (RFC 7239 section 6.3).
fn validate_obfuscated(text: &str) -> Result<(), ObfuscatedIdentifierError> {
    let rest = text.strip_prefix('_').ok_or(ObfuscatedIdentifierError::Syntax)?;
    if !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')) {
        Ok(())
    } else {
        Err(ObfuscatedIdentifierError::Syntax)
    }
}

/// An obfuscated node name or port (RFC 7239 section 6.3), such as `_hidden` or `_SEVKISEK`: an identifier a
/// proxy uses instead of an address it does not disclose.
///
/// ```
/// use ngsi_ld::http::headers::{ObfuscatedIdentifier, ObfuscatedIdentifierError};
///
/// let identifier: ObfuscatedIdentifier = "_gazonk".parse()?;
/// assert_eq!(identifier.to_string(), "_gazonk");
/// assert_eq!(
///     "gazonk".parse::<ObfuscatedIdentifier>(),
///     Err(ObfuscatedIdentifierError::Syntax)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_obfuscated, error = ObfuscatedIdentifierError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ObfuscatedIdentifier(String);

/// The name of a node (RFC 7239 section 6): an IP address, `unknown`, or an obfuscated identifier.
///
/// The enum is exhaustive: RFC 7239 section 6 defines `nodename` as exactly these forms.
///
/// ```
/// use ngsi_ld::http::headers::NodeName;
/// use std::net::Ipv4Addr;
///
/// assert_eq!(
///     NodeName::from(Ipv4Addr::new(192, 0, 2, 60)).to_string(),
///     "192.0.2.60"
/// );
/// assert_eq!(NodeName::Unknown.to_string(), "unknown");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum NodeName {
    /// An IPv4 address.
    #[display("{_0}")]
    Ipv4(Ipv4Addr),
    /// An IPv6 address, written in brackets.
    #[display("[{_0}]")]
    Ipv6(Ipv6Addr),
    /// `unknown`: the proxy does not know the node, but knows a proxy stands there (section 6.2).
    #[display("unknown")]
    #[from(skip)]
    Unknown,
    /// An obfuscated identifier (section 6.3).
    #[display("{_0}")]
    Obfuscated(ObfuscatedIdentifier),
}

impl From<IpAddr> for NodeName {
    fn from(address: IpAddr) -> NodeName {
        match address {
            IpAddr::V4(address) => NodeName::Ipv4(address),
            IpAddr::V6(address) => NodeName::Ipv6(address),
        }
    }
}

/// The port of a node (RFC 7239 section 6): a TCP port or an obfuscated identifier.
///
/// The enum is exhaustive: RFC 7239 section 6 defines `node-port` as exactly these forms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum NodePort {
    /// A TCP port.
    #[display("{_0}")]
    Port(u16),
    /// An obfuscated identifier (section 6.3).
    #[display("{_0}")]
    Obfuscated(ObfuscatedIdentifier),
}

/// Why a string is not a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum NodeError {
    /// The name is not an IP address, `unknown` or an obfuscated identifier.
    #[error("the node name is not an IP address, unknown or an obfuscated identifier")]
    Name,
    /// The port is not a TCP port or an obfuscated identifier.
    #[error("the node port is not a TCP port or an obfuscated identifier")]
    Port,
}

/// A node of a `Forwarded` element (RFC 7239 section 6): `nodename [ ":" node-port ]`, the client a request came
/// from (`for`) or the proxy interface that received it (`by`).
///
/// ```
/// use ngsi_ld::http::headers::Node;
/// use std::net::{Ipv6Addr, SocketAddr};
///
/// let node: Node = "[2001:db8:cafe::17]:4711".parse()?;
/// assert_eq!(
///     node,
///     Node::from(SocketAddr::from((
///         Ipv6Addr::new(0x2001, 0xdb8, 0xcafe, 0, 0, 0, 0, 0x17),
///         4711
///     )))
/// );
/// assert_eq!(Node::unknown().to_string(), "unknown");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    /// `nodename`.
    name: NodeName,
    /// `node-port`, when written.
    port: Option<NodePort>,
}

impl Node {
    /// The node `name`, without a port.
    #[must_use]
    pub const fn new(name: NodeName) -> Node {
        Node { name, port: None }
    }

    /// The node a proxy does not know (RFC 7239 section 6.2).
    #[must_use]
    pub const fn unknown() -> Node {
        Node::new(NodeName::Unknown)
    }

    /// Adds `port`.
    #[must_use]
    pub fn with_port(self, port: impl Into<NodePort>) -> Node {
        Node {
            port: Some(port.into()),
            ..self
        }
    }

    /// The node name.
    #[must_use]
    pub const fn name(&self) -> &NodeName {
        &self.name
    }

    /// The node port, when written.
    #[must_use]
    pub const fn port(&self) -> Option<&NodePort> {
        self.port.as_ref()
    }
}

impl From<NodeName> for Node {
    fn from(name: NodeName) -> Node {
        Node::new(name)
    }
}

impl From<IpAddr> for Node {
    fn from(address: IpAddr) -> Node {
        Node::new(address.into())
    }
}

impl From<Ipv4Addr> for Node {
    fn from(address: Ipv4Addr) -> Node {
        Node::new(address.into())
    }
}

impl From<Ipv6Addr> for Node {
    fn from(address: Ipv6Addr) -> Node {
        Node::new(address.into())
    }
}

impl From<SocketAddr> for Node {
    fn from(address: SocketAddr) -> Node {
        Node::from(address.ip()).with_port(address.port())
    }
}

// The port is optional and introduced by `:`, which a derive cannot express.
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)?;
        self.port.as_ref().map_or(Ok(()), |port| write!(f, ":{port}"))
    }
}

/// Reads a node name.
fn parse_name(text: &str) -> Option<NodeName> {
    if let Some(address) = text.strip_prefix('[') {
        address.strip_suffix(']')?.parse().ok().map(NodeName::Ipv6)
    } else if text.eq_ignore_ascii_case(UNKNOWN) {
        Some(NodeName::Unknown)
    } else if text.starts_with('_') {
        text.parse().ok().map(NodeName::Obfuscated)
    } else {
        text.parse().ok().map(NodeName::Ipv4)
    }
}

/// Reads a node port.
fn parse_node_port(text: &str) -> Option<NodePort> {
    if text.starts_with('_') {
        text.parse().ok().map(NodePort::Obfuscated)
    } else {
        parse_port(text).map(NodePort::Port)
    }
}

// The port is optional and introduced by `:`, which a derive cannot express.
impl FromStr for Node {
    type Err = NodeError;

    fn from_str(text: &str) -> Result<Node, NodeError> {
        let (name, port) = split_host_port(text).ok_or(NodeError::Name)?;
        let node = Node::new(parse_name(name).ok_or(NodeError::Name)?);
        match port {
            Some(port) => Ok(node.with_port(parse_node_port(port).ok_or(NodeError::Port)?)),
            None => Ok(node),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{Node, NodeError, NodeName, NodePort, ObfuscatedIdentifier, ObfuscatedIdentifierError};
    use pretty_assertions::assert_eq;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    fn parse(text: &str) -> Result<String, NodeError> {
        text.parse::<Node>().map(|node| node.to_string())
    }

    #[test]
    fn every_form_of_node_round_trips() {
        for text in [
            "192.0.2.43",
            "192.0.2.43:47011",
            "[2001:db8:cafe::17]",
            "[2001:db8:cafe::17]:4711",
            "unknown",
            "unknown:_p",
            "_hidden",
            "_SEVKISEK:_x.y-z",
        ] {
            assert_eq!(parse(text), Ok(text.to_owned()), "{text}");
        }
        assert_eq!(parse("UNKNOWN"), Ok("unknown".to_owned()));
    }

    #[test]
    fn addresses_convert_into_nodes() {
        let v4 = Ipv4Addr::new(192, 0, 2, 60);
        assert_eq!(Node::from(v4).name(), &NodeName::Ipv4(v4));
        assert_eq!(Node::from(IpAddr::V6(Ipv6Addr::LOCALHOST)).to_string(), "[::1]");
        let node = Node::from(SocketAddr::from((v4, 8080)));
        assert_eq!(node.port(), Some(&NodePort::Port(8080)));
        assert_eq!(node.to_string(), "192.0.2.60:8080");
        assert_eq!(Node::from(NodeName::Unknown), Node::unknown());
        let obfuscated: ObfuscatedIdentifier = "_x".parse().unwrap();
        assert_eq!(Node::unknown().with_port(obfuscated).to_string(), "unknown:_x");
    }

    #[test]
    fn a_malformed_name_or_port_is_rejected() {
        for text in ["", "example.com", "2001:db8::1", "[2001:db8::1", "[192.0.2.1]", "_", "_a b", "300.1.1.1"] {
            assert_eq!(parse(text), Err(NodeError::Name), "{text}");
        }
        for text in ["192.0.2.1:", "192.0.2.1:65536", "192.0.2.1:_", "unknown:x"] {
            assert_eq!(parse(text), Err(NodeError::Port), "{text}");
        }
    }

    #[test]
    fn an_obfuscated_identifier_follows_its_grammar() {
        assert_eq!(
            "_a.b_c-1".parse::<ObfuscatedIdentifier>().map(|identifier| identifier.to_string()),
            Ok("_a.b_c-1".to_owned())
        );
        for text in ["", "_", "a", "_a:b", "_a+b"] {
            assert_eq!(text.parse::<ObfuscatedIdentifier>(), Err(ObfuscatedIdentifierError::Syntax), "{text}");
        }
    }
}
