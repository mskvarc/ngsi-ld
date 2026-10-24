//! The NGSI-LD Language Filter (NGSI-LD clause 4.15), written in the `lang` parameter of Retrieve Entity and Query
//! Entities.
//!
//! A filter asks the broker to return each `LanguageProperty` as a `Property` whose value is the text of one
//! language, with a `lang` member naming the language chosen. It lists language preferences separated by `,`, each a
//! language tag (IETF RFC 5646) or the wildcard `*`, optionally weighted with a quality value as in an HTTP
//! `Accept-Language` field (clause 4.15, example 4; IETF RFC 9110 section 12.4.2):
//!
//! ```
//! use ngsi_ld::query::{Quality, language_filter};
//!
//! let lang = language_filter::language("fr-CH")
//!     .language("fr")
//!     .weighted_language("en", Quality::from_thousandths(800)?)
//!     .weighted_any(Quality::from_thousandths(500)?)
//!     .build()?;
//! assert_eq!(lang.to_string(), "fr-CH,fr,en;q=0.8,*;q=0.5");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Language filters
//!
//! [`LanguageFilter`], [`LanguageRange`].
//!
//! # Start functions
//!
//! [`language`], [`any`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

use crate::{
    model::language::parse_language_tag,
    query::{
        Expected,
        ParseError,
        Problem,
        Weighted,
        build_error::Deferred,
        grammar::{self, write_separated},
        language_filter::builders::LanguageFilterBuilder,
    },
};
use langtag::LangTagBuf;
use std::{fmt, str::FromStr};

pub mod builders;
mod parser;

/// The wildcard of a language filter, which matches any language (clause 4.15, example 3).
const WILDCARD: &str = "*";

/// The languages a preference of a language filter accepts: one language tag, or any language.
///
/// Clause 4.15 takes the language tags of IETF RFC 5646, so a range here is a well-formed BCP 47 tag, not any basic
/// language range of IETF RFC 4647 as an HTTP `Accept-Language` field allows; `en-a-b`, a valid basic range but no
/// tag, is refused. A preference is weighted as a [`Weighted<LanguageRange>`](Weighted).
///
/// The enum is exhaustive: clause 4.15 names a language or uses the wildcard.
///
/// ```
/// use ngsi_ld::query::language_filter::LanguageRange;
///
/// assert_eq!(LanguageRange::Any.to_string(), "*");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum LanguageRange {
    /// A language tag (IETF RFC 5646).
    #[display("{_0}")]
    Tag(LangTagBuf),
    /// `*`: any language the broker supports (clause 4.15, example 3).
    #[display("*")]
    Any,
}

/// A language filter (NGSI-LD clause 4.15): a non-empty list of language preferences.
///
/// ```
/// use ngsi_ld::query::language_filter::LanguageFilter;
///
/// let lang: LanguageFilter = "fr-CH,fr;q=0.9,en;q=0.8,*;q=0.5".parse()?;
/// assert_eq!(lang.preferences().len(), 4);
/// assert_eq!(lang.to_string(), "fr-CH,fr;q=0.9,en;q=0.8,*;q=0.5");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageFilter(Vec<Weighted<LanguageRange>>);

impl LanguageFilter {
    /// Returns the preferences, in order; there is at least one.
    #[must_use]
    pub fn preferences(&self) -> &[Weighted<LanguageRange>] {
        &self.0
    }

    /// Takes the preferences, in order; there is at least one.
    #[must_use]
    pub fn into_preferences(self) -> Vec<Weighted<LanguageRange>> {
        self.0
    }
}

// The preferences are joined with `,`, which a derive cannot express.
impl fmt::Display for LanguageFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated(f, &self.0, ',')
    }
}

// No derive parses the language filter grammar.
impl FromStr for LanguageFilter {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::filter, Expected::LanguageRange, Expected::LanguageSeparator)
    }
}

/// Validates a language tag.
fn parse_tag(tag: &str) -> Deferred<LanguageRange> {
    Deferred::from_result(
        parse_language_tag(tag)
            .map(LanguageRange::Tag)
            .map_err(|source| Problem::InvalidLanguageTag { source }),
    )
}

/// Starts a filter with the language `tag`.
///
/// ```
/// use ngsi_ld::query::language_filter;
///
/// assert_eq!(language_filter::language("en").build()?.to_string(), "en");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn language(tag: impl AsRef<str>) -> LanguageFilterBuilder {
    LanguageFilterBuilder(Deferred::valid(Vec::new())).language(tag)
}

/// Starts a filter with the wildcard `*`: the text of any language the broker supports (clause 4.15, example 3).
#[must_use]
pub fn any() -> LanguageFilterBuilder {
    LanguageFilterBuilder(Deferred::valid(Vec::new())).any()
}

/// Whether `text` is the wildcard.
fn is_wildcard(text: &str) -> bool {
    text == WILDCARD
}

#[cfg(test)]
mod tests;
