//! `GeoJSON` geometries, the values of a `GeoProperty` (NGSI-LD clause 4.7.1, IETF RFC 7946).
//!
//! Clause 4.7.1 admits every RFC 7946 geometry except `GeometryCollection`. The geometry types here enforce the
//! structural rules of RFC 7946 section 3.1 on construction: a position has two or three numbers, a line string
//! at least two positions, a linear ring at least four positions with the last equal to the first, a polygon at
//! least one ring.
//!
//! A [`Shape`] is the bare shape; a [`Geometry`] is the RFC 7946 geometry object, a shape with an optional bounding box
//! (RFC 7946 section 5) and any foreign members (RFC 7946 section 6.1). The WGS 84 ranges of RFC 7946 section 4 are
//! checked when a position is built in code; a position read from a document only has to be finite, so a document keeps
//! what it says.
//!
//! # Geometries
//!
//! With the `geojson` feature, a [`Geometry`] converts to and from the `geojson` crate's geometry. Both directions keep
//! the altitude, the bounding box and the foreign members (RFC 7946 section 6.1), in the order the `geojson` map keeps,
//! which sorts them unless `serde_json` preserves order. Coordinates, and the numbers of a foreign member that are not
//! integers, pass through `f64`, and a value coming from `geojson` is held to the WGS 84 ranges as a value built in
//! code is.
//!
//! [`Geometry`], [`Shape`], [`Position`], [`Dimension`], [`LineString`], [`LinearRing`], [`Polygon`], [`BoundingBox`].
//!
//! # Foreign members
//!
//! [`ForeignMembers`], [`ForeignMemberName`], [`ForeignMemberNameError`], [`ForbiddenGeometryMember`].
//!
//! # Errors
//!
//! [`GeometryError`], [`GeometryMember`].

use crate::model::RawJson;
pub use crate::model::geometry::{
    bounding_box::BoundingBox,
    error::{GeometryError, GeometryMember},
    foreign_members::{ForbiddenGeometryMember, ForeignMemberName, ForeignMemberNameError, ForeignMembers},
    line_string::LineString,
    linear_ring::LinearRing,
    polygon::Polygon,
    position::{Dimension, Position},
};
use jstrict::Print;
use std::fmt::{self, Display};

mod bounding_box;
mod error;
mod foreign_members;
#[cfg(feature = "geo-types")]
mod geo_types_conversion;
#[cfg(feature = "geojson")]
mod geojson_conversion;
mod json;
mod line_string;
mod linear_ring;
mod polygon;
mod position;

/// The shape of a `GeoJSON` geometry NGSI-LD accepts (NGSI-LD clause 4.7.1): every RFC 7946 geometry except
/// `GeometryCollection`. The set is closed by the clause, so the enum is exhaustive. Its JSON form is read and
/// written by the `json` submodule.
///
/// A shape stands alone where a bounding box cannot be written: the reference geometry of a geoquery (clause
/// 5.2.13) and the origin of an ordering (clause 4.23.3). Everywhere else a [`Geometry`] holds it.
///
/// ```
/// use ngsi_ld::model::geometry::{Position, Shape};
///
/// let point = Shape::Point(Position::try_new(13.3986, 52.5547)?);
/// assert_eq!(
///     point.to_string(),
///     r#"{"type":"Point","coordinates":[13.3986,52.5547]}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::IsVariant, strum::IntoStaticStr)]
pub enum Shape {
    /// A single position.
    Point(Position),
    /// Several positions.
    MultiPoint(Vec<Position>),
    /// A line string.
    LineString(LineString),
    /// Several line strings.
    MultiLineString(Vec<LineString>),
    /// A polygon.
    Polygon(Polygon),
    /// Several polygons.
    MultiPolygon(Vec<Polygon>),
}

/// A `GeoJSON` geometry object (RFC 7946 section 3.1): a [`Shape`], an optional bounding box (RFC 7946 section 5)
/// of the shape's dimension, and the foreign members the object carries (RFC 7946 section 6.1), which are written
/// back as they were read. It is the value of a `GeoProperty`, the `location`, `observationSpace` and
/// `operationSpace` of a registration, and the `geometry` of a `Feature`.
///
/// ```
/// use ngsi_ld::model::geometry::{Geometry, Shape, BoundingBox, Position};
///
/// let point = Shape::Point(Position::try_new(13.3986, 52.5547)?);
/// let corner = Position::try_new(13.3986, 52.5547)?;
/// let geometry = Geometry::from(point).with_bbox(BoundingBox::try_new(corner.clone(), corner)?)?;
/// assert_eq!(
///     geometry.to_string(),
///     r#"{"type":"Point","coordinates":[13.3986,52.5547],"bbox":[13.3986,52.5547,13.3986,52.5547]}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    /// The geometry type and its coordinates.
    shape: Shape,
    /// Boxed: the two corners are larger than most shapes, and most geometries carry no box.
    bbox: Option<Box<BoundingBox>>,
    /// The foreign members, in document order.
    foreign_members: ForeignMembers,
}

impl Geometry {
    /// Returns the shape.
    #[must_use]
    pub const fn shape(&self) -> &Shape {
        &self.shape
    }

    /// Returns the bounding box, if the geometry has one.
    #[must_use]
    pub fn bbox(&self) -> Option<&BoundingBox> {
        self.bbox.as_deref()
    }

    /// Returns the foreign members.
    #[must_use]
    pub const fn foreign_members(&self) -> &ForeignMembers {
        &self.foreign_members
    }

    /// Returns the shape, dropping the bounding box and the foreign members.
    #[must_use]
    pub fn into_shape(self) -> Shape {
        self.shape
    }

    /// Sets the foreign member `name` (RFC 7946 section 6.1), keeping its place when it is already set.
    #[must_use]
    pub fn with_foreign_member(mut self, name: ForeignMemberName, value: RawJson) -> Self {
        self.foreign_members.insert(name, value);
        self
    }

    /// Gives the geometry a bounding box.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::BoundingBoxDimension`] when the box does not have the shape's dimension (RFC 7946
    /// section 5).
    pub fn with_bbox(self, bbox: BoundingBox) -> Result<Self, GeometryError> {
        let (geometry, bounding_box) = (self.shape.dimension(), bbox.dimension());
        if geometry == bounding_box {
            Ok(Self {
                bbox: Some(Box::new(bbox)),
                ..self
            })
        } else {
            Err(GeometryError::BoundingBoxDimension { geometry, bounding_box })
        }
    }
}

// No derive leaves the bounding box and the foreign members empty.
impl From<Shape> for Geometry {
    fn from(shape: Shape) -> Self {
        Self {
            shape,
            bbox: None,
            foreign_members: ForeignMembers::new(),
        }
    }
}

// The text form is the compact JSON form, which only exists once the geometry is turned into a JSON tree.
impl Display for Geometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_json().compact_print(), f)
    }
}

// Same as for `Geometry`.
impl Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_json().compact_print(), f)
    }
}

#[cfg(test)]
mod tests;
