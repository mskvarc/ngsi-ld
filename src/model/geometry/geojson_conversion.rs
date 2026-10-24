//! Conversions between NGSI-LD geometries and the `geojson` crate (behind the `geojson` feature).

use crate::model::{
    Number,
    RawJson,
    geometry::{BoundingBox, ForeignMemberName, Geometry, GeometryError, LineString, LinearRing, Polygon, Position, Shape},
    nesting::grow,
};
use geojson::{Bbox, GeometryValue, JsonObject, JsonValue};
use jstrict::{NumberBuf, Value as Json};

// Written out: see the module documentation.
impl From<&Position> for geojson::Position {
    fn from(position: &Position) -> Self {
        let (longitude, latitude) = (position.longitude().as_f64(), position.latitude().as_f64());
        match position.altitude() {
            Some(altitude) => Self::from([longitude, latitude, altitude.as_f64()]),
            None => Self::from([longitude, latitude]),
        }
    }
}

// Written out: see the module documentation.
impl TryFrom<&geojson::Position> for Position {
    type Error = GeometryError;

    fn try_from(position: &geojson::Position) -> Result<Self, Self::Error> {
        match *position.as_slice() {
            [longitude, latitude] => Self::try_new(longitude, latitude),
            [longitude, latitude, altitude] => Self::with_altitude(longitude, latitude, altitude),
            _ => Err(GeometryError::InvalidPosition),
        }
    }
}

/// Converts `geojson` positions.
fn positions(positions: &[geojson::Position]) -> Result<Vec<Position>, GeometryError> {
    positions.iter().map(Position::try_from).collect()
}

/// Converts positions into `geojson` positions.
fn geojson_positions(positions: &[Position]) -> Vec<geojson::Position> {
    positions.iter().map(<geojson::Position>::from).collect()
}

/// Converts a polygon into `geojson` rings.
fn geojson_polygon(polygon: &Polygon) -> Vec<Vec<geojson::Position>> {
    polygon.rings().iter().map(|ring| geojson_positions(ring.positions())).collect()
}

/// Converts `geojson` rings into a polygon, holding them to RFC 7946.
fn polygon(rings: &[Vec<geojson::Position>]) -> Result<Polygon, GeometryError> {
    Polygon::try_new(rings.iter().map(|ring| LinearRing::try_new(positions(ring)?)).collect::<Result<_, _>>()?)
}

/// Converts a shape into a `geojson` geometry value.
fn geojson_value(shape: &Shape) -> GeometryValue {
    match shape {
        Shape::Point(position) => GeometryValue::Point { coordinates: position.into() },
        Shape::MultiPoint(points) => GeometryValue::MultiPoint {
            coordinates: geojson_positions(points),
        },
        Shape::LineString(line) => GeometryValue::LineString {
            coordinates: geojson_positions(line.positions()),
        },
        Shape::MultiLineString(lines) => GeometryValue::MultiLineString {
            coordinates: lines.iter().map(|line| geojson_positions(line.positions())).collect(),
        },
        Shape::Polygon(polygon) => GeometryValue::Polygon {
            coordinates: geojson_polygon(polygon),
        },
        Shape::MultiPolygon(polygons) => GeometryValue::MultiPolygon {
            coordinates: polygons.iter().map(geojson_polygon).collect(),
        },
    }
}

/// Converts a bounding box into the `geojson` array: every coordinate of the south-western corner, then every
/// coordinate of the north-eastern one (RFC 7946 section 5).
fn geojson_bbox(bbox: &BoundingBox) -> Bbox {
    bbox.south_west()
        .coordinates()
        .chain(bbox.north_east().coordinates())
        .map(Number::as_f64)
        .collect()
}

/// Converts a `geojson` bounding box: four or six numbers, each corner held to the WGS 84 ranges.
fn bbox(bbox: &[f64]) -> Result<BoundingBox, GeometryError> {
    let (south_west, north_east) = match *bbox {
        [west, south, east, north] => (Position::try_new(west, south)?, Position::try_new(east, north)?),
        [west, south, low, east, north, high] => (Position::with_altitude(west, south, low)?, Position::with_altitude(east, north, high)?),
        _ => return Err(GeometryError::InvalidBoundingBox),
    };
    BoundingBox::try_new(south_west, north_east)
}

/// Converts a `geojson` geometry value. A `GeometryCollection` is excluded by NGSI-LD clause 4.7.1.
fn shape(value: &GeometryValue) -> Result<Shape, GeometryError> {
    match value {
        GeometryValue::Point { coordinates } => Position::try_from(coordinates).map(Shape::Point),
        GeometryValue::MultiPoint { coordinates } => positions(coordinates).map(Shape::MultiPoint),
        GeometryValue::LineString { coordinates } => LineString::try_new(positions(coordinates)?).map(Shape::LineString),
        GeometryValue::MultiLineString { coordinates } => coordinates
            .iter()
            .map(|line| LineString::try_new(positions(line)?))
            .collect::<Result<_, _>>()
            .map(Shape::MultiLineString),
        GeometryValue::Polygon { coordinates } => polygon(coordinates).map(Shape::Polygon),
        GeometryValue::MultiPolygon { coordinates } => coordinates
            .iter()
            .map(|rings| polygon(rings))
            .collect::<Result<_, _>>()
            .map(Shape::MultiPolygon),
        GeometryValue::GeometryCollection { .. } => Err(GeometryError::GeometryCollection),
    }
}

/// Converts raw JSON into a `geojson` JSON value, each level on a stack that grows with the depth (see [`grow`]).
/// An integer in the range of an `i64` or a `u64` is kept exactly; any other number passes through `f64`, and one
/// beyond its range, which a `geojson` value cannot hold, makes the result `None`.
fn geojson_json(json: &Json) -> Option<JsonValue> {
    grow(|| match json {
        Json::Null => Some(JsonValue::Null),
        Json::Boolean(boolean) => Some(JsonValue::Bool(*boolean)),
        Json::Number(number) => match (number.as_i64(), number.as_u64()) {
            (Some(integer), _) => Some(JsonValue::from(integer)),
            (None, Some(integer)) => Some(JsonValue::from(integer)),
            (None, None) => {
                let float = number.as_f64_lossy();
                float.is_finite().then(|| JsonValue::from(float))
            }
        },
        Json::String(text) => Some(JsonValue::from(text.as_str())),
        Json::Array(items) => items.iter().map(geojson_json).collect::<Option<_>>().map(JsonValue::Array),
        Json::Object(object) => object
            .iter()
            .map(|entry| geojson_json(&entry.value).map(|value| (entry.key.as_str().to_owned(), value)))
            .collect::<Option<JsonObject>>()
            .map(JsonValue::Object),
    })
}

/// Converts a `geojson` JSON value into JSON, each level on a stack that grows with the depth (see [`grow`]). A number
/// that is neither an integer nor a finite `f64`, which only a `serde_json` built with arbitrary precision holds,
/// makes the result `None`.
fn json(value: &JsonValue) -> Option<Json> {
    grow(|| match value {
        JsonValue::Null => Some(Json::Null),
        JsonValue::Bool(boolean) => Some(Json::Boolean(*boolean)),
        JsonValue::Number(number) => match (number.as_i64(), number.as_u64(), number.as_f64()) {
            (Some(integer), _, _) => Some(Json::Number(NumberBuf::from(integer))),
            (None, Some(integer), _) => Some(Json::Number(NumberBuf::from(integer))),
            (None, None, Some(float)) => NumberBuf::try_from(float).ok().map(Json::Number),
            (None, None, None) => None,
        },
        JsonValue::String(text) => Some(Json::from(text.as_str())),
        JsonValue::Array(items) => items.iter().map(json).collect::<Option<_>>().map(Json::Array),
        JsonValue::Object(members) => members
            .iter()
            .map(|(key, member)| json(member).map(|member| (key.as_str().into(), member)))
            .collect::<Option<_>>()
            .map(Json::Object),
    })
}

/// Converts to a `geojson` geometry, keeping the bounding box and the foreign members.
impl TryFrom<&Geometry> for geojson::Geometry {
    type Error = GeometryError;

    fn try_from(geometry: &Geometry) -> Result<Self, Self::Error> {
        let mut converted = Self::new(geojson_value(geometry.shape()));
        converted.bbox = geometry.bbox().map(geojson_bbox);
        if !geometry.foreign_members().is_empty() {
            let members = geometry
                .foreign_members()
                .iter()
                .map(|(name, value)| {
                    geojson_json(&value.to_json())
                        .map(|value| (name.as_ref().to_owned(), value))
                        // Clone: the error owns the name, while the geometry keeps it.
                        .ok_or_else(|| GeometryError::ForeignMemberNumber { name: name.clone() })
                })
                .collect::<Result<JsonObject, _>>()?;
            converted.foreign_members = Some(members);
        }
        Ok(converted)
    }
}

/// Converts from a `geojson` geometry, keeping its foreign members. A `GeometryCollection` (excluded by NGSI-LD
/// clause 4.7.1) is rejected.
impl TryFrom<&geojson::Geometry> for Geometry {
    type Error = GeometryError;

    fn try_from(geometry: &geojson::Geometry) -> Result<Self, Self::Error> {
        let mut converted = Self::from(shape(&geometry.value)?);
        for (key, value) in geometry.foreign_members.iter().flatten() {
            let name = ForeignMemberName::try_new(key.as_str())?;
            let Some(value) = json(value) else {
                return Err(GeometryError::ForeignMemberNumber { name });
            };
            converted = converted.with_foreign_member(name, RawJson::from_json(value));
        }
        match &geometry.bbox {
            Some(numbers) => converted.with_bbox(bbox(numbers)?),
            None => Ok(converted),
        }
    }
}
