//! The structure of a regular expression (IEEE 1003.2, POSIX extended syntax), as far as NGSI-LD needs it: where
//! the groups, bracket expressions and escapes of an expression open and close.
//!
//! NGSI-LD writes regular expressions in two places. The `~=` and `!~=` operators of the query language (clause
//! 4.9) write one unquoted inside a query, where it ends at the first `;` or `|` outside any group and bracket
//! expression, or at a `)` that closes no group of the expression. The `idPattern` parameter and member (Table
//! 6.4.3.2-1, clause 5.2.33) hold a whole expression on their own, where `;`, `|` and an unmatched `)` are
//! ordinary characters of the expression (POSIX makes an unmatched `)` literal). The expressions are never
//! compiled: a broker evaluates them. They are only scanned, so that every printed expression reads back as the
//! same one.

use std::{iter::Peekable, str::CharIndices};

/// Why a string is not a well-formed regular expression.
///
/// A well-formed expression closes its groups, bracket expressions and escapes.
///
/// The enum is `#[non_exhaustive]`: a later check may find more.
///
/// ```
/// use ngsi_ld::query::{IdPattern, RegularExpressionError};
///
/// assert_eq!(
///     "(a|b".parse::<IdPattern>(),
///     Err(RegularExpressionError::UnclosedGroup)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum RegularExpressionError {
    /// The string is empty.
    #[error("a regular expression cannot be empty")]
    Empty,
    /// A group is opened and never closed.
    #[error("a group of the regular expression is never closed")]
    UnclosedGroup,
    /// A bracket expression is opened and never closed.
    #[error("a bracket expression of the regular expression is never closed")]
    UnclosedBracket,
    /// The expression ends with a lone `\`.
    #[error("the regular expression ends with a lone backslash")]
    DanglingEscape,
}

/// Where a scanned regular expression may end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Boundary {
    /// Inside a query (clause 4.9): at a top-level `;` or `|`, or at a `)` that closes no group.
    Query,
    /// At the end of the text only: the expression is a whole parameter or member value.
    Whole,
}

/// How far a regular expression extends, and in which state its scan ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Extent {
    /// The byte length of the expression.
    pub(crate) length: usize,
    /// The number of groups still open at the end.
    pub(crate) open_groups: usize,
    /// The state of the scan at the end.
    pub(crate) end: ScanEnd,
}

/// The state a scan of a regular expression ended in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanEnd {
    /// Outside any bracket expression or escape.
    Clean,
    /// Inside a bracket expression.
    InBracket,
    /// Right after a `\`.
    InEscape,
}

/// Whether a bracket expression was closed before the text ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bracket {
    /// Its `]` was found.
    Closed,
    /// The text ended inside it.
    Unclosed,
}

/// Consumes a bracket expression whose `[` was already consumed, returning whether it was closed.
///
/// In a POSIX bracket expression a `]` right after `[` or `[^` is literal, a `\` is literal, and the character
/// classes `[:alpha:]`, collating symbols `[.x.]` and equivalence classes `[=x=]` hold their own `]`.
fn skip_bracket(chars: &mut Peekable<CharIndices<'_>>) -> Bracket {
    chars.next_if(|&(_, c)| c == '^');
    chars.next_if(|&(_, c)| c == ']');
    while let Some((_, c)) = chars.next() {
        if c == ']' {
            return Bracket::Closed;
        }
        if c == '['
            && let Some((_, kind)) = chars.next_if(|&(_, next)| matches!(next, ':' | '.' | '='))
        {
            skip_class(chars, kind);
        }
    }
    Bracket::Unclosed
}

/// Consumes a character class, collating symbol or equivalence class up to its closing `kind` and `]`.
fn skip_class(chars: &mut Peekable<CharIndices<'_>>, kind: char) {
    while let Some((_, c)) = chars.next() {
        if c == kind && chars.next_if(|&(_, next)| next == ']').is_some() {
            return;
        }
    }
}

/// Scans the regular expression at the start of `text`, up to where `boundary` ends it.
pub(crate) fn extent(text: &str, boundary: Boundary) -> Extent {
    let mut chars = text.char_indices().peekable();
    let mut open_groups = 0_usize;
    while let Some((index, c)) = chars.next() {
        let ends_here = match boundary {
            // A `)` that closes no group ends the expression where the query goes on.
            Boundary::Query => matches!(c, ';' | '|' | ')') && open_groups == 0,
            Boundary::Whole => false,
        };
        if ends_here {
            return Extent {
                length: index,
                open_groups,
                end: ScanEnd::Clean,
            };
        }
        if c == ')' {
            // In a whole expression an unmatched `)` is an ordinary character (POSIX).
            open_groups = open_groups.saturating_sub(1);
        } else if c == '(' {
            open_groups = open_groups.saturating_add(1);
        } else if c == '\\' && chars.next().is_none() {
            // The escaped character is consumed with the backslash; the scan goes on when there is one.
            return Extent {
                length: text.len(),
                open_groups,
                end: ScanEnd::InEscape,
            };
        } else if c == '[' && skip_bracket(&mut chars) == Bracket::Unclosed {
            // The bracket expression is consumed up to its `]`; the scan goes on when it is closed.
            return Extent {
                length: text.len(),
                open_groups,
                end: ScanEnd::InBracket,
            };
        }
    }
    Extent {
        length: text.len(),
        open_groups,
        end: ScanEnd::Clean,
    }
}

/// Checks that `text`, scanned up to where `boundary` ends it, is a non-empty expression whose groups, bracket
/// expressions and escapes close, returning its extent.
pub(crate) fn check(text: &str, boundary: Boundary) -> Result<Extent, RegularExpressionError> {
    let scan = extent(text, boundary);
    match (scan.length == 0, scan.end, scan.open_groups) {
        (true, _, _) => Err(RegularExpressionError::Empty),
        (false, ScanEnd::InBracket, _) => Err(RegularExpressionError::UnclosedBracket),
        (false, ScanEnd::InEscape, _) => Err(RegularExpressionError::DanglingEscape),
        (false, ScanEnd::Clean, 0) => Ok(scan),
        (false, ScanEnd::Clean, _) => Err(RegularExpressionError::UnclosedGroup),
    }
}

#[cfg(test)]
mod tests {
    use crate::query::regular_expression::{Boundary, ScanEnd, extent};

    #[test]
    fn a_query_ends_the_expression_at_a_top_level_delimiter_and_a_whole_value_does_not() {
        assert_eq!(extent("^a$|^b$", Boundary::Query).length, 3);
        assert_eq!(extent("^a$|^b$", Boundary::Whole).length, 7);
        assert_eq!(extent("a(b|c);d", Boundary::Query).length, 6);
        assert_eq!(extent("a)b", Boundary::Query).length, 1);
        let whole = extent("a)b", Boundary::Whole);
        assert_eq!((whole.length, whole.open_groups, whole.end), (3, 0, ScanEnd::Clean));
    }

    #[test]
    fn brackets_escapes_and_groups_are_tracked_in_both_boundaries() {
        for boundary in [Boundary::Query, Boundary::Whole] {
            assert_eq!(extent("[|;)]x", boundary).length, 6);
            assert_eq!(extent("[a", boundary).end, ScanEnd::InBracket);
            assert_eq!(extent("a\\", boundary).end, ScanEnd::InEscape);
            assert_eq!(extent("(a", boundary).open_groups, 1);
            assert_eq!(extent("[[:alpha:]]", boundary).end, ScanEnd::Clean);
        }
    }
}
