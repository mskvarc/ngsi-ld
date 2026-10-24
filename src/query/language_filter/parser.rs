//! The parser of the NGSI-LD Language Filter (NGSI-LD clause 4.15).

use crate::query::{
    Expected,
    ParseError,
    Quality,
    Weighted,
    grammar::{Failure, Input, Outcome, expecting},
    language_filter::{LanguageFilter, LanguageRange, is_wildcard},
};
use winnow::{
    Parser,
    combinator::{cut_err, opt, preceded, repeat},
    stream::Location,
    token::{one_of, take_till, take_while},
};

/// Parses a filter: preferences separated by `,`.
pub(crate) fn filter(input: &mut Input<'_>) -> Outcome<LanguageFilter> {
    let first = preference(input)?;
    let rest: Vec<Weighted<LanguageRange>> = repeat(0.., preceded(',', cut_err(preference))).parse_next(input)?;
    Ok(LanguageFilter([first].into_iter().chain(rest).collect()))
}

/// Parses one preference: a language range, optionally followed by `;q=` and a quality value.
fn preference(input: &mut Input<'_>) -> Outcome<Weighted<LanguageRange>> {
    let range = expecting(Expected::LanguageRange, range).parse_next(input)?;
    let quality = opt(preceded(';', cut_err(expecting(Expected::Quality, weight)))).parse_next(input)?;
    Ok(Weighted::from_parts(range, quality))
}

/// Parses a language range, which runs up to the next `,` or `;`.
fn range(input: &mut Input<'_>) -> Outcome<LanguageRange> {
    let position = input.current_token_start();
    let text = take_till(1.., [',', ';']).parse_next(input)?;
    if is_wildcard(text) {
        return Ok(LanguageRange::Any);
    }
    text.parse()
        .map(LanguageRange::Tag)
        .map_err(|source| Failure::fatal(ParseError::InvalidLanguageTag { position, source }))
}

/// Parses `q=` and a quality value; RFC 9110 section 12.4.2 makes the `q` case-insensitive.
fn weight(input: &mut Input<'_>) -> Outcome<Quality> {
    (one_of(['q', 'Q']), '=').parse_next(input)?;
    let position = input.current_token_start();
    let text = take_while(1.., |c: char| c.is_ascii_digit() || c == '.').parse_next(input)?;
    text.parse().map_err(|source| Failure::fatal(ParseError::InvalidQuality { position, source }))
}
