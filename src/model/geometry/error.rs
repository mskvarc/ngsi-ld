//! Why data is not a `GeoJSON` geometry NGSI-LD accepts, and the members of a geometry object an error names.

use crate::model::{
    NotFinite,
    Number,
    geometry::{Dimension, ForeignMemberName, ForeignMemberNameError},
};

/// Why data is not a geometry NGSI-LD accepts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum GeometryError {
    /// The data is not a JSON object.
    #[error("a geometry must be a JSON object")]
    NotAnObject,
    /// A required member is missing.
    #[error("a geometry needs a `{member}` member")]
    MissingMember {
        /// The missing member.
        member: GeometryMember,
    },
    /// A member is written twice.
    #[error("a geometry has the `{member}` member twice")]
    DuplicateMember {
        /// The repeated member.
        member: GeometryMember,
    },
    /// A foreign member is written twice.
    #[error("a geometry has the foreign member `{name}` twice")]
    DuplicateForeignMember {
        /// The repeated member.
        name: ForeignMemberName,
    },
    /// A number of a foreign member cannot be converted for the `geojson` crate, whose numbers are integers or
    /// finite `f64`s.
    #[error("the foreign member `{name}` holds a number the geojson crate cannot hold")]
    ForeignMemberNumber {
        /// The member.
        name: ForeignMemberName,
    },
    /// A member is one a geometry object cannot have as a foreign member (RFC 7946 section 7.1).
    #[error("invalid foreign member")]
    ForeignMember {
        /// Why the member name is refused.
        #[from]
        source: ForeignMemberNameError,
    },
    /// The `type` member names no geometry type NGSI-LD accepts.
    #[error("not a geometry type NGSI-LD accepts")]
    UnknownType,
    /// The geometry is a `GeometryCollection`, which clause 4.7.1 excludes.
    #[error("GeometryCollection is not allowed")]
    GeometryCollection,
    /// A position is not an array of two or three numbers.
    #[error("a position must be an array of two or three numbers")]
    InvalidPosition,
    /// A longitude lies outside [-180, 180] (RFC 7946 section 4 places positions in WGS 84).
    #[error("longitude {value} lies outside [-180, 180]")]
    LongitudeOutOfRange {
        /// The longitude, as written.
        value: Number,
    },
    /// A latitude lies outside [-90, 90] (RFC 7946 section 4 places positions in WGS 84).
    #[error("latitude {value} lies outside [-90, 90]")]
    LatitudeOutOfRange {
        /// The latitude, as written.
        value: Number,
    },
    /// A geometry written as a string (clause 4.7.2) is not a JSON object.
    #[error("a geometry written as a string must hold a JSON object")]
    EncodedNotJson,
    /// A coordinate number cannot be written as JSON.
    #[error("invalid coordinate")]
    InvalidCoordinate {
        /// The number error.
        #[source]
        source: NotFinite,
    },
    /// The coordinates do not have the nesting the geometry type requires.
    #[error("the coordinates do not have the nesting the geometry type requires")]
    InvalidNesting,
    /// A line string or ring has too few positions.
    #[error("expected at least {minimum} positions, found {found}")]
    TooFewPositions {
        /// The minimum RFC 7946 requires.
        minimum: usize,
        /// The number found.
        found: usize,
    },
    /// A linear ring does not end where it starts.
    #[error("a linear ring must end at its first position")]
    RingNotClosed,
    /// A polygon has no ring.
    #[error("a polygon needs at least one ring")]
    EmptyPolygon,
    /// A `bbox` is not an array of four or six finite numbers (RFC 7946 section 5).
    #[error("a bounding box must be an array of four or six numbers")]
    InvalidBoundingBox,
    /// The corners of a bounding box have different dimensions.
    #[error("the corners of a bounding box must have the same dimension")]
    BoundingBoxCorners,
    /// A bounding box does not have the dimension of its geometry (RFC 7946 section 5).
    #[error("a {geometry}D geometry cannot have a {bounding_box}D bounding box")]
    BoundingBoxDimension {
        /// The dimension of the geometry.
        geometry: Dimension,
        /// The dimension of the bounding box.
        bounding_box: Dimension,
    },
    /// The southern edge of a bounding box lies north of its northern edge (RFC 7946 section 5.2).
    #[error("the southern latitude {south} of a bounding box lies north of its northern latitude {north}")]
    BoundingBoxLatitudes {
        /// The southern latitude, as written.
        south: Number,
        /// The northern latitude, as written.
        north: Number,
    },
}

/// A member of a `GeoJSON` geometry object that NGSI-LD accepts.
///
/// RFC 7946 section 3.1 defines `type` and `coordinates`, section 5 `bbox`, and a geometry carries nothing else.
///
/// The enum is exhaustive: RFC 7946 closes the set of members a geometry object defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum GeometryMember {
    /// `type`.
    #[strum(serialize = "type")]
    Type,
    /// `coordinates`.
    #[strum(serialize = "coordinates")]
    Coordinates,
    /// `bbox`.
    #[strum(serialize = "bbox")]
    Bbox,
}
