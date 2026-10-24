//! The syntax the query languages share: the input they read, the error a parser fails with, the driver that
//! turns a failure into a [`ParseError`], the `AttrName` and `orOp` productions of clause 4.9 that clauses 4.17
//! and 4.21 reuse, and the printing of separated lists.

use crate::{
    model::Term,
    query::{Expected, ParseError},
};
use std::fmt;
use winnow::{
    LocatingSlice,
    ModalResult,
    Parser,
    error::{ErrMode, ParserError},
    stream::{Location, Stream},
    token::{one_of, take_while},
};

/// The input of every query-language parser: the query text, with byte offsets.
pub(crate) type Input<'i> = LocatingSlice<&'i str>;

/// The result of a query-language parser.
pub(crate) type Outcome<T> = ModalResult<T, Failure>;

/// Why a parser failed, as it bubbles up through the combinators.
#[derive(Debug)]
pub(crate) struct Failure {
    /// The byte offset of the failure.
    position: usize,
    /// What went wrong there.
    reason: Reason,
}

/// What went wrong where a [`Failure`] happened.
#[derive(Debug)]
enum Reason {
    /// A primitive parser did not match; the enclosing production says what was expected.
    Unknown,
    /// The grammar expected something else.
    Expected(Expected),
    /// The text matched the grammar but is not a valid value.
    Invalid(ParseError),
}

impl Failure {
    /// An error that stops the parse: the input matched a production but its content is invalid.
    pub(crate) const fn fatal(error: ParseError) -> ErrMode<Self> {
        ErrMode::Cut(Self {
            position: error.position(),
            reason: Reason::Invalid(error),
        })
    }

    /// A recoverable failure at `position`, where the grammar expected `expected`.
    pub(crate) const fn expected(expected: Expected, position: usize) -> ErrMode<Self> {
        ErrMode::Backtrack(Self {
            position,
            reason: Reason::Expected(expected),
        })
    }

    /// Turns the failure into the public error, naming `fallback` when no production said what it expected.
    fn into_parse_error(self, fallback: Expected) -> ParseError {
        match self.reason {
            Reason::Unknown => ParseError::Unexpected {
                expected: fallback,
                position: self.position,
            },
            Reason::Expected(expected) => ParseError::Unexpected {
                expected,
                position: self.position,
            },
            Reason::Invalid(error) => error,
        }
    }
}

impl<'i> ParserError<Input<'i>> for Failure {
    type Inner = Self;

    fn from_input(input: &Input<'i>) -> Self {
        Self {
            position: input.current_token_start(),
            reason: Reason::Unknown,
        }
    }

    // The default implementation panics in debug builds; a failed assertion is reported as a parse failure.
    fn assert(input: &Input<'i>, _message: &'static str) -> Self {
        Self::from_input(input)
    }

    // The alternative that got furthest into the input explains the failure best.
    fn or(self, other: Self) -> Self {
        if self.position > other.position { self } else { other }
    }

    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}

/// Runs `parser`, reporting a recoverable failure as "expected `expected`" at the position where it started.
///
/// An unrecoverable failure keeps its own, more precise, explanation.
pub(crate) fn expecting<'i, T>(expected: Expected, mut parser: impl Parser<Input<'i>, T, ErrMode<Failure>>) -> impl Parser<Input<'i>, T, ErrMode<Failure>> {
    move |input: &mut Input<'i>| {
        let position = input.current_token_start();
        parser.parse_next(input).map_err(|error| match error {
            ErrMode::Backtrack(_) => Failure::expected(expected, position),
            other @ (ErrMode::Cut(_) | ErrMode::Incomplete(_)) => other,
        })
    }
}

/// Parses the whole of `text` with `parser`.
///
/// A failure that no production explained reports `fallback`; input left over after `parser` reports
/// `trailing`, or an unbalanced parenthesis when it starts with `)`.
pub(crate) fn run<'i, T>(
    text: &'i str,
    mut parser: impl Parser<Input<'i>, T, ErrMode<Failure>>,
    fallback: Expected,
    trailing: Expected,
) -> Result<T, ParseError> {
    let mut input = LocatingSlice::new(text);
    let value = parser.parse_next(&mut input).map_err(|error| match error {
        ErrMode::Backtrack(failure) | ErrMode::Cut(failure) => failure.into_parse_error(fallback),
        ErrMode::Incomplete(_) => ParseError::Unexpected {
            expected: fallback,
            position: text.len(),
        },
    })?;
    let position = input.current_token_start();
    match input.peek_token() {
        None => Ok(value),
        Some(')') => Err(ParseError::UnbalancedParenthesis { position }),
        Some(_) => Err(ParseError::Unexpected { expected: trailing, position }),
    }
}

/// Whether `c` can appear in an `AttrName`: a letter, a number or `_` (clause 4.9, `TermChar`).
///
/// `char::is_alphanumeric` is slightly broader than the Unicode categories Letter and Number; [`Term`] checks
/// the exact categories once the name is delimited.
fn is_term_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Parses an `AttrName` (clause 4.9): `unicodeLetter *TermChar`.
pub(crate) fn term(input: &mut Input<'_>) -> Outcome<Term> {
    let position = input.current_token_start();
    let text = take_while(1.., is_term_char).parse_next(input)?;
    text.parse().map_err(|source| Failure::fatal(ParseError::InvalidName { position, source }))
}

/// Writes `items` separated by `separator`.
pub(crate) fn write_separated<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T], separator: char) -> fmt::Result {
    items
        .iter()
        .enumerate()
        .try_for_each(|(index, item)| if index == 0 { write!(f, "{item}") } else { write!(f, "{separator}{item}") })
}

/// Parses the `orOp` of clauses 4.17 and 4.21, which accept `,` as an alternative spelling of `|`.
pub(crate) fn list_separator(input: &mut Input<'_>) -> Outcome<char> {
    one_of(['|', ',']).parse_next(input)
}
