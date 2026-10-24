//! The builder of a [`LanguageFilter`].
//!
//! [`LanguageFilterBuilder`].

use crate::{
    model::BuildError,
    query::{
        Problem,
        Quality,
        Weighted,
        build_error::Deferred,
        language_filter::{LanguageFilter, LanguageRange, parse_tag},
    },
};

/// A language filter under construction, started by [`language`](crate::query::language_filter::language) or [`any`](crate::query::language_filter::any).
///
/// Tags are validated on the spot, without panicking; [`build`](Self::build) reports every invalid one. Weights
/// are [`Quality`] values, valid by construction.
///
/// ```
/// use ngsi_ld::query::language_filter;
///
/// let lang = language_filter::language("fr-CH").language("fr").build()?;
/// assert_eq!(lang.to_string(), "fr-CH,fr");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageFilterBuilder(pub(super) Deferred<Vec<Weighted<LanguageRange>>>);

impl LanguageFilterBuilder {
    /// Adds the language `tag`.
    #[must_use]
    pub fn language(self, tag: impl AsRef<str>) -> Self {
        self.push(parse_tag(tag.as_ref()).map(Weighted::new))
    }

    /// Adds the language `tag`, weighted with `quality`.
    #[must_use]
    pub fn weighted_language(self, tag: impl AsRef<str>, quality: Quality) -> Self {
        self.push(parse_tag(tag.as_ref()).map(|range| Weighted::new(range).with_quality(quality)))
    }

    /// Adds the wildcard `*`.
    #[must_use]
    pub fn any(self) -> Self {
        self.push(Deferred::valid(Weighted::new(LanguageRange::Any)))
    }

    /// Adds the wildcard `*`, weighted with `quality`.
    #[must_use]
    pub fn weighted_any(self, quality: Quality) -> Self {
        self.push(Deferred::valid(Weighted::new(LanguageRange::Any).with_quality(quality)))
    }

    /// Appends a preference.
    fn push(self, preference: Deferred<Weighted<LanguageRange>>) -> Self {
        Self(self.0.zip(preference).map(|(mut preferences, preference)| {
            preferences.push(preference);
            preferences
        }))
    }

    /// Returns the filter.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid language tag.
    pub fn build(self) -> Result<LanguageFilter, BuildError<Problem>> {
        self.0.finish().map(LanguageFilter)
    }
}

/// A filter already built.
impl From<LanguageFilter> for LanguageFilterBuilder {
    fn from(filter: LanguageFilter) -> Self {
        Self(Deferred::valid(filter.0))
    }
}

impl TryFrom<LanguageFilterBuilder> for LanguageFilter {
    type Error = BuildError<Problem>;

    fn try_from(builder: LanguageFilterBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
