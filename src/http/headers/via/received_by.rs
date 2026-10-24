//! The `received-by` of a `Via` element (RFC 9110 section 7.6.3).

use crate::{
    http::headers::host_port::parse_port,
    model::{ContextSourceAlias, ContextSourceAliasError},
};
use std::{fmt, str::FromStr};

/// Why a string is not a `received-by`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ReceivedByError {
    /// The pseudonym is not an HTTP token.
    #[error("the pseudonym is not an HTTP token")]
    Pseudonym(#[source] ContextSourceAliasError),
    /// The text after `:` is not a port.
    #[error("the port is not a decimal number from 0 to 65535")]
    Port,
}

/// Who received a message on a `Via` hop (RFC 9110 section 7.6.3): `pseudonym [ ":" port ]`, the host name and port
/// of the recipient or a pseudonym standing for it.
///
/// The pseudonym is an HTTP token, the same grammar as the `contextSourceAlias` a context broker writes as its
/// pseudonym when it forwards a distributed operation (NGSI-LD clause 6.3.18, Table 5.2.40-1), so it is held as a
/// [`ContextSourceAlias`]; a host name or an IPv4 address is a token too. An IPv6 literal is no token, and RFC 9110
/// does not allow one here.
///
/// ```
/// use ngsi_ld::http::headers::ReceivedBy;
///
/// let received_by: ReceivedBy = "broker-a:1026".parse()?;
/// assert_eq!(received_by.pseudonym().as_ref(), "broker-a");
/// assert_eq!(received_by.port(), Some(1026));
/// assert_eq!(received_by.to_string(), "broker-a:1026");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceivedBy {
    /// `pseudonym`.
    pseudonym: ContextSourceAlias,
    /// `port`, when written.
    port: Option<u16>,
}

impl ReceivedBy {
    /// The recipient `pseudonym`, without a port.
    #[must_use]
    pub const fn new(pseudonym: ContextSourceAlias) -> ReceivedBy {
        ReceivedBy { pseudonym, port: None }
    }

    /// Adds the `port` the recipient received the message on.
    #[must_use]
    pub fn with_port(self, port: u16) -> ReceivedBy {
        ReceivedBy { port: Some(port), ..self }
    }

    /// The pseudonym.
    #[must_use]
    pub const fn pseudonym(&self) -> &ContextSourceAlias {
        &self.pseudonym
    }

    /// The port, when written.
    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }
}

impl From<ContextSourceAlias> for ReceivedBy {
    fn from(pseudonym: ContextSourceAlias) -> ReceivedBy {
        ReceivedBy::new(pseudonym)
    }
}

// The port is optional and introduced by `:`, which a derive cannot express.
impl fmt::Display for ReceivedBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.pseudonym.fmt(f)?;
        self.port.map_or(Ok(()), |port| write!(f, ":{port}"))
    }
}

// The port is optional and introduced by `:`, which a derive cannot express.
impl FromStr for ReceivedBy {
    type Err = ReceivedByError;

    fn from_str(text: &str) -> Result<ReceivedBy, ReceivedByError> {
        let (pseudonym, port) = text.split_once(':').map_or((text, None), |(pseudonym, port)| (pseudonym, Some(port)));
        let received_by = ReceivedBy::new(pseudonym.parse().map_err(ReceivedByError::Pseudonym)?);
        match port {
            Some(port) => Ok(received_by.with_port(parse_port(port).ok_or(ReceivedByError::Port)?)),
            None => Ok(received_by),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{ReceivedBy, ReceivedByError},
        model::{ContextSourceAlias, ContextSourceAliasError},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn a_pseudonym_with_or_without_a_port_round_trips() {
        for text in ["broker-a", "proxy.example.org:8080", "192.0.2.1:80"] {
            assert_eq!(
                text.parse::<ReceivedBy>().map(|received_by| received_by.to_string()),
                Ok(text.to_owned()),
                "{text}"
            );
        }
    }

    #[test]
    fn an_alias_converts_without_a_port() {
        let alias: ContextSourceAlias = "broker-b".parse().unwrap();
        let received_by = ReceivedBy::from(alias.clone());
        assert_eq!((received_by.pseudonym(), received_by.port()), (&alias, None));
        assert_eq!(received_by.with_port(1026).port(), Some(1026));
    }

    #[test]
    fn a_pseudonym_that_is_no_token_or_a_bad_port_is_rejected() {
        assert_eq!("".parse::<ReceivedBy>(), Err(ReceivedByError::Pseudonym(ContextSourceAliasError::Empty)));
        assert_eq!(
            "[::1]".parse::<ReceivedBy>(),
            Err(ReceivedByError::Pseudonym(ContextSourceAliasError::InvalidCharacter))
        );
        assert_eq!("proxy:".parse::<ReceivedBy>(), Err(ReceivedByError::Port));
        assert_eq!("proxy:99999".parse::<ReceivedBy>(), Err(ReceivedByError::Port));
        assert_eq!("proxy:8080:1".parse::<ReceivedBy>(), Err(ReceivedByError::Port));
        assert_eq!("urn:ngsi-ld".parse::<ReceivedBy>(), Err(ReceivedByError::Port));
    }
}
