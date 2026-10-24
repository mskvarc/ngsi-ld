//! The JSON form of a geometry: a `GeoJSON` geometry object (IETF RFC 7946 section 3.1), or such an object
//! written as a string, which clause 4.7.2 requires a system to accept.

use crate::model::{
    RawJson,
    geometry::{
        BoundingBox,
        Dimension,
        ForeignMemberName,
        ForeignMembers,
        Geometry,
        GeometryError,
        GeometryMember,
        LineString,
        LinearRing,
        Polygon,
        Position,
        Shape,
    },
};
use jstrict::{Object, Value as Json};

/// The `type` of a `GeoJSON` geometry object (RFC 7946 section 1.4). RFC 7946 closes the set; NGSI-LD refuses
/// the last one (clause 4.7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
enum GeometryType {
    /// `Point`.
    Point,
    /// `MultiPoint`.
    MultiPoint,
    /// `LineString`.
    LineString,
    /// `MultiLineString`.
    MultiLineString,
    /// `Polygon`.
    Polygon,
    /// `MultiPolygon`.
    MultiPolygon,
    /// `GeometryCollection`, which clause 4.7.1 excludes.
    GeometryCollection,
}

/// The members of a geometry object, each read at most once.
#[derive(Default)]
struct Members<'a> {
    /// `type`.
    geometry_type: Option<&'a Json>,
    /// `coordinates`.
    coordinates: Option<&'a Json>,
    /// `bbox`.
    bbox: Option<&'a Json>,
    /// The foreign members (RFC 7946 section 6.1).
    foreign: ForeignMembers,
}

impl<'a> Members<'a> {
    /// Sorts the members of `object`, refusing a member twice and a member RFC 7946 section 7.1 forbids; every
    /// member RFC 7946 does not define is a foreign member.
    fn of(object: &'a Object) -> Result<Self, GeometryError> {
        let mut members = Self::default();
        for entry in object {
            let Ok(member) = entry.key.as_str().parse::<GeometryMember>() else {
                let name = ForeignMemberName::try_new(entry.key.as_str())?;
                if members.foreign.get(&name).is_some() {
                    return Err(GeometryError::DuplicateForeignMember { name });
                }
                members.foreign.insert(name, RawJson::copy_of(&entry.value));
                continue;
            };
            let slot = match member {
                GeometryMember::Type => &mut members.geometry_type,
                GeometryMember::Coordinates => &mut members.coordinates,
                GeometryMember::Bbox => &mut members.bbox,
            };
            if slot.replace(&entry.value).is_some() {
                return Err(GeometryError::DuplicateMember { member });
            }
        }
        Ok(members)
    }
}

impl Geometry {
    /// Reads a geometry object, or a geometry written as a JSON string, which clause 4.7.2 requires a system to
    /// accept when the string parses into a geometry object.
    pub(crate) fn from_json(json: &Json) -> Result<Self, GeometryError> {
        match json {
            Json::String(encoded) => Self::from_object(&encoded.as_str().parse::<Json>().map_err(|_| GeometryError::EncodedNotJson)?),
            Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_) => Self::from_object(json),
        }
    }

    /// Reads a geometry object.
    fn from_object(json: &Json) -> Result<Self, GeometryError> {
        let members = Members::of(json.as_object().ok_or(GeometryError::NotAnObject)?)?;
        let geometry_type = members
            .geometry_type
            .ok_or(GeometryError::MissingMember { member: GeometryMember::Type })?
            .as_str()
            .and_then(|name| name.parse::<GeometryType>().ok())
            .ok_or(GeometryError::UnknownType)?;
        let coordinates = members.coordinates.ok_or(GeometryError::MissingMember {
            member: GeometryMember::Coordinates,
        });
        let geometry = Self {
            foreign_members: members.foreign,
            ..Self::from(Shape::with_type(geometry_type, coordinates)?)
        };
        match members.bbox {
            Some(bbox) => geometry.with_bbox(BoundingBox::from_json(bbox)?),
            None => Ok(geometry),
        }
    }

    /// Writes the geometry object: `type`, `coordinates`, `bbox` if the geometry has one, then the foreign members
    /// in their order.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = self.shape.to_object();
        if let Some(bbox) = &self.bbox {
            object.push(<&str>::from(GeometryMember::Bbox).into(), bbox.to_json());
        }
        for (name, value) in self.foreign_members.iter() {
            object.push(name.as_ref().into(), value.to_json());
        }
        Json::Object(object)
    }
}

impl Shape {
    /// Reads a shape from its `type` and its `coordinates` given apart, as a geoquery carries them (clauses 4.10
    /// and 5.2.13).
    pub(crate) fn from_parts(type_name: &str, coordinates: &Json) -> Result<Self, GeometryError> {
        let geometry_type = type_name.parse::<GeometryType>().map_err(|_| GeometryError::UnknownType)?;
        Self::with_type(geometry_type, Ok(coordinates))
    }

    /// Reads the coordinates of a shape of type `geometry_type`.
    fn with_type(geometry_type: GeometryType, coordinates: Result<&Json, GeometryError>) -> Result<Self, GeometryError> {
        match geometry_type {
            GeometryType::Point => Position::from_json(coordinates?).map(Self::Point),
            GeometryType::MultiPoint => list(coordinates?, Position::from_json).map(Self::MultiPoint),
            GeometryType::LineString => line_string(coordinates?).map(Self::LineString),
            GeometryType::MultiLineString => list(coordinates?, line_string).map(Self::MultiLineString),
            GeometryType::Polygon => polygon(coordinates?).map(Self::Polygon),
            GeometryType::MultiPolygon => list(coordinates?, polygon).map(Self::MultiPolygon),
            GeometryType::GeometryCollection => Err(GeometryError::GeometryCollection),
        }
    }

    /// The `type` of the geometry object.
    const fn geometry_type(&self) -> GeometryType {
        match self {
            Self::Point(_) => GeometryType::Point,
            Self::MultiPoint(_) => GeometryType::MultiPoint,
            Self::LineString(_) => GeometryType::LineString,
            Self::MultiLineString(_) => GeometryType::MultiLineString,
            Self::Polygon(_) => GeometryType::Polygon,
            Self::MultiPolygon(_) => GeometryType::MultiPolygon,
        }
    }

    /// Returns the dimension of the shape: three when any position has an altitude, otherwise two. RFC 7946
    /// section 5 gives a bounding box this dimension.
    #[must_use]
    pub fn dimension(&self) -> Dimension {
        let any_altitude = |positions: &[Position]| positions.iter().any(|position| position.altitude().is_some());
        let polygon_altitude = |polygon: &Polygon| polygon.rings().iter().any(|ring| any_altitude(ring.positions()));
        let three = match self {
            Self::Point(position) => position.altitude().is_some(),
            Self::MultiPoint(positions) => any_altitude(positions),
            Self::LineString(line) => any_altitude(line.positions()),
            Self::MultiLineString(lines) => lines.iter().any(|line| any_altitude(line.positions())),
            Self::Polygon(polygon) => polygon_altitude(polygon),
            Self::MultiPolygon(polygons) => polygons.iter().any(polygon_altitude),
        };
        if three { Dimension::Three } else { Dimension::Two }
    }

    /// Writes the `coordinates` member alone, as the geoquery and ordering parameters carry it (clauses 4.10 and
    /// 4.23.3).
    pub(crate) fn coordinates_json(&self) -> Json {
        match self {
            Self::Point(position) => position.to_json(),
            Self::MultiPoint(positions) => positions_json(positions),
            Self::LineString(line) => positions_json(line.positions()),
            Self::MultiLineString(lines) => Json::Array(lines.iter().map(|line| positions_json(line.positions())).collect()),
            Self::Polygon(polygon) => polygon_json(polygon),
            Self::MultiPolygon(polygons) => Json::Array(polygons.iter().map(polygon_json).collect()),
        }
    }

    /// The `type` of the geometry object.
    pub(crate) fn type_name(&self) -> &'static str {
        self.geometry_type().into()
    }

    /// Writes the members of the geometry object, `type` first.
    fn to_object(&self) -> Object {
        let mut object = Object::new();
        object.push(<&str>::from(GeometryMember::Type).into(), Json::from(self.type_name()));
        object.push(<&str>::from(GeometryMember::Coordinates).into(), self.coordinates_json());
        object
    }

    /// Writes the geometry object of the shape alone.
    pub(crate) fn to_json(&self) -> Json {
        Json::Object(self.to_object())
    }
}

/// Reads an array whose items `item` reads.
fn list<T>(json: &Json, item: fn(&Json) -> Result<T, GeometryError>) -> Result<Vec<T>, GeometryError> {
    json.as_array().ok_or(GeometryError::InvalidNesting)?.iter().map(item).collect()
}

/// Reads a line string's coordinates.
fn line_string(json: &Json) -> Result<LineString, GeometryError> {
    LineString::try_new(list(json, Position::from_json)?)
}

/// Reads a polygon's coordinates.
fn polygon(json: &Json) -> Result<Polygon, GeometryError> {
    Polygon::try_new(list(json, |ring| LinearRing::try_new(list(ring, Position::from_json)?))?)
}

/// Writes positions as a JSON array.
fn positions_json(positions: &[Position]) -> Json {
    Json::Array(positions.iter().map(Position::to_json).collect())
}

/// Writes a polygon's rings as a JSON array.
fn polygon_json(polygon: &Polygon) -> Json {
    Json::Array(polygon.rings().iter().map(|ring| positions_json(ring.positions())).collect())
}
