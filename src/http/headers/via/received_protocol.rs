//! The `received-protocol` of a `Via` element (RFC 9110 section 7.6.3).

use crate::model::{Token, TokenError};
use std::{fmt, str::FromStr};

/// Why a string is not a `received-protocol`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ReceivedProtocolError {
    /// The protocol name before the `/` is not a token.
    #[error("the protocol name is not an HTTP token")]
    Name(#[source] TokenError),
    /// The protocol version is not a token.
    #[error("the protocol version is not an HTTP token")]
    Version(#[source] TokenError),
}

/// The protocol a `Via` hop received the message in.
///
/// RFC 9110 section 7.6.3 writes it `[ protocol-name "/" ] protocol-version`, such as `1.1` or `HTTP/2`. The name is
/// omitted when it is `HTTP`.
///
/// ```
/// use ngsi_ld::http::headers::ReceivedProtocol;
///
/// let protocol: ReceivedProtocol = "HTTP/1.1".parse()?;
/// assert_eq!(protocol.name().map(AsRef::as_ref), Some("HTTP"));
/// assert_eq!(protocol.version().as_ref(), "1.1");
/// assert_eq!("2".parse::<ReceivedProtocol>()?.to_string(), "2");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReceivedProtocol {
    /// `protocol-name`, absent for HTTP.
    name: Option<Token>,
    /// `protocol-version`.
    version: Token,
}

impl ReceivedProtocol {
    /// An HTTP protocol of `version`, written without a name.
    #[must_use]
    pub const fn new(version: Token) -> ReceivedProtocol {
        ReceivedProtocol { name: None, version }
    }

    /// The protocol `name` of `version`.
    #[must_use]
    pub const fn named(name: Token, version: Token) -> ReceivedProtocol {
        ReceivedProtocol { name: Some(name), version }
    }

    /// The protocol name, when written.
    #[must_use]
    pub const fn name(&self) -> Option<&Token> {
        self.name.as_ref()
    }

    /// The protocol version.
    #[must_use]
    pub const fn version(&self) -> &Token {
        &self.version
    }
}

// The name is optional and followed by `/`, which a derive cannot express.
impl fmt::Display for ReceivedProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}/")?;
        }
        self.version.fmt(f)
    }
}

// The protocol name is optional and written before `/`, which a derive cannot express.
impl FromStr for ReceivedProtocol {
    type Err = ReceivedProtocolError;

    fn from_str(text: &str) -> Result<ReceivedProtocol, ReceivedProtocolError> {
        match text.split_once('/') {
            Some((name, version)) => Ok(ReceivedProtocol::named(
                name.parse().map_err(ReceivedProtocolError::Name)?,
                version.parse().map_err(ReceivedProtocolError::Version)?,
            )),
            None => text.parse().map(ReceivedProtocol::new).map_err(ReceivedProtocolError::Version),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{ReceivedProtocol, ReceivedProtocolError},
        model::TokenError,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn a_bare_version_and_a_named_protocol_round_trip() {
        for text in ["1.1", "HTTP/1.1", "HTTP/2", "SHTTP/1.3"] {
            assert_eq!(text.parse::<ReceivedProtocol>().map(|protocol| protocol.to_string()), Ok(text.to_owned()));
        }
        assert_eq!("1.0".parse::<ReceivedProtocol>(), Ok(ReceivedProtocol::new("1.0".parse().unwrap())));
    }

    #[test]
    fn a_value_outside_the_grammar_is_rejected() {
        assert_eq!("".parse::<ReceivedProtocol>(), Err(ReceivedProtocolError::Version(TokenError::Empty)));
        assert_eq!("HTTP/".parse::<ReceivedProtocol>(), Err(ReceivedProtocolError::Version(TokenError::Empty)));
        assert_eq!("/1.1".parse::<ReceivedProtocol>(), Err(ReceivedProtocolError::Name(TokenError::Empty)));
        assert_eq!(
            "a/b/c".parse::<ReceivedProtocol>(),
            Err(ReceivedProtocolError::Version(TokenError::InvalidCharacter))
        );
        assert_eq!(
            "1 1".parse::<ReceivedProtocol>(),
            Err(ReceivedProtocolError::Version(TokenError::InvalidCharacter))
        );
        assert_eq!(
            "HTTP:1.1".parse::<ReceivedProtocol>(),
            Err(ReceivedProtocolError::Version(TokenError::InvalidCharacter))
        );
    }
}
