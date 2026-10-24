//! The scheme of a URI (RFC 3986 section 3.1), as the forwarding headers name a protocol.

/// Why a string is not a URI scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum SchemeError {
    /// The scheme is empty.
    #[error("a scheme must not be empty")]
    Empty,
    /// The scheme does not start with a letter, or holds a character other than a letter, a digit, `+`, `-` or `.`.
    #[error("a scheme is a letter followed by letters, digits, +, - and .")]
    InvalidCharacter,
}

/// Checks a scheme: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )` (RFC 3986 section 3.1).
fn validate_scheme(scheme: &str) -> Result<(), SchemeError> {
    let mut bytes = scheme.bytes();
    let first = bytes.next().ok_or(SchemeError::Empty)?;
    if first.is_ascii_alphabetic() && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')) {
        Ok(())
    } else {
        Err(SchemeError::InvalidCharacter)
    }
}

/// A URI scheme (RFC 3986 section 3.1), such as `http`, `https` or `ws`, kept in lowercase since schemes are
/// case-insensitive.
///
/// ```
/// use ngsi_ld::http::headers::{Scheme, SchemeError};
///
/// let scheme: Scheme = "HTTPS".parse()?;
/// assert_eq!(scheme.to_string(), "https");
/// assert_eq!(
///     "1http".parse::<Scheme>(),
///     Err(SchemeError::InvalidCharacter)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    sanitize(lowercase),
    validate(with = validate_scheme, error = SchemeError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct Scheme(String);

#[cfg(test)]
mod tests {
    use crate::http::headers::{Scheme, SchemeError};
    use pretty_assertions::assert_eq;

    #[test]
    fn a_scheme_is_kept_in_lowercase() {
        for (text, expected) in [
            ("http", "http"),
            ("HTTPS", "https"),
            ("Ws", "ws"),
            ("coap+tcp", "coap+tcp"),
            ("a1.b-c", "a1.b-c"),
        ] {
            assert_eq!(text.parse::<Scheme>().map(|scheme| scheme.to_string()), Ok(expected.to_owned()), "{text}");
        }
    }

    #[test]
    fn a_scheme_outside_the_grammar_is_rejected() {
        assert_eq!("".parse::<Scheme>(), Err(SchemeError::Empty));
        for text in ["1http", "+http", "ht tp", "http:", "ht_tp", "caf\u{e9}"] {
            assert_eq!(text.parse::<Scheme>(), Err(SchemeError::InvalidCharacter), "{text}");
        }
    }
}
