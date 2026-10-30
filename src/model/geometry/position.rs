//! A position of a `GeoJSON` geometry (IETF RFC 7946 section 3.1.1) and its JSON form.

use crate::model::{NotFinite, Number, geometry::GeometryError};
use jstrict::{Print, Value as Json};
use std::{
    cmp::Ordering,
    fmt::{self, Display},
};

/// The number of coordinates of a position: two, or three with an altitude.
///
/// RFC 7946 section 3.1.1.
///
/// The enum is exhaustive: RFC 7946 refuses a position with more elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display)]
pub enum Dimension {
    /// Longitude and latitude.
    #[strum(serialize = "2")]
    Two,
    /// Longitude, latitude and altitude.
    #[strum(serialize = "3")]
    Three,
}

/// A position: longitude, latitude and an optional altitude.
///
/// RFC 7946 section 3.1.1.
///
/// ```
/// use ngsi_ld::model::geometry::Position;
///
/// let position = Position::try_new(13.3986, 52.5547)?;
/// assert_eq!(position.to_string(), "[13.3986,52.5547]");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Position {
    /// The longitude in decimal degrees, the first coordinate.
    longitude: Number,
    /// The latitude in decimal degrees, the second coordinate.
    latitude: Number,
    /// The altitude, the optional third coordinate; `None` for a position of [`Dimension::Two`].
    altitude: Option<Number>,
}

impl Position {
    /// A two-dimensional position.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidCoordinate`] when a coordinate is NaN or infinite, and
    /// [`GeometryError::LongitudeOutOfRange`] or [`GeometryError::LatitudeOutOfRange`] outside WGS 84.
    pub fn try_new(longitude: f64, latitude: f64) -> Result<Self, GeometryError> {
        Self::in_range(coordinate(longitude)?, coordinate(latitude)?, None)
    }

    /// A three-dimensional position.
    ///
    /// # Errors
    ///
    /// As [`Position::try_new`].
    pub fn try_with_altitude(longitude: f64, latitude: f64, altitude: f64) -> Result<Self, GeometryError> {
        Self::in_range(coordinate(longitude)?, coordinate(latitude)?, Some(coordinate(altitude)?))
    }

    /// A two-dimensional position from numbers that keep the digits they were written with, such as an ordinate
    /// with more digits than the nearest `f64` holds.
    ///
    /// ```
    /// use ngsi_ld::model::{Number, geometry::Position};
    ///
    /// let longitude: Number = "13.39860000000000000001".parse()?;
    /// let position = Position::try_from_numbers(longitude, Number::from(52))?;
    /// assert_eq!(position.to_string(), "[13.39860000000000000001,52]");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Position::try_new`]; a number too large for an `f64` is not finite once read as one.
    pub fn try_from_numbers(longitude: Number, latitude: Number) -> Result<Self, GeometryError> {
        Self::in_range(longitude, latitude, None)
    }

    /// A three-dimensional position from numbers that keep the digits they were written with.
    ///
    /// # Errors
    ///
    /// As [`Position::try_from_numbers`].
    pub fn try_from_numbers_with_altitude(longitude: Number, latitude: Number, altitude: Number) -> Result<Self, GeometryError> {
        Self::in_range(longitude, latitude, Some(altitude))
    }

    /// Checks that every coordinate is finite and inside the WGS 84 ranges RFC 7946 section 4 places positions in,
    /// which a position built in code keeps.
    pub(crate) fn in_range(longitude: Number, latitude: Number, altitude: Option<Number>) -> Result<Self, GeometryError> {
        finite(&longitude, &latitude, altitude.as_ref())?;
        if !(-180.0..=180.0).contains(&longitude.as_f64()) {
            return Err(GeometryError::LongitudeOutOfRange { value: longitude });
        }
        if !(-90.0..=90.0).contains(&latitude.as_f64()) {
            return Err(GeometryError::LatitudeOutOfRange { value: latitude });
        }
        Ok(Self { longitude, latitude, altitude })
    }

    /// A position read from a document: every coordinate must be finite, and the WGS 84 ranges are not checked,
    /// so the position keeps what the document says.
    pub(crate) fn read(longitude: Number, latitude: Number, altitude: Option<Number>) -> Result<Self, GeometryError> {
        finite(&longitude, &latitude, altitude.as_ref())?;
        Ok(Self { longitude, latitude, altitude })
    }

    /// Returns the longitude.
    #[must_use]
    pub const fn longitude(&self) -> &Number {
        &self.longitude
    }

    /// Returns the latitude.
    #[must_use]
    pub const fn latitude(&self) -> &Number {
        &self.latitude
    }

    /// Returns the latitude, consuming the position.
    pub(crate) fn into_latitude(self) -> Number {
        self.latitude
    }

    /// Returns the altitude, if any.
    #[must_use]
    pub const fn altitude(&self) -> Option<&Number> {
        self.altitude.as_ref()
    }

    /// Returns the number of coordinates.
    #[must_use]
    pub const fn dimension(&self) -> Dimension {
        match self.altitude {
            Some(_) => Dimension::Three,
            None => Dimension::Two,
        }
    }

    /// Whether two positions denote the same point, whatever their spelling (`1` and `1.0`, `0` and `-0.0`): RFC
    /// 7946 section 3.1.6 requires the last position of a ring to hold values identical to the first.
    pub(crate) fn same_point(&self, other: &Self) -> bool {
        let same = |left: &Number, right: &Number| matches!(left.as_f64().partial_cmp(&right.as_f64()), Some(Ordering::Equal));
        let same_altitude = match (&self.altitude, &other.altitude) {
            (Some(left), Some(right)) => same(left, right),
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
        };
        same(&self.longitude, &other.longitude) && same(&self.latitude, &other.latitude) && same_altitude
    }
}

/// Checks that a line string or a ring has at least `minimum` positions.
pub(crate) const fn at_least(minimum: usize, positions: &[Position]) -> Result<(), GeometryError> {
    if positions.len() < minimum {
        Err(GeometryError::TooFewPositions {
            minimum,
            found: positions.len(),
        })
    } else {
        Ok(())
    }
}

/// Checks that every coordinate is finite once read as an `f64`: a JSON number beyond the range of one is not.
fn finite(longitude: &Number, latitude: &Number, altitude: Option<&Number>) -> Result<(), GeometryError> {
    [Some(longitude), Some(latitude), altitude]
        .into_iter()
        .flatten()
        .map(Number::as_f64)
        .find(|value| !value.is_finite())
        .map_or(Ok(()), |value| Err(GeometryError::InvalidCoordinate { source: NotFinite::of(value) }))
}

/// Converts a coordinate into a JSON number.
fn coordinate(value: f64) -> Result<Number, GeometryError> {
    Number::try_from(value).map_err(|source| GeometryError::InvalidCoordinate { source })
}

impl Position {
    /// Reads `[longitude, latitude(, altitude)]`; the coordinates only have to be finite (see [`Position::read`]).
    pub(crate) fn from_json(json: &Json) -> Result<Self, GeometryError> {
        let numbers = json
            .as_array()
            .ok_or(GeometryError::InvalidPosition)?
            .iter()
            .map(|item| match item {
                // The JSON tree is borrowed, and the position owns its numbers.
                Json::Number(number) => Ok(Number::from_json(number.clone())),
                Json::Null | Json::Boolean(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => Err(GeometryError::InvalidPosition),
            })
            .collect::<Result<Vec<_>, _>>()?;
        // RFC 7946 section 3.1.1 calls a fourth element unspecified and ambiguous, so it is refused, not dropped.
        match <[Number; 2]>::try_from(numbers) {
            Ok([longitude, latitude]) => Self::read(longitude, latitude, None),
            Err(numbers) => match <[Number; 3]>::try_from(numbers) {
                Ok([longitude, latitude, altitude]) => Self::read(longitude, latitude, Some(altitude)),
                Err(_) => Err(GeometryError::InvalidPosition),
            },
        }
    }

    /// The coordinates in the order RFC 7946 writes them: longitude, latitude and the altitude, if any.
    pub(crate) fn coordinates(&self) -> impl Iterator<Item = &Number> {
        [Some(&self.longitude), Some(&self.latitude), self.altitude.as_ref()].into_iter().flatten()
    }

    /// Writes the position as a JSON array.
    pub(crate) fn to_json(&self) -> Json {
        Json::Array(self.coordinates().map(Number::to_json).collect())
    }
}

// The text form is the compact JSON form, which only exists once the position is turned into a JSON tree.
impl Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_json().compact_print(), f)
    }
}
