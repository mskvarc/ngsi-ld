//! The parser of the `georel` production of NGSI-LD clause 4.10.

use crate::query::{
    Expected,
    ParseError,
    geoquery::{Distance, GeoRelation, NearDistance},
    grammar::{Failure, Input, Outcome, expecting},
};
use winnow::{
    Parser,
    combinator::{alt, cut_err, delimited, preceded},
    stream::Location,
    token::take_while,
};

/// Parses a geo-relationship.
pub(crate) fn relation(input: &mut Input<'_>) -> Outcome<GeoRelation> {
    expecting(
        Expected::GeoRelation,
        alt((
            preceded("near", cut_err(near_distance)).map(GeoRelation::Near),
            "within".value(GeoRelation::Within),
            "contains".value(GeoRelation::Contains),
            "intersects".value(GeoRelation::Intersects),
            "equals".value(GeoRelation::Equals),
            "disjoint".value(GeoRelation::Disjoint),
            "overlaps".value(GeoRelation::Overlaps),
        )),
    )
    .parse_next(input)
}

/// Parses the modifier and the distance after `near`: `;maxDistance==2000`.
fn near_distance(input: &mut Input<'_>) -> Outcome<NearDistance> {
    let modifier: fn(Distance) -> NearDistance = expecting(
        Expected::DistanceModifier,
        delimited(
            ';',
            alt((
                "maxDistance".value(NearDistance::Max as fn(Distance) -> NearDistance),
                "minDistance".value(NearDistance::Min as fn(Distance) -> NearDistance),
            )),
            "==",
        ),
    )
    .parse_next(input)?;
    distance.map(modifier).parse_next(input)
}

/// Parses a `PositiveNumber`, which runs up to the first character no JSON number holds.
fn distance(input: &mut Input<'_>) -> Outcome<Distance> {
    let position = input.current_token_start();
    let text = expecting(
        Expected::PositiveNumber,
        take_while(1.., |c: char| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E')),
    )
    .parse_next(input)?;
    text.parse().map_err(|source| Failure::fatal(ParseError::InvalidDistance { position, source }))
}
