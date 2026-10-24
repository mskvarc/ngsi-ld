//! The `token` of IETF RFC 9110 section 5.6.2: the lexical unit most HTTP header grammars are built from, and the
//! grammar of the `contextSourceAlias` of a context source (NGSI-LD Tables 5.2.9-1 and 5.2.40-1).
//!
//! The grammar is defined once here, outside the HTTP binding, so the data model and the typed headers share it.

/// Why a string is not an HTTP token.
///
/// The enum is exhaustive: a token is one or more `tchar`, so it is either empty or holds another character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum TokenError {
    /// The token is empty.
    #[error("a token must not be empty")]
    Empty,
    /// The token holds a character that is not a `tchar`.
    #[error("a token may only hold the characters of an HTTP token")]
    InvalidCharacter,
}

/// Whether `byte` is a `tchar` (RFC 9110 section 5.6.2).
pub(crate) const fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
        )
}

/// Whether `text` is a token: one or more `tchar`.
pub(crate) fn is_token(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(is_tchar)
}

/// Checks a token.
fn validate_token(text: &str) -> Result<(), TokenError> {
    if text.is_empty() {
        Err(TokenError::Empty)
    } else if is_token(text) {
        Ok(())
    } else {
        Err(TokenError::InvalidCharacter)
    }
}

/// An HTTP token (RFC 9110 section 5.6.2): one or more visible ASCII characters other than the delimiters
/// `"(),/:;<=>?@[\]{}`, kept as written.
///
/// It is, for instance, the protocol name and version of a `Via` hop and the protocol of an `Alt-Svc` alternative.
///
/// It is also the grammar of the `contextSourceAlias` of a context source (NGSI-LD Tables 5.2.9-1 and 5.2.40-1).
///
/// ```
/// use ngsi_ld::model::{Token, TokenError};
///
/// let token: Token = "HTTP".parse()?;
/// assert_eq!(token.to_string(), "HTTP");
/// assert_eq!("1 1".parse::<Token>(), Err(TokenError::InvalidCharacter));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_token, error = TokenError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct Token(String);

#[cfg(test)]
mod tests {
    use crate::model::{Token, TokenError, token::is_token};
    use pretty_assertions::assert_eq;

    #[test]
    fn every_tchar_is_accepted() {
        let token = "!#$%&'*+-.^_`|~09AZaz";
        assert!(is_token(token));
        assert_eq!(token.parse::<Token>().map(|token| token.to_string()), Ok(token.to_owned()));
    }

    #[test]
    fn an_empty_token_is_rejected() {
        assert_eq!("".parse::<Token>(), Err(TokenError::Empty));
        assert!(!is_token(""));
    }

    #[test]
    fn each_delimiter_whitespace_and_non_ascii_character_is_rejected() {
        for text in [
            "a b",
            "a\"",
            "a(",
            "a)",
            "a,",
            "a/",
            "a:",
            "a;",
            "a<",
            "a=",
            "a>",
            "a?",
            "a@",
            "a[",
            "a\\",
            "a]",
            "a{",
            "a}",
            "caf\u{e9}",
        ] {
            assert_eq!(text.parse::<Token>(), Err(TokenError::InvalidCharacter), "{text}");
        }
    }
}
