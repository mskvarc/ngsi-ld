//! The parser of the NGSI-LD Entity Ordering Language (the ABNF of NGSI-LD clause 4.23.3).

use crate::query::{
    Expected,
    grammar::{Failure, Input, Outcome, expecting},
    ordering::{OrderBy, OrderingKey, OrderingTerm, SortDirection},
    q::parser::whole_value_path,
};
use std::str::FromStr;
use winnow::{
    Parser,
    combinator::{alt, cut_err, opt, preceded, repeat},
    stream::Location,
};

/// Parses an ordering: terms separated by `,`.
pub(crate) fn order_by(input: &mut Input<'_>) -> Outcome<OrderBy> {
    let first = ordering_term(input)?;
    let rest: Vec<OrderingTerm> = repeat(0.., preceded(',', cut_err(ordering_term))).parse_next(input)?;
    Ok(OrderBy([first].into_iter().chain(rest).collect()))
}

/// Parses one term: a value path (an entity member when it is one), optionally followed by `;` and a direction.
///
/// The ABNF names an `AttrName`; examples 4 and 5 of clause 4.23.3 sort by `address[city]` and `name.observedAt`,
/// so a term holds the whole value path of clause 4.9.
fn ordering_term(input: &mut Input<'_>) -> Outcome<OrderingTerm> {
    let key = expecting(Expected::OrderingKey, whole_value_path).map(OrderingKey::from).parse_next(input)?;
    let direction = opt(preceded(';', cut_err(expecting(Expected::SortDirection, direction)))).parse_next(input)?;
    Ok(direction.into_iter().fold(OrderingTerm::new(key), OrderingTerm::with_direction))
}

/// Parses a direction; the longer tokens are tried first, so `dist-asc` is not read as a prefix.
fn direction(input: &mut Input<'_>) -> Outcome<SortDirection> {
    let position = input.current_token_start();
    let token = alt(("dist-asc", "dist-desc", "asc", "desc")).parse_next(input)?;
    SortDirection::from_str(token).map_err(|_| Failure::expected(Expected::SortDirection, position))
}
