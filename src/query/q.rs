//! The NGSI-LD Query Language (NGSI-LD clause 4.9), written in the `q` parameter and, for context source
//! registrations, in the `csf` parameter.
//!
//! A [`Query`] is a tree of [`QueryTerm`]s combined with AND (`;`) and OR (`|`). AND binds tighter than OR
//! (clause 4.9), so the printer writes parentheses only around an OR that is an operand of an AND.
//!
//! The builder starts from [`attr`] and reads like the query it prints:
//!
//! ```
//! use ngsi_ld::query::q::{self, Query};
//!
//! let query: Query = q::attr("speed")
//!     .gt(50)
//!     .and(q::attr("brandName").ne("Mercedes"))
//!     .build()?;
//! assert_eq!(query.to_string(), r#"speed>50;brandName!="Mercedes""#);
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! The parser reads the same text back into the same tree:
//!
//! ```
//! use ngsi_ld::query::q::Query;
//!
//! let query: Query = r#"(speed>50|rpm>3000);brandName=="Mercedes""#.parse()?;
//! assert!(matches!(query, Query::And(_)));
//! assert_eq!(
//!     query.to_string(),
//!     r#"(speed>50|rpm>3000);brandName=="Mercedes""#
//! );
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! Names are given as strings and checked as the builder goes, without panicking: an invalid name is carried
//! along as a [`Problem`], and [`QueryBuilder::build`] (or
//! `Query::try_from`) returns every problem at once.
//!
//! # Where the parser accepts more than the grammar
//!
//! The ABNF of clause 4.9 is narrower than the examples and the prose of the same clause. The parser follows
//! the examples:
//!
//! - groups nest to any depth (example 6 nests two levels, which `QueryTermAssoc` does not allow);
//! - a linked entity path takes a type hint of one entity type (example 14, `sensor{Device:humidity}`), where
//!   the ABNF asks for at least two, and the hint may precede a plain attribute path;
//! - a member path holds `*` and language tags (the prose uses `color[*]`, Annex C.5.13 uses `marque[en-GB]`),
//!   where the ABNF allows only `AttrName`s.
//!
//! # Depth
//!
//! Groups nest to any depth and a path follows any number of relationships, so a query a broker returns may be
//! thousands of levels deep. The parser, the printer and every walk of the tree (`Clone`, `PartialEq`, `Hash`,
//! `Debug`, `Drop` and the mapping of names) run each level of grouping on a stack that grows with the depth, and
//! a linked entity path is a flat list of links, so no depth overflows the stack.
//!
//! # Queries
//!
//! [`Query`], [`Operands`], [`QueryTerm`], [`Comparison`], [`Operator`], [`attr`].
//!
//! # Paths
//!
//! [`AttributePath`], [`ValuePath`], [`MemberName`], [`MemberError`], [`Link`], [`LinkedPath`].
//!
//! # Values
//!
//! [`Value`], [`ComparableValue`], [`EqualityValue`], [`ValueList`], [`ValueListError`], [`Range`], [`UnquotedUri`],
//! [`UnquotedUriError`], [`Pattern`], [`PatternError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, each_in_text},
};
pub use crate::query::q::{
    path::{AttributePath, Link, LinkedPath, MemberError, MemberName, ValuePath},
    pattern::{Pattern, PatternError},
    term::{Comparison, Operator, QueryTerm},
    value::{ComparableValue, EqualityValue, Range, UnquotedUri, UnquotedUriError, Value, ValueList, ValueListError},
};
use crate::{
    model::{BuildError, nesting::grow},
    query::{
        Expected,
        ParseError,
        Problem,
        grammar,
        q::builders::{PathBuilder, QueryBuilder},
    },
};
use std::{fmt, mem, slice, str::FromStr};

pub mod builders;
pub(crate) mod parser;
mod path;
mod pattern;
mod term;
mod value;
mod walk;

/// Starts a query on the attribute `name` (the first `AttrName` of an attribute path, clause 4.9).
///
/// An invalid name does not panic: it is reported by
/// [`QueryBuilder::build`](crate::query::q::builders::QueryBuilder::build).
///
/// ```
/// use ngsi_ld::query::q;
///
/// let query = q::attr("temperature").exists().build()?;
/// assert_eq!(query.to_string(), "temperature");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn attr(name: impl AsRef<str>) -> PathBuilder {
    PathBuilder::new(name.as_ref())
}

/// A query of the NGSI-LD Query Language (the `Query` production of clause 4.9).
///
/// AND and OR are n-ary: an AND never holds another AND as a direct operand, nor an OR another OR, so a query
/// has exactly one tree and the printed form is unambiguous. [`Query::and`] and [`Query::or`] keep the tree in
/// that shape.
///
/// The enum is exhaustive: clause 4.9 defines exactly these two logical operators.
///
/// ```
/// use ngsi_ld::query::q;
///
/// let fast = q::attr("speed").gt(50).build()?;
/// let revving = q::attr("rpm").gt(3000).build()?;
/// let mercedes = q::attr("brandName").eq("Mercedes").build()?;
/// let query = fast.or(revving).and(mercedes);
/// assert_eq!(
///     query.to_string(),
///     r#"(speed>50|rpm>3000);brandName=="Mercedes""#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::From)]
pub enum Query {
    /// A single query term.
    #[from]
    Term(QueryTerm),
    /// Every operand holds (`andOp`, `;`).
    And(Operands),
    /// At least one operand holds (`orOp`, `|`).
    Or(Operands),
}

/// The operands of an AND or an OR: at least two, none of them the same operator.
///
/// Every level of grouping passes through the operands, so their `Clone`, `PartialEq`, `Hash`, `Debug` and `Drop`
/// are written to run each level on a stack that grows with the depth.
#[derive(Eq)]
pub struct Operands(Vec<Query>);

impl Operands {
    /// Returns the operands, in order.
    #[must_use]
    pub fn as_slice(&self) -> &[Query] {
        &self.0
    }

    /// Iterates over the operands, in order.
    pub fn iter(&self) -> slice::Iter<'_, Query> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Operands {
    type IntoIter = slice::Iter<'a, Query>;
    type Item = &'a Query;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Query {
    /// Combines two queries with AND, merging nested ANDs into one.
    #[must_use]
    pub fn and(self, other: Self) -> Self {
        let mut operands = self.into_and_operands();
        operands.extend(other.into_and_operands());
        Self::And(Operands(operands))
    }

    /// Combines two queries with OR, merging nested ORs into one.
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        let mut operands = self.into_or_operands();
        operands.extend(other.into_or_operands());
        Self::Or(Operands(operands))
    }

    /// The operands this query contributes to an enclosing AND.
    fn into_and_operands(self) -> Vec<Self> {
        match self {
            Self::And(mut operands) => mem::take(&mut operands.0),
            single @ (Self::Term(_) | Self::Or(_)) => vec![single],
        }
    }

    /// The operands this query contributes to an enclosing OR.
    fn into_or_operands(self) -> Vec<Self> {
        match self {
            Self::Or(mut operands) => mem::take(&mut operands.0),
            single @ (Self::Term(_) | Self::And(_)) => vec![single],
        }
    }
}

/// Writes `operands` separated by `separator`, parenthesising each operand `needs_group` selects.
///
/// Each level of grouping is written on a stack that grows with the depth.
fn write_operands(f: &mut fmt::Formatter<'_>, operands: &Operands, separator: char, needs_group: fn(&Query) -> bool) -> fmt::Result {
    grow(|| {
        operands.iter().enumerate().try_for_each(|(index, operand)| {
            if index > 0 {
                write!(f, "{separator}")?;
            }
            if needs_group(operand) {
                write!(f, "({operand})")
            } else {
                write!(f, "{operand}")
            }
        })
    })
}

// The printer joins operands and decides where parentheses go, which a derive cannot express.
impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Term(term) => term.fmt(f),
            Self::And(operands) => write_operands(f, operands, ';', |operand| matches!(operand, Self::Or(_))),
            Self::Or(operands) => write_operands(f, operands, '|', |_| false),
        }
    }
}

// No derive parses the query grammar.
impl FromStr for Query {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::query, Expected::QueryTerm, Expected::LogicalOperator)
    }
}

impl TryFrom<QueryBuilder> for Query {
    type Error = BuildError<Problem>;

    fn try_from(builder: QueryBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

#[cfg(feature = "linked-data")]
impl MapNames for Query {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Term(term) => term.map_names(mapping).map(Self::Term),
            Self::And(mut operands) => grow(|| each_in_text(mem::take(&mut operands.0), mapping)).map(|operands| Self::And(Operands(operands))),
            Self::Or(mut operands) => grow(|| each_in_text(mem::take(&mut operands.0), mapping)).map(|operands| Self::Or(Operands(operands))),
        }
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// A query carries no `@context`: it is written under the context of the request that carries it.
    impl for Query
);

#[cfg(test)]
mod tests;
