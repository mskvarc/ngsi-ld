//! The basic language range an `Accept-Language` header lists (RFC 9110 section 12.5.4, RFC 4647 section 2.1).

use crate::query::language_filter::LanguageRange;
use std::str::FromStr;

/// The range matching any language (RFC 4647 section 2.1).
const ANY: &str = "*";

/// The most characters a subtag of a basic language range holds (RFC 4647 section 2.1).
const MAXIMUM_SUBTAG_LENGTH: usize = 8;

/// Why a string is not a basic language range (RFC 4647 section 2.1).
///
/// The enum is exhaustive: the grammar of a basic language range has no other rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum BasicLanguageRangeError {
    /// The text is empty.
    #[error("a language range cannot be empty")]
    Empty,
    /// The first subtag is not one to eight letters.
    #[error("the first subtag of a language range is one to eight letters")]
    FirstSubtag,
    /// A later subtag is not one to eight letters or digits.
    #[error("a subtag of a language range after the first is one to eight letters or digits")]
    Subtag,
}

/// A basic language range (RFC 4647 section 2.1), the element RFC 9110 section 12.5.4 gives `Accept-Language`:
/// `*`, or one to eight letters followed by `-`-separated subtags of one to eight letters or digits. It is kept as
/// written; RFC 4647 compares ranges without regard to case.
///
/// A basic range need not be a language tag: `en-a-b` is a range, but no BCP 47 tag, since an extension subtag has at
/// least two characters. Every language tag is a range, so every [`LanguageRange`] of a Language Filter (NGSI-LD
/// clause 4.15) converts into one.
///
/// ```
/// use ngsi_ld::{
///     http::headers::{BasicLanguageRange, BasicLanguageRangeError},
///     query::language_filter::LanguageRange,
/// };
///
/// let range: BasicLanguageRange = "en-a-b".parse()?;
/// assert_eq!(range.as_ref(), "en-a-b");
/// assert_eq!(
///     BasicLanguageRange::from(LanguageRange::Any).to_string(),
///     "*"
/// );
/// assert_eq!(
///     "en_US".parse::<BasicLanguageRange>(),
///     Err(BasicLanguageRangeError::FirstSubtag)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::AsRef)]
#[as_ref(str)]
pub struct BasicLanguageRange(String);

impl BasicLanguageRange {
    /// `*`: any language.
    #[must_use]
    pub fn any() -> BasicLanguageRange {
        BasicLanguageRange(ANY.to_owned())
    }
}

/// Whether `subtag` holds one to eight bytes, each `allowed`.
fn is_subtag(subtag: &str, allowed: fn(&u8) -> bool) -> bool {
    (1..=MAXIMUM_SUBTAG_LENGTH).contains(&subtag.len()) && subtag.as_bytes().iter().all(allowed)
}

/// Checks `language-range = (1*8ALPHA *("-" 1*8alphanum)) / "*"` (RFC 4647 section 2.1).
fn validate(text: &str) -> Result<(), BasicLanguageRangeError> {
    if text == ANY {
        return Ok(());
    }
    if text.is_empty() {
        return Err(BasicLanguageRangeError::Empty);
    }
    let mut subtags = text.split('-');
    if !subtags.next().is_some_and(|first| is_subtag(first, u8::is_ascii_alphabetic)) {
        return Err(BasicLanguageRangeError::FirstSubtag);
    }
    if !subtags.all(|subtag| is_subtag(subtag, u8::is_ascii_alphanumeric)) {
        return Err(BasicLanguageRangeError::Subtag);
    }
    Ok(())
}

// A derive would delegate to the infallible `FromStr` of `String`; the grammar has to be checked here instead, and
// a nutype would make the conversion from a language tag check again what the tag grammar already guarantees.
impl FromStr for BasicLanguageRange {
    type Err = BasicLanguageRangeError;

    fn from_str(text: &str) -> Result<BasicLanguageRange, BasicLanguageRangeError> {
        validate(text).map(|()| BasicLanguageRange(text.to_owned()))
    }
}

/// The range of a Language Filter preference (NGSI-LD clause 4.15). A BCP 47 tag (RFC 5646 section 2.1) is one to
/// eight letters followed by subtags of one to eight letters or digits, so it is a basic range as written.
impl From<LanguageRange> for BasicLanguageRange {
    fn from(range: LanguageRange) -> BasicLanguageRange {
        match range {
            LanguageRange::Tag(tag) => BasicLanguageRange(tag.to_string()),
            LanguageRange::Any => BasicLanguageRange::any(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{BasicLanguageRange, BasicLanguageRangeError},
        query::language_filter::LanguageRange,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn every_form_of_the_grammar_is_read_as_written() {
        for text in [
            "*",
            "en",
            "de-AT",
            "en-a-b",
            "zh-Hant-TW",
            "x-private",
            "i-default",
            "abcdefgh-12345678",
            "sgn-BE-FR",
        ] {
            assert_eq!(text.parse::<BasicLanguageRange>().map(|range| range.to_string()), Ok(text.to_owned()), "{text}");
        }
        assert_eq!("*".parse::<BasicLanguageRange>(), Ok(BasicLanguageRange::any()));
    }

    #[test]
    fn an_empty_text_or_a_malformed_subtag_is_rejected_with_its_reason() {
        assert_eq!("".parse::<BasicLanguageRange>(), Err(BasicLanguageRangeError::Empty));
        for text in ["en_US", "1en", "abcdefghi", "-en", "*-CH", "e\u{e9}", "**", "fr en-x"] {
            assert_eq!(text.parse::<BasicLanguageRange>(), Err(BasicLanguageRangeError::FirstSubtag), "{text}");
        }
        for text in ["en-", "en--US", "en-123456789", "en-*", "en-U S"] {
            assert_eq!(text.parse::<BasicLanguageRange>(), Err(BasicLanguageRangeError::Subtag), "{text}");
        }
    }

    #[test]
    fn every_language_filter_range_converts_as_written() {
        for text in ["fr-CH", "en-GB-oed", "x-whatever", "zh-Hant-TW", "de-DE-u-co-phonebk"] {
            let range = BasicLanguageRange::from(LanguageRange::Tag(text.parse().unwrap()));
            assert_eq!(range.as_ref(), text);
            assert_eq!(text.parse::<BasicLanguageRange>(), Ok(range));
        }
        assert_eq!(BasicLanguageRange::from(LanguageRange::Any), BasicLanguageRange::any());
    }
}
