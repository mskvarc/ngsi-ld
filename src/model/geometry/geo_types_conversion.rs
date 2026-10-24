//! Conversions between NGSI-LD geometries and `geo-types` (behind the `geo-types` feature), so a caller can hand
//! a geometry to the georust algorithms and back.

use crate::model::geometry::{GeometryError, LineString, LinearRing, Polygon, Position, Shape};
use geo_types::{Coord, Geometry as GeoTypesGeometry, LineString as GeoTypesLineString, Polygon as GeoTypesPolygon};
use std::iter;

/// `geo-types` coordinates are two-dimensional, so the altitude of a position is dropped. No derive converts
/// between two crates' geometries, so every conversion here is written out.
impl From<&Position> for Coord {
    fn from(position: &Position) -> Self {
        Self {
            x: position.longitude().as_f64(),
            y: position.latitude().as_f64(),
        }
    }
}

// Written out: see the first conversion of this module.
impl TryFrom<Coord> for Position {
    type Error = GeometryError;

    fn try_from(coord: Coord) -> Result<Self, Self::Error> {
        Self::new(coord.x, coord.y)
    }
}

/// Converts a `geo-types` line string's coordinates into positions.
fn positions(line: geo_types::LineString) -> Result<Vec<Position>, GeometryError> {
    line.into_iter().map(Position::try_from).collect()
}

/// Converts positions into a `geo-types` line string.
fn geo_line(positions: &[Position]) -> geo_types::LineString {
    positions.iter().map(Coord::from).collect()
}

/// Converts a polygon into a `geo-types` polygon; its rings are already closed.
fn geo_polygon(polygon: &Polygon) -> geo_types::Polygon {
    let mut rings = polygon.rings().iter().map(|ring| geo_line(ring.positions()));
    let exterior = rings.next().unwrap_or_else(|| GeoTypesLineString::new(Vec::new()));
    GeoTypesPolygon::new(exterior, rings.collect())
}

/// Converts a `geo-types` polygon, holding its rings to RFC 7946 (closed, at least four positions).
fn polygon(polygon: geo_types::Polygon) -> Result<Polygon, GeometryError> {
    let (exterior, interiors) = polygon.into_inner();
    Polygon::new(
        iter::once(exterior)
            .chain(interiors)
            .map(|ring| LinearRing::new(positions(ring)?))
            .collect::<Result<_, _>>()?,
    )
}

/// Converts into a `geo-types` geometry. Altitudes are dropped, because `geo-types` is two-dimensional; it has no
/// bounding box either, so the conversion is the shape's.
impl From<&Shape> for GeoTypesGeometry {
    fn from(shape: &Shape) -> Self {
        match shape {
            Shape::Point(position) => Self::Point(geo_types::Point(position.into())),
            Shape::MultiPoint(points) => Self::MultiPoint(points.iter().map(|point| geo_types::Point(point.into())).collect()),
            Shape::LineString(line) => Self::LineString(geo_line(line.positions())),
            Shape::MultiLineString(lines) => Self::MultiLineString(lines.iter().map(|line| geo_line(line.positions())).collect()),
            Shape::Polygon(polygon) => Self::Polygon(geo_polygon(polygon)),
            Shape::MultiPolygon(polygons) => Self::MultiPolygon(polygons.iter().map(geo_polygon).collect()),
        }
    }
}

/// Converts from a `geo-types` geometry. `GeometryCollection`, which NGSI-LD clause 4.7.1 excludes, and `Line`,
/// `Rect` and `Triangle`, which `GeoJSON` does not define, are rejected.
impl TryFrom<GeoTypesGeometry> for Shape {
    type Error = GeometryError;

    fn try_from(geometry: GeoTypesGeometry) -> Result<Self, Self::Error> {
        match geometry {
            GeoTypesGeometry::Point(point) => Position::try_from(point.0).map(Self::Point),
            GeoTypesGeometry::MultiPoint(points) => points
                .into_iter()
                .map(|point| Position::try_from(point.0))
                .collect::<Result<_, _>>()
                .map(Self::MultiPoint),
            GeoTypesGeometry::LineString(line) => LineString::new(positions(line)?).map(Self::LineString),
            GeoTypesGeometry::MultiLineString(lines) => lines
                .into_iter()
                .map(|line| LineString::new(positions(line)?))
                .collect::<Result<_, _>>()
                .map(Self::MultiLineString),
            GeoTypesGeometry::Polygon(shape) => polygon(shape).map(Self::Polygon),
            GeoTypesGeometry::MultiPolygon(shapes) => shapes.into_iter().map(polygon).collect::<Result<_, _>>().map(Self::MultiPolygon),
            GeoTypesGeometry::GeometryCollection(_) => Err(GeometryError::GeometryCollection),
            GeoTypesGeometry::Line(_) | GeoTypesGeometry::Rect(_) | GeoTypesGeometry::Triangle(_) => Err(GeometryError::UnknownType),
        }
    }
}
