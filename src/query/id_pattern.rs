//! The regular expression entity ids are matched against (the `idPattern` parameter of Tables 6.4.3.2-1, 6.4.3.3-1
//! and 6.18.3.2-1, and the `idPattern` member of the `EntitySelector` of NGSI-LD clause 5.2.33).

use crate::query::{
    RegularExpressionError,
    regular_expression::{Boundary, check},
};

/// Validates that `value` is a non-empty regular expression whose groups, bracket expressions and escapes close.
fn validate_id_pattern(value: &str) -> Result<(), RegularExpressionError> {
    check(value, Boundary::Whole).map(|_| ())
}

/// A regular expression (IEEE 1003.2, POSIX extended syntax) entity ids are matched against: the `idPattern`
/// parameter (Table 6.4.3.2-1) and the `idPattern` member of an `EntitySelector` (clause 5.2.33).
///
/// An id pattern is a whole value of its own, so unlike a [`Pattern`](crate::query::q::Pattern) of the
/// query language (clause 4.9), which a `;`, a `|` or an unmatched `)` would end inside a query, it may hold a
/// top-level alternation such as `^a$|^b$`. The expression is not compiled, since a broker evaluates it: it is only
/// checked to be non-empty and to close its groups, bracket expressions and escapes.
///
/// ```
/// use ngsi_ld::query::IdPattern;
///
/// let either: IdPattern = "^urn:ngsi-ld:Vehicle:A.*$|^urn:ngsi-ld:Vehicle:B.*$".parse()?;
/// assert_eq!(
///     either.to_string(),
///     "^urn:ngsi-ld:Vehicle:A.*$|^urn:ngsi-ld:Vehicle:B.*$"
/// );
/// assert!("(unclosed".parse::<IdPattern>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_id_pattern, error = RegularExpressionError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct IdPattern(String);

#[cfg(test)]
mod tests {
    use crate::query::{IdPattern, RegularExpressionError};
    use pretty_assertions::assert_eq;

    #[test]
    fn a_top_level_alternation_and_an_unmatched_closing_parenthesis_are_part_of_the_pattern() {
        for text in ["^a$|^b$", "a;b", "a)b", "^urn:ngsi-ld:Vehicle:(A|B)[0-9]+$", "[[:digit:]]+"] {
            let pattern: IdPattern = text.parse().unwrap();
            assert_eq!(pattern.to_string(), text);
            assert_eq!(pattern.as_ref(), text);
        }
    }

    #[test]
    fn an_empty_or_unclosed_pattern_is_refused() {
        assert_eq!("".parse::<IdPattern>(), Err(RegularExpressionError::Empty));
        assert_eq!("(a|b".parse::<IdPattern>(), Err(RegularExpressionError::UnclosedGroup));
        assert_eq!("[ab".parse::<IdPattern>(), Err(RegularExpressionError::UnclosedBracket));
        assert_eq!("ab\\".parse::<IdPattern>(), Err(RegularExpressionError::DanglingEscape));
    }
}
