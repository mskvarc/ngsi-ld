//! One element of a `Via` header: one hop (RFC 9110 section 7.6.3).

use crate::http::headers::{ReceivedBy, ReceivedByError, ReceivedProtocol, ReceivedProtocolError, ViaComment, ViaCommentError};
use std::{fmt, str::FromStr};

/// The whitespace that separates the parts of an element (`RWS`, RFC 9110 section 5.6.3).
const RWS: [char; 2] = [' ', '\t'];

/// Why a string is not a `Via` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ViaEntryError {
    /// The element has no `received-by` after its protocol.
    #[error("a via element names the protocol and then who received it")]
    MissingReceivedBy,
    /// The protocol is malformed.
    #[error("the received protocol is malformed")]
    Protocol(#[from] ReceivedProtocolError),
    /// The recipient is malformed.
    #[error("the recipient is malformed")]
    ReceivedBy(#[from] ReceivedByError),
    /// Text after the recipient is not one comment in parentheses.
    #[error("the text after the recipient is not a comment")]
    NotAComment,
    /// The comment is malformed.
    #[error("the comment is malformed")]
    Comment(#[from] ViaCommentError),
}

/// One hop of a `Via` header (RFC 9110 section 7.6.3): `received-protocol RWS received-by [ RWS comment ]`.
///
/// A context broker forwarding a distributed operation adds one with its `contextSourceAlias` as the pseudonym
/// (NGSI-LD clause 6.3.18).
///
/// ```
/// use ngsi_ld::http::headers::{ReceivedBy, ReceivedProtocol, ViaEntry};
///
/// let entry = ViaEntry::new(
///     "1.1".parse::<ReceivedProtocol>()?,
///     ReceivedBy::new("broker-a".parse()?),
/// );
/// assert_eq!(entry.to_string(), "1.1 broker-a");
/// let parsed: ViaEntry = "HTTP/2 proxy:8080 (edge, eu)".parse()?;
/// assert_eq!(parsed.comment().map(AsRef::as_ref), Some("edge, eu"));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViaEntry {
    /// `received-protocol`.
    protocol: ReceivedProtocol,
    /// `received-by`.
    received_by: ReceivedBy,
    /// `comment`, without its parentheses.
    comment: Option<ViaComment>,
}

impl ViaEntry {
    /// The hop over `protocol` to `received_by`, without a comment.
    #[must_use]
    pub fn new(protocol: ReceivedProtocol, received_by: impl Into<ReceivedBy>) -> ViaEntry {
        ViaEntry {
            protocol,
            received_by: received_by.into(),
            comment: None,
        }
    }

    /// Adds `comment`.
    #[must_use]
    pub fn with_comment(self, comment: ViaComment) -> ViaEntry {
        ViaEntry {
            comment: Some(comment),
            ..self
        }
    }

    /// The protocol the hop received the message in.
    #[must_use]
    pub const fn protocol(&self) -> &ReceivedProtocol {
        &self.protocol
    }

    /// Who received the message.
    #[must_use]
    pub const fn received_by(&self) -> &ReceivedBy {
        &self.received_by
    }

    /// The comment, without its parentheses.
    #[must_use]
    pub const fn comment(&self) -> Option<&ViaComment> {
        self.comment.as_ref()
    }
}

// The parts are space-separated and the comment optional and parenthesised, which a derive cannot express.
impl fmt::Display for ViaEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.protocol, self.received_by)?;
        self.comment.as_ref().map_or(Ok(()), |comment| write!(f, " ({comment})"))
    }
}

// The parts are space-separated and the comment optional and parenthesised, which a derive cannot express.
impl FromStr for ViaEntry {
    type Err = ViaEntryError;

    fn from_str(text: &str) -> Result<ViaEntry, ViaEntryError> {
        let (protocol, rest) = text.split_once(RWS).ok_or(ViaEntryError::MissingReceivedBy)?;
        let rest = rest.trim_start_matches(RWS);
        let (received_by, rest) = rest.split_once(RWS).unwrap_or((rest, ""));
        let entry = ViaEntry::new(protocol.parse()?, received_by.parse::<ReceivedBy>()?);
        let rest = rest.trim_matches(RWS);
        if rest.is_empty() {
            return Ok(entry);
        }
        let comment = rest
            .strip_prefix('(')
            .and_then(|comment| comment.strip_suffix(')'))
            .ok_or(ViaEntryError::NotAComment)?;
        Ok(entry.with_comment(comment.parse()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{ReceivedByError, ReceivedProtocolError, ViaCommentError, ViaEntry, ViaEntryError},
        model::TokenError,
    };
    use pretty_assertions::assert_eq;

    fn parse(text: &str) -> Result<String, ViaEntryError> {
        text.parse::<ViaEntry>().map(|entry| entry.to_string())
    }

    #[test]
    fn an_element_with_or_without_a_comment_round_trips() {
        for text in ["1.1 broker-a", "HTTP/2 proxy.example.org:8080", "1.0 fred (Apache/1.1)", "1.1 p (a, (b))"] {
            assert_eq!(parse(text), Ok(text.to_owned()), "{text}");
        }
    }

    #[test]
    fn extra_whitespace_between_parts_is_read() {
        assert_eq!(parse("1.1 \t broker-a \t (x) "), Ok("1.1 broker-a (x)".to_owned()));
        let entry: ViaEntry = "1.1 broker-a".parse().unwrap();
        assert_eq!(entry.protocol().to_string(), "1.1");
        assert_eq!(entry.received_by().to_string(), "broker-a");
        assert_eq!(entry.comment(), None);
    }

    #[test]
    fn a_malformed_element_is_rejected_with_its_reason() {
        assert_eq!(parse("broker-a"), Err(ViaEntryError::MissingReceivedBy));
        assert_eq!(
            parse("1.1/ broker-a"),
            Err(ViaEntryError::Protocol(ReceivedProtocolError::Version(TokenError::Empty)))
        );
        assert_eq!(parse("1.1 broker-a:x"), Err(ViaEntryError::ReceivedBy(ReceivedByError::Port)));
        assert_eq!(parse("1.1 broker-a extra"), Err(ViaEntryError::NotAComment));
        assert_eq!(parse("1.1 broker-a (a) (b)"), Err(ViaEntryError::Comment(ViaCommentError::Unbalanced)));
    }
}
