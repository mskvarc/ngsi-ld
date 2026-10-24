//! The parser of the NGSI-LD Scope Query Language (the ABNF of NGSI-LD clause 4.19).

use crate::query::{
    Expected,
    ParseError,
    grammar::{Failure, Input, Outcome, expecting, list_separator},
    scope_query::{ScopeAlternative, ScopeConjunction, ScopePattern, Selection},
};
use winnow::{
    Parser,
    combinator::{alt, cut_err, eof, preceded, repeat, terminated},
    error::ErrMode,
    stream::Location,
    token::take_till,
};

/// Whether `c` is an operator or a parenthesis of the scope query language.
const fn is_separator(c: char) -> bool {
    matches!(c, ',' | ';' | '|' | '(' | ')')
}

/// Parses a whole scope query: `/#` on its own, or alternatives separated by `|` or `,`.
pub(crate) fn scope_query(input: &mut Input<'_>) -> Outcome<Selection> {
    alt((terminated("/#", eof).value(Selection::AnyScope), alternatives.map(Selection::Alternatives))).parse_next(input)
}

/// Parses the alternatives.
fn alternatives(input: &mut Input<'_>) -> Outcome<Vec<ScopeAlternative>> {
    let first = alternative(input)?;
    let rest: Vec<ScopeAlternative> = repeat(0.., preceded(list_separator, cut_err(alternative))).parse_next(input)?;
    Ok([first].into_iter().chain(rest).collect())
}

/// Parses one alternative: a parenthesised conjunction, or a pattern optionally followed by `/#`.
fn alternative(input: &mut Input<'_>) -> Outcome<ScopeAlternative> {
    expecting(Expected::Scope, alt((conjunction, single))).parse_next(input)
}

/// Parses a parenthesised conjunction of patterns separated by `;`.
fn conjunction(input: &mut Input<'_>) -> Outcome<ScopeAlternative> {
    let position = input.current_token_start();
    '('.parse_next(input)?;
    let first = cut_err(expecting(Expected::Scope, pattern)).parse_next(input)?;
    let rest: Vec<ScopePattern> = repeat(0.., preceded(';', cut_err(expecting(Expected::Scope, pattern)))).parse_next(input)?;
    cut_err(')')
        .parse_next(input)
        .map_err(|_: ErrMode<Failure>| Failure::fatal(ParseError::UnbalancedParenthesis { position }))?;
    Ok(ScopeAlternative::AllOf(ScopeConjunction([first].into_iter().chain(rest).collect())))
}

/// Parses a pattern, with the whole tree below it when it ends with `/#`.
fn single(input: &mut Input<'_>) -> Outcome<ScopeAlternative> {
    let position = input.current_token_start();
    let text = take_till(1.., is_separator).parse_next(input)?;
    match text.strip_suffix("/#") {
        Some(pattern) => read_pattern(pattern, position).map(ScopeAlternative::Subtree),
        None => read_pattern(text, position).map(ScopeAlternative::Scope),
    }
}

/// Parses a pattern, which runs up to the next operator or parenthesis.
fn pattern(input: &mut Input<'_>) -> Outcome<ScopePattern> {
    let position = input.current_token_start();
    let text = take_till(1.., is_separator).parse_next(input)?;
    read_pattern(text, position)
}

/// Validates the pattern `text`, found at byte `position`.
fn read_pattern(text: &str, position: usize) -> Outcome<ScopePattern> {
    text.parse().map_err(|source| Failure::fatal(ParseError::InvalidScope { position, source }))
}
