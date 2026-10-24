//! The parser of the NGSI-LD Attribute Projection Language (the ABNF of NGSI-LD clause 4.21).
//!
//! A linked projection is read on a stack that grows with the depth ([`grow`]), so a projection nested any number
//! of levels deep is read without overflowing the stack.

use crate::{
    model::nesting::grow,
    query::{
        Expected,
        grammar::{Input, Outcome, expecting, list_separator, term},
        projection::{Projection, ProjectionTerm},
    },
};
use winnow::{
    Parser,
    combinator::{cut_err, opt, preceded, repeat},
};

/// Parses a projection: terms separated by `,` or `|`.
pub(crate) fn projection(input: &mut Input<'_>) -> Outcome<Projection> {
    let first = projection_term(input)?;
    let rest: Vec<ProjectionTerm> = repeat(0.., preceded(list_separator, cut_err(projection_term))).parse_next(input)?;
    Ok(Projection([first].into_iter().chain(rest).collect()))
}

/// Parses one term: a name, followed for an attribute by an optional linked projection in braces.
fn projection_term(input: &mut Input<'_>) -> Outcome<ProjectionTerm> {
    let parsed = expecting(Expected::ProjectionTerm, term).map(ProjectionTerm::from).parse_next(input)?;
    let ProjectionTerm::Attribute(attribute) = parsed else {
        return Ok(parsed);
    };
    if opt('{').parse_next(input)?.is_none() {
        return Ok(ProjectionTerm::Attribute(attribute));
    }
    let linked = cut_err(|input: &mut Input<'_>| grow(|| projection(input))).parse_next(input)?;
    cut_err(expecting(Expected::ClosingBrace, '}')).parse_next(input)?;
    Ok(ProjectionTerm::Attribute(attribute.with_linked(linked)))
}
