//! The NGSI-LD Geoquery Language (NGSI-LD clause 4.10), written in the `georel`, `geometry`, `coordinates` and
//! `geoproperty` parameters, and the `GeoQuery` object that carries the same four members in a body (clause 5.2.13).
//!
//! A geoquery tests a spatial relationship between a target geometry, the value of a `GeoProperty` of each entity
//! (`location` unless another is named), and a reference geometry. Only the relationship has a grammar: the
//! reference geometry is `GeoJSON` (IETF RFC 7946), the crate's own [`Shape`].
//!
//! ```
//! use ngsi_ld::{
//!     model::geometry::{Position, Shape},
//!     query::geoquery,
//! };
//!
//! let point = Shape::Point(Position::new(8.0, 40.0)?);
//! let near = geoquery::near_max(2000, point)
//!     .geoproperty("observationSpace")
//!     .build()?;
//! assert_eq!(near.relation().to_string(), "near;maxDistance==2000");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Geoqueries
//!
//! [`GeoQuery`], [`GeoRelation`], [`Distance`], [`NearDistance`], [`DistanceError`].
//!
//! # Start functions
//!
//! [`near_max`], [`near_min`], [`within`], [`contains`], [`intersects`], [`equals`], [`disjoint`], [`overlaps`].
//!
//! # JSON form
//!
//! [`GeoQueryMember`], [`GeoQueryError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, member},
};
pub use crate::query::geoquery::json::{GeoQueryError, GeoQueryMember};
use crate::{
    model::{AttributeName, Number, geometry::Shape, value::positive_number},
    query::{Expected, ParseError, Problem, build_error::Deferred, geoquery::builders::GeoQueryBuilder, grammar},
};
use jstrict::Value as Json;
use std::str::FromStr;

pub mod builders;
mod json;
mod parser;

/// Why a number is not the distance of a `near` relation (the `PositiveNumber` production of NGSI-LD clause 4.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum DistanceError {
    /// The text is not a JSON number (IETF RFC 8259, section 6).
    #[error("a distance must be a JSON number")]
    NotANumber,
    /// The number is zero or negative.
    #[error("a distance must be greater than zero")]
    NotPositive,
}

/// The distance of a `near` relation, in metres: a JSON number other than zero and without a minus sign (the
/// `PositiveNumber` production of NGSI-LD clause 4.10), kept in its lexical form.
///
/// ```
/// use ngsi_ld::{model::Number, query::geoquery::Distance};
///
/// let distance = Distance::try_from(Number::from(2000))?;
/// assert_eq!(distance.to_string(), "2000");
/// assert_eq!("1e3".parse::<Distance>()?.to_string(), "1e3");
/// assert!("0".parse::<Distance>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::AsRef, derive_more::Into)]
#[display("{_0}")]
pub struct Distance(Number);

impl Distance {
    /// Checks that `number` is greater than zero, by its lexical form.
    fn check(number: &Number) -> Result<(), DistanceError> {
        if positive_number::is_positive(number) {
            Ok(())
        } else {
            Err(DistanceError::NotPositive)
        }
    }

    /// Validates a distance given to a builder, keeping the number in the problem when it is refused.
    pub(crate) fn deferred(number: Number) -> Deferred<Self> {
        match Self::check(&number) {
            Ok(()) => Deferred::valid(Self(number)),
            Err(source) => Deferred::invalid(Problem::InvalidDistance { distance: number, source }),
        }
    }

    /// Returns the number of metres.
    #[must_use]
    pub const fn as_number(&self) -> &Number {
        &self.0
    }
}

impl TryFrom<Number> for Distance {
    type Error = DistanceError;

    fn try_from(number: Number) -> Result<Self, Self::Error> {
        Self::check(&number).map(|()| Self(number))
    }
}

/// Reads the lexical form of a JSON number; surrounding white space, which JSON would skip, is refused.
impl FromStr for Distance {
    type Err = DistanceError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let numeric = !text.is_empty()
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E'));
        match text.parse::<Json>() {
            Ok(Json::Number(number)) if numeric => Self::try_from(Number::from_json(number)),
            Ok(Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) | Json::Object(_)) | Err(_) => Err(DistanceError::NotANumber),
        }
    }
}

/// The distance modifier of a `near` relation (the `distance` production of NGSI-LD clause 4.10).
///
/// The enum is exhaustive: clause 4.10 defines these two modifiers.
///
/// ```
/// use ngsi_ld::{
///     model::Number,
///     query::geoquery::{Distance, NearDistance},
/// };
///
/// let within_two_km = NearDistance::Max(Distance::try_from(Number::from(2000))?);
/// assert_eq!(within_two_km.to_string(), "maxDistance==2000");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum NearDistance {
    /// `maxDistance`: the target geometry lies within the given distance of the reference geometry.
    #[display("maxDistance=={_0}")]
    Max(Distance),
    /// `minDistance`: the target geometry lies farther than the given distance from the reference geometry.
    #[display("minDistance=={_0}")]
    Min(Distance),
}

impl NearDistance {
    /// Returns the distance, whichever the modifier.
    #[must_use]
    pub const fn distance(&self) -> &Distance {
        match self {
            Self::Max(distance) | Self::Min(distance) => distance,
        }
    }
}

/// The spatial relationship a geoquery tests (the `georel` production of NGSI-LD clause 4.10), written in the
/// `georel` parameter.
///
/// The enum is exhaustive: clause 4.10 closes the set of relationships. A `near` relation always carries one
/// distance, and no other relation carries one.
///
/// ```
/// use ngsi_ld::query::geoquery::GeoRelation;
///
/// let near: GeoRelation = "near;minDistance==0.5".parse()?;
/// assert_eq!(near.to_string(), "near;minDistance==0.5");
/// assert_eq!("within".parse::<GeoRelation>()?, GeoRelation::Within);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum GeoRelation {
    /// `near`, within or beyond a distance of the reference geometry.
    #[display("near;{_0}")]
    Near(NearDistance),
    /// `within`: the target geometry lies within the reference geometry.
    #[display("within")]
    Within,
    /// `contains`: the target geometry contains the reference geometry.
    #[display("contains")]
    Contains,
    /// `intersects`: the target geometry intersects the reference geometry.
    #[display("intersects")]
    Intersects,
    /// `equals`: the target geometry equals the reference geometry.
    #[display("equals")]
    Equals,
    /// `disjoint`: the target geometry is disjoint from the reference geometry.
    #[display("disjoint")]
    Disjoint,
    /// `overlaps`: the target geometry overlaps the reference geometry.
    #[display("overlaps")]
    Overlaps,
}

// No derive parses the `georel` grammar.
impl FromStr for GeoRelation {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::relation, Expected::GeoRelation, Expected::EndOfInput)
    }
}

/// A geoquery (NGSI-LD clauses 4.10 and 5.2.13): a spatial relationship, the reference geometry, and the
/// `GeoProperty` it is applied to, `location` when none is named.
///
/// Built by [`near_max`] and the other starting functions, or from validated parts with [`GeoQuery::new`].
///
/// ```
/// use ngsi_ld::{
///     model::geometry::{Position, Shape},
///     query::geoquery::{GeoQuery, GeoRelation},
/// };
///
/// let query = GeoQuery::new(
///     GeoRelation::Intersects,
///     Shape::Point(Position::new(8.0, 40.0)?),
/// );
/// assert_eq!(query.relation(), &GeoRelation::Intersects);
/// assert!(query.geoproperty().is_none());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GeoQuery {
    /// `georel`.
    relation: GeoRelation,
    /// `geometry` and `coordinates`.
    geometry: Shape,
    /// `geoproperty`, when one is named.
    geoproperty: Option<AttributeName>,
}

impl GeoQuery {
    /// A geoquery testing `relation` against `geometry`, applied to `location`.
    #[must_use]
    pub const fn new(relation: GeoRelation, geometry: Shape) -> GeoQuery {
        GeoQuery {
            relation,
            geometry,
            geoproperty: None,
        }
    }

    /// Applies the geoquery to the `GeoProperty` `name` instead of `location`.
    #[must_use]
    pub fn with_geoproperty(self, name: AttributeName) -> Self {
        Self {
            geoproperty: Some(name),
            ..self
        }
    }

    /// Returns the spatial relationship (`georel`).
    #[must_use]
    pub const fn relation(&self) -> &GeoRelation {
        &self.relation
    }

    /// Returns the reference geometry (`geometry` and `coordinates`).
    #[must_use]
    pub const fn geometry(&self) -> &Shape {
        &self.geometry
    }

    /// Returns the `GeoProperty` the geoquery is applied to (`geoproperty`); `None` applies it to `location`.
    #[must_use]
    pub const fn geoproperty(&self) -> Option<&AttributeName> {
        self.geoproperty.as_ref()
    }
}

/// Starts a geoquery matching the entities within `metres` of `geometry` (`near;maxDistance==`).
///
/// `metres` is a [`Number`], or an integer; a decimal goes through [`Number::try_from`].
#[must_use]
pub fn near_max(metres: impl Into<Number>, geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(
        Distance::deferred(metres.into()).map(|distance| GeoRelation::Near(NearDistance::Max(distance))),
        geometry,
    )
}

/// Starts a geoquery matching the entities farther than `metres` from `geometry` (`near;minDistance==`).
#[must_use]
pub fn near_min(metres: impl Into<Number>, geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(
        Distance::deferred(metres.into()).map(|distance| GeoRelation::Near(NearDistance::Min(distance))),
        geometry,
    )
}

/// Starts a geoquery matching the entities whose target geometry lies within `geometry`.
#[must_use]
pub const fn within(geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(Deferred::valid(GeoRelation::Within), geometry)
}

/// Starts a geoquery matching the entities whose target geometry contains `geometry`.
#[must_use]
pub const fn contains(geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(Deferred::valid(GeoRelation::Contains), geometry)
}

/// Starts a geoquery matching the entities whose target geometry intersects `geometry`.
#[must_use]
pub const fn intersects(geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(Deferred::valid(GeoRelation::Intersects), geometry)
}

/// Starts a geoquery matching the entities whose target geometry equals `geometry`.
#[must_use]
pub const fn equals(geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(Deferred::valid(GeoRelation::Equals), geometry)
}

/// Starts a geoquery matching the entities whose target geometry is disjoint from `geometry`.
#[must_use]
pub const fn disjoint(geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(Deferred::valid(GeoRelation::Disjoint), geometry)
}

/// Starts a geoquery matching the entities whose target geometry overlaps `geometry`.
#[must_use]
pub const fn overlaps(geometry: Shape) -> GeoQueryBuilder {
    GeoQueryBuilder::new(Deferred::valid(GeoRelation::Overlaps), geometry)
}

#[cfg(feature = "linked-data")]
impl MapNames for GeoQuery {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            geoproperty: member(self.geoproperty, "geoproperty", mapping)?,
            ..self
        })
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// A geoquery carries no `@context`.
    impl for GeoQuery
);

#[cfg(test)]
mod tests;
