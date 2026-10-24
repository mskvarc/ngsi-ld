//! The NGSI-LD Scope Query Language, written in the `scopeQ` parameter.
//!
//! NGSI-LD clause 4.19.
//!
//! A scope query is `/#`, which matches every entity with a scope, or a disjunction of alternatives, each a scope
//! pattern (`/Madrid/+/ParqueNorte`), a pattern with the whole tree of scopes below it (`/Madrid/Gardens/#`), or a
//! parenthesised conjunction of patterns (`(/Madrid/Districts;/CompanyA)`). Clause 4.19 accepts `,` as an
//! alternative spelling of the OR operator; the parser reads it and the printer always writes `|`.
//!
//! ```
//! use ngsi_ld::query::scope_query;
//!
//! let query = scope_query::all_of(["/Madrid/Districts", "/CompanyA"])
//!     .or("/CompanyB")
//!     .build()?;
//! assert_eq!(query.to_string(), "(/Madrid/Districts;/CompanyA)|/CompanyB");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Scope queries
//!
//! [`ScopeQuery`], [`ScopeAlternative`], [`ScopeConjunction`], [`ScopePattern`], [`LevelPattern`].
//!
//! # Start functions
//!
//! [`scope`], [`subtree`], [`any_scope`], [`all_of`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::query::scope_query::pattern::{LevelPattern, ScopePattern};
use crate::query::{
    Expected,
    ParseError,
    grammar::{self, write_separated},
    scope_query::builders::ScopeQueryBuilder,
};
use std::{fmt, str::FromStr};

pub mod builders;
mod parser;
mod pattern;

/// A parenthesised conjunction of scope patterns: the entities within all of them.
///
/// It is the first `OrScopeQ` production of NGSI-LD clause 4.19.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeConjunction(Vec<ScopePattern>);

impl ScopeConjunction {
    /// Returns the patterns; there is at least one.
    #[must_use]
    pub fn patterns(&self) -> &[ScopePattern] {
        &self.0
    }
}

// The patterns are parenthesised and joined with `;`, which a derive cannot express.
impl fmt::Display for ScopeConjunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(")?;
        write_separated(f, &self.0, ';')?;
        f.write_str(")")
    }
}

/// One alternative of a scope query.
///
/// It is the `OrScopeQ` production of NGSI-LD clause 4.19.
///
/// The enum is exhaustive: clause 4.19 defines these three forms.
///
/// ```
/// use ngsi_ld::query::scope_query::{ScopeAlternative, ScopeQuery};
///
/// let query: ScopeQuery = "/Madrid/Gardens/#".parse()?;
/// assert!(matches!(
///     query.alternatives(),
///     [ScopeAlternative::Subtree(_)]
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ScopeAlternative {
    /// The entities within a scope the pattern matches.
    #[display("{_0}")]
    Scope(ScopePattern),
    /// The entities within a scope the pattern matches or any scope below one (`/#` after the pattern).
    #[display("{_0}/#")]
    Subtree(ScopePattern),
    /// The entities within all the patterns.
    #[display("{_0}")]
    AllOf(ScopeConjunction),
}

/// What a scope query selects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Selection {
    /// `/#`: every entity with a scope.
    AnyScope,
    /// The entities one of the alternatives selects; never empty.
    Alternatives(Vec<ScopeAlternative>),
}

/// A scope query.
///
/// It is the `ScopesQ` production of NGSI-LD clause 4.19.
///
/// ```
/// use ngsi_ld::query::scope_query::ScopeQuery;
///
/// let query: ScopeQuery = "(/Madrid/Districts;/CompanyA),/CompanyB".parse()?;
/// assert_eq!(query.alternatives().len(), 2);
/// assert_eq!(query.to_string(), "(/Madrid/Districts;/CompanyA)|/CompanyB");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeQuery(Selection);

impl ScopeQuery {
    /// Whether the query is `/#`, which selects every entity with a scope.
    #[must_use]
    pub const fn is_any_scope(&self) -> bool {
        match self.0 {
            Selection::AnyScope => true,
            Selection::Alternatives(_) => false,
        }
    }

    /// Returns the alternatives; none for `/#`.
    #[must_use]
    pub fn alternatives(&self) -> &[ScopeAlternative] {
        match &self.0 {
            Selection::AnyScope => &[],
            Selection::Alternatives(alternatives) => alternatives,
        }
    }
}

// Alternatives are joined with `|`, which a derive cannot express.
impl fmt::Display for ScopeQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Selection::AnyScope => f.write_str("/#"),
            Selection::Alternatives(alternatives) => write_separated(f, alternatives, '|'),
        }
    }
}

// No derive parses the scope query grammar.
impl FromStr for ScopeQuery {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::scope_query, Expected::Scope, Expected::Separator).map(Self)
    }
}

/// The scope query `/#`, which selects every entity with a scope.
///
/// NGSI-LD clause 4.19.
///
/// ```
/// use ngsi_ld::query::scope_query;
///
/// assert_eq!(scope_query::any_scope().to_string(), "/#");
/// ```
#[must_use]
pub const fn any_scope() -> ScopeQuery {
    ScopeQuery(Selection::AnyScope)
}

/// Starts a scope query with the entities within a scope `pattern` matches.
///
/// An example pattern is `/Madrid/+/ParqueNorte`.
///
/// ```
/// use ngsi_ld::query::scope_query;
///
/// let query = scope_query::scope("/Madrid/+/ParqueNorte").build()?;
/// assert_eq!(query.to_string(), "/Madrid/+/ParqueNorte");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn scope(pattern: impl AsRef<str>) -> ScopeQueryBuilder {
    ScopeQueryBuilder::empty().or(pattern)
}

/// Starts a scope query with the entities in or below a scope `pattern` matches.
///
/// The query matches the entities within a scope the pattern matches or any scope below one, written with `/#` after
/// the pattern.
///
/// ```
/// use ngsi_ld::query::scope_query;
///
/// let query = scope_query::subtree("/Madrid/Gardens").build()?;
/// assert_eq!(query.to_string(), "/Madrid/Gardens/#");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn subtree(pattern: impl AsRef<str>) -> ScopeQueryBuilder {
    ScopeQueryBuilder::empty().or_subtree(pattern)
}

/// Starts a scope query with the entities within all the scope `patterns`.
#[must_use]
pub fn all_of(patterns: impl IntoIterator<Item = impl AsRef<str>>) -> ScopeQueryBuilder {
    ScopeQueryBuilder::empty().or_all_of(patterns)
}

#[cfg(test)]
mod tests;
