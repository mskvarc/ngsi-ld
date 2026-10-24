//! Regular expressions of the NGSI-LD Query Language (the `RegExp` production of NGSI-LD clause 4.9).

use crate::query::{
    RegularExpressionError,
    regular_expression::{Boundary, check},
};

/// Why a string is not a regular expression that can be written in a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum PatternError {
    /// The expression does not close its groups, bracket expressions or escapes, or is empty.
    #[error(transparent)]
    Structure(#[from] RegularExpressionError),
    /// A `;`, a `|` or a `)` would end the expression early in a query.
    #[error("`{found}` at byte {position} would end the regular expression in a query")]
    Delimiter {
        /// The delimiting character.
        found: char,
        /// Its byte offset in the expression.
        position: usize,
    },
}

/// Validates that `value` is a non-empty regular expression a query delimits exactly.
fn validate_pattern(value: &str) -> Result<(), PatternError> {
    let scan = check(value, Boundary::Query)?;
    match value.get(scan.length..).and_then(|rest| rest.chars().next()) {
        Some(found) => Err(PatternError::Delimiter { found, position: scan.length }),
        None => Ok(()),
    }
}

/// A regular expression (IEEE 1003.2, POSIX extended syntax) for the `~=` and `!~=` operators of NGSI-LD
/// clause 4.9, written verbatim in the query.
///
/// The expression is not compiled: a broker evaluates it. It is only checked to be non-empty and to be delimited
/// exactly when written in a query, so that every printed query parses back to the same expression. The `idPattern`
/// of a query or an entity selector is a whole value of its own, read as an [`IdPattern`](crate::query::IdPattern),
/// which may hold a top-level alternation.
///
/// Clause 4.9 writes a regular expression (IEEE 1003.2, POSIX extended syntax) unquoted after `~=` or `!~=`,
/// and does not say where it ends. A regular expression can hold `;`, `|` and `)`, which also delimit query
/// terms. The crate resolves the ambiguity the way the structure of the expression suggests: the expression
/// ends at the first `;` or `|` outside any group and bracket expression, or at a `)` that closes no group of
/// the expression. A `|` inside a group, such as `(c|z)`, belongs to the expression; a top-level alternation
/// cannot be written in a query and has to be grouped.
///
/// ```
/// use ngsi_ld::query::q::Pattern;
///
/// let pattern: Pattern = "^Mer(c|z)edes$".parse()?;
/// assert_eq!(pattern.to_string(), "^Mer(c|z)edes$");
/// assert!("Mercedes|BMW".parse::<Pattern>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_pattern, error = PatternError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct Pattern(String);
