//! The builder of scope queries, started by [`scope`](crate::query::scope_query::scope),
//! [`subtree`](crate::query::scope_query::subtree) or [`all_of`](crate::query::scope_query::all_of).
//!
//! [`ScopeQueryBuilder`].

use crate::{
    model::BuildError,
    query::{
        Problem,
        build_error::Deferred,
        scope_query::{ScopeAlternative, ScopeConjunction, ScopePattern, ScopeQuery, Selection},
    },
};

/// Validates one scope pattern.
fn parse_pattern(pattern: &str) -> Deferred<ScopePattern> {
    Deferred::from_result(pattern.parse().map_err(|source| Problem::InvalidScopePattern { source }))
}

/// A scope query under construction.
///
/// Patterns are validated on the spot, without panicking; [`build`](Self::build) reports every invalid one.
///
/// ```
/// use ngsi_ld::query::scope_query;
///
/// let query = scope_query::scope("/Madrid")
///     .or_subtree("/Barcelona")
///     .or_all_of(["/CompanyA", "/CompanyB"])
///     .build()?;
/// assert_eq!(
///     query.to_string(),
///     "/Madrid|/Barcelona/#|(/CompanyA;/CompanyB)"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeQueryBuilder(Deferred<Selection>);

impl ScopeQueryBuilder {
    /// No alternative yet.
    pub(crate) const fn empty() -> Self {
        Self(Deferred::valid(Selection::Alternatives(Vec::new())))
    }

    /// Adds the entities within a scope `pattern` matches.
    #[must_use]
    pub fn or(self, pattern: impl AsRef<str>) -> Self {
        self.push(parse_pattern(pattern.as_ref()).map(ScopeAlternative::Scope))
    }

    /// Adds the entities within a scope `pattern` matches or any scope below one (`/#` after the pattern).
    #[must_use]
    pub fn or_subtree(self, pattern: impl AsRef<str>) -> Self {
        self.push(parse_pattern(pattern.as_ref()).map(ScopeAlternative::Subtree))
    }

    /// Adds the entities within all the scope `patterns`.
    #[must_use]
    pub fn or_all_of(self, patterns: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let conjunction = patterns
            .into_iter()
            .map(|pattern| parse_pattern(pattern.as_ref()))
            .collect::<Deferred<Vec<_>>>()
            .and_then(|patterns| {
                if patterns.is_empty() {
                    Err(Problem::EmptyScopeConjunction)
                } else {
                    Ok(patterns)
                }
            });
        self.push(conjunction.map(|patterns| ScopeAlternative::AllOf(ScopeConjunction(patterns))))
    }

    /// Appends an alternative; `/#` takes none (clause 4.19).
    fn push(self, alternative: Deferred<ScopeAlternative>) -> Self {
        Self(self.0.zip(alternative).and_then(|(selection, alternative)| match selection {
            Selection::AnyScope => Err(Problem::AnyScopeWithAlternatives),
            Selection::Alternatives(mut alternatives) => {
                alternatives.push(alternative);
                Ok(Selection::Alternatives(alternatives))
            }
        }))
    }

    /// Returns the scope query.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid pattern, every empty conjunction, and an alternative added to
    /// `/#`.
    pub fn build(self) -> Result<ScopeQuery, BuildError<Problem>> {
        self.0.finish().map(ScopeQuery)
    }
}

/// A scope query of the one scope `pattern`, as [`scope`](crate::query::scope_query::scope) starts it.
impl From<&str> for ScopeQueryBuilder {
    fn from(pattern: &str) -> Self {
        Self::empty().or(pattern)
    }
}

/// A scope query already built.
impl From<ScopeQuery> for ScopeQueryBuilder {
    fn from(query: ScopeQuery) -> Self {
        Self(Deferred::valid(query.0))
    }
}

impl TryFrom<ScopeQueryBuilder> for ScopeQuery {
    type Error = BuildError<Problem>;

    fn try_from(builder: ScopeQueryBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
