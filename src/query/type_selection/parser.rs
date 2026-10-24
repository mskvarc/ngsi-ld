//! The parser of the NGSI-LD Entity Type Selection Language (the ABNF of NGSI-LD clause 4.17).

use crate::query::{
    Expected,
    ParseError,
    grammar::{Failure, Input, Outcome, expecting, list_separator},
    type_selection::{SelectableType, is_separator},
};
use winnow::{
    Parser,
    combinator::{alt, cut_err, preceded, repeat},
    error::ErrMode,
    stream::Location,
    token::take_till,
};

/// Parses the alternatives of a selection, separated by `|` or `,`.
pub(crate) fn selection(input: &mut Input<'_>) -> Outcome<Vec<Vec<SelectableType>>> {
    let first = alternative(input)?;
    let rest: Vec<Vec<SelectableType>> = repeat(0.., preceded(list_separator, cut_err(alternative))).parse_next(input)?;
    Ok([first].into_iter().chain(rest).collect())
}

/// Parses one alternative: a parenthesised conjunction or a single entity type.
fn alternative(input: &mut Input<'_>) -> Outcome<Vec<SelectableType>> {
    expecting(Expected::EntityType, alt((conjunction, entity_type.map(|selected| vec![selected])))).parse_next(input)
}

/// Parses a parenthesised conjunction of entity types separated by `;`.
fn conjunction(input: &mut Input<'_>) -> Outcome<Vec<SelectableType>> {
    let position = input.current_token_start();
    '('.parse_next(input)?;
    let first = cut_err(expecting(Expected::EntityType, entity_type)).parse_next(input)?;
    let rest: Vec<SelectableType> = repeat(0.., preceded(';', cut_err(expecting(Expected::EntityType, entity_type)))).parse_next(input)?;
    cut_err(')')
        .parse_next(input)
        .map_err(|_: ErrMode<Failure>| Failure::fatal(ParseError::UnbalancedParenthesis { position }))?;
    Ok([first].into_iter().chain(rest).collect())
}

/// Parses an entity type, which runs up to the next operator or parenthesis.
fn entity_type(input: &mut Input<'_>) -> Outcome<SelectableType> {
    let position = input.current_token_start();
    let text = take_till(1.., is_separator).parse_next(input)?;
    text.parse()
        .map_err(|source| Failure::fatal(ParseError::InvalidEntityType { position, source }))
}
