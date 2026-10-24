//! Content codings (RFC 9110 section 8.4.1) and the coding ranges `Accept-Encoding` lists (section 12.5.3).

use crate::model::{TokenError, token::is_token};
use std::str::FromStr;

/// The coding that applies no transformation, which RFC 9110 section 12.5.3 names apart from the content codings.
const IDENTITY: &str = "identity";

/// The range that matches any coding (RFC 9110 section 12.5.3).
const ANY: &str = "*";

/// Why a string is not a content coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ContentCodingError {
    /// The coding is not an HTTP token.
    #[error("a content coding is an HTTP token")]
    Token(#[from] TokenError),
    /// The text is `identity` or `*`, which `Accept-Encoding` names apart from the content codings.
    #[error("identity and * are coding ranges, not content codings")]
    Reserved,
}

/// Checks a content coding, already in lowercase.
fn validate_coding(coding: &str) -> Result<(), ContentCodingError> {
    if coding.is_empty() {
        Err(TokenError::Empty.into())
    } else if !is_token(coding) {
        Err(TokenError::InvalidCharacter.into())
    } else if coding == IDENTITY || coding == ANY {
        Err(ContentCodingError::Reserved)
    } else {
        Ok(())
    }
}

/// A content coding (RFC 9110 section 8.4.1), such as `gzip`, `deflate`, `br` or `zstd`: a token, kept in
/// lowercase since content codings are case-insensitive.
///
/// ```
/// use ngsi_ld::http::headers::{ContentCoding, ContentCodingError};
///
/// let coding: ContentCoding = "GZip".parse()?;
/// assert_eq!(coding.to_string(), "gzip");
/// assert_eq!(
///     "identity".parse::<ContentCoding>(),
///     Err(ContentCodingError::Reserved)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    sanitize(lowercase),
    validate(with = validate_coding, error = ContentCodingError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ContentCoding(String);

/// A coding an `Accept-Encoding` element names (RFC 9110 section 12.5.3): a content coding, `identity`, or `*` for
/// any coding.
///
/// The enum is exhaustive: section 12.5.3 defines `codings` as exactly these three forms.
///
/// ```
/// use ngsi_ld::http::headers::CodingRange;
///
/// assert_eq!("IDENTITY".parse::<CodingRange>(), Ok(CodingRange::Identity));
/// assert_eq!(CodingRange::Any.to_string(), "*");
/// assert_eq!("br".parse::<CodingRange>()?.to_string(), "br");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum CodingRange {
    /// A content coding.
    #[display("{_0}")]
    Coding(ContentCoding),
    /// `identity`: the representation without any coding.
    #[display("identity")]
    Identity,
    /// `*`: any coding not listed otherwise.
    #[display("*")]
    Any,
}

// `*` and `identity` are recognised before any other coding, which `strum::EnumString` cannot do for the `Coding` variant.
impl FromStr for CodingRange {
    type Err = ContentCodingError;

    fn from_str(text: &str) -> Result<CodingRange, ContentCodingError> {
        if text == ANY {
            Ok(CodingRange::Any)
        } else if text.eq_ignore_ascii_case(IDENTITY) {
            Ok(CodingRange::Identity)
        } else {
            text.parse().map(CodingRange::Coding)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{CodingRange, ContentCoding, ContentCodingError},
        model::TokenError,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn a_coding_is_kept_in_lowercase() {
        assert_eq!("ZSTD".parse::<ContentCoding>().map(|coding| coding.to_string()), Ok("zstd".to_owned()));
        assert_eq!("x-gzip".parse::<ContentCoding>().map(|coding| coding.to_string()), Ok("x-gzip".to_owned()));
    }

    #[test]
    fn a_coding_that_is_no_token_or_is_reserved_is_rejected() {
        assert_eq!("".parse::<ContentCoding>(), Err(ContentCodingError::Token(TokenError::Empty)));
        assert_eq!("g zip".parse::<ContentCoding>(), Err(ContentCodingError::Token(TokenError::InvalidCharacter)));
        assert_eq!("Identity".parse::<ContentCoding>(), Err(ContentCodingError::Reserved));
        assert_eq!("*".parse::<ContentCoding>(), Err(ContentCodingError::Reserved));
    }

    #[test]
    fn each_coding_range_parses_and_renders() {
        assert_eq!("*".parse::<CodingRange>(), Ok(CodingRange::Any));
        assert_eq!("identity".parse::<CodingRange>(), Ok(CodingRange::Identity));
        assert_eq!("gzip".parse::<CodingRange>().map(|range| range.to_string()), Ok("gzip".to_owned()));
        assert_eq!(CodingRange::Identity.to_string(), "identity");
        assert_eq!("gz;ip".parse::<CodingRange>(), Err(ContentCodingError::Token(TokenError::InvalidCharacter)));
    }
}
