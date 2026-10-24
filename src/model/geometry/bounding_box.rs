//! The bounding box of a `GeoJSON` geometry (IETF RFC 7946 section 5) and its JSON form.

use crate::model::{
    Number,
    geometry::{Dimension, GeometryError, Position},
};
use jstrict::Value as Json;

/// A bounding box (RFC 7946 section 5): its south-western and north-eastern corners, both of one dimension, the
/// southern edge not north of the northern one.
///
/// The western edge may lie east of the eastern one: such a box crosses the antimeridian (RFC 7946 section 5.2).
/// Altitudes are kept as given.
///
/// ```
/// use ngsi_ld::model::geometry::{BoundingBox, Position};
///
/// let bbox = BoundingBox::try_new(
///     Position::try_new(170.0, 0.0)?,
///     Position::try_new(-170.0, 1.0)?,
/// )?;
/// assert_eq!(bbox.north_east().longitude().to_string(), "-170");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundingBox {
    /// The south-western corner: the first half of the `bbox` member.
    south_west: Position,
    /// The north-eastern corner: the second half of the `bbox` member; west of `south_west` when the box crosses
    /// the antimeridian.
    north_east: Position,
}

impl BoundingBox {
    /// A bounding box from its corners.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::BoundingBoxCorners`] when the corners have different dimensions, and
    /// [`GeometryError::BoundingBoxLatitudes`] when the south-western corner lies north of the north-eastern one.
    pub fn try_new(south_west: Position, north_east: Position) -> Result<Self, GeometryError> {
        if south_west.dimension() != north_east.dimension() {
            return Err(GeometryError::BoundingBoxCorners);
        }
        if south_west.latitude().as_f64() > north_east.latitude().as_f64() {
            return Err(GeometryError::BoundingBoxLatitudes {
                south: south_west.into_latitude(),
                north: north_east.into_latitude(),
            });
        }
        Ok(Self { south_west, north_east })
    }

    /// Returns the south-western corner: the lowest values of every coordinate but the longitude, which is the
    /// western edge.
    #[must_use]
    pub const fn south_west(&self) -> &Position {
        &self.south_west
    }

    /// Returns the north-eastern corner: the highest values of every coordinate but the longitude, which is the
    /// eastern edge.
    #[must_use]
    pub const fn north_east(&self) -> &Position {
        &self.north_east
    }

    /// Returns the dimension of the corners.
    #[must_use]
    pub const fn dimension(&self) -> Dimension {
        self.south_west.dimension()
    }

    /// Reads `[west, south, east, north]` or `[west, south, low, east, north, high]`.
    pub(crate) fn from_json(json: &Json) -> Result<Self, GeometryError> {
        let numbers = json
            .as_array()
            .ok_or(GeometryError::InvalidBoundingBox)?
            .iter()
            .map(|item| match item {
                // The JSON tree is borrowed, and the box owns its numbers.
                Json::Number(number) => Some(Number::from_json(number.clone())).filter(|number| number.as_f64().is_finite()),
                Json::Null | Json::Boolean(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => None,
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(GeometryError::InvalidBoundingBox)?;
        let (south_west, north_east) = match <[Number; 4]>::try_from(numbers) {
            Ok([west, south, east, north]) => (Position::read(west, south, None)?, Position::read(east, north, None)?),
            Err(numbers) => match <[Number; 6]>::try_from(numbers) {
                Ok([west, south, low, east, north, high]) => (Position::read(west, south, Some(low))?, Position::read(east, north, Some(high))?),
                Err(_) => return Err(GeometryError::InvalidBoundingBox),
            },
        };
        Self::try_new(south_west, north_east)
    }

    /// Writes the box as RFC 7946 section 5 orders it: every coordinate of the south-western corner, then every
    /// coordinate of the north-eastern one.
    pub(crate) fn to_json(&self) -> Json {
        Json::Array(
            self.south_west
                .coordinates()
                .chain(self.north_east.coordinates())
                .map(Number::to_json)
                .collect(),
        )
    }
}
