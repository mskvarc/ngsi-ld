//! A closed `GeoJSON` linear ring (IETF RFC 7946 section 3.1.6).

use crate::model::geometry::{GeometryError, Position, position::at_least};

/// A closed linear ring: four or more positions, the last equal to the first (RFC 7946 section 3.1.6).
///
/// ```
/// use ngsi_ld::model::geometry::{GeometryError, LinearRing, Position};
///
/// let corner = |longitude, latitude| Position::new(longitude, latitude);
/// let ring = LinearRing::new(vec![
///     corner(0.0, 0.0)?,
///     corner(1.0, 0.0)?,
///     corner(1.0, 1.0)?,
///     corner(0.0, 0.0)?,
/// ])?;
/// assert_eq!(ring.positions().len(), 4);
/// assert_eq!(
///     LinearRing::new(vec![
///         corner(0.0, 0.0)?,
///         corner(1.0, 0.0)?,
///         corner(1.0, 1.0)?,
///         corner(0.0, 1.0)?
///     ]),
///     Err(GeometryError::RingNotClosed)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinearRing(Vec<Position>);

impl LinearRing {
    /// A ring through `positions`.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::TooFewPositions`] for fewer than four positions and
    /// [`GeometryError::RingNotClosed`] when the last position differs from the first.
    pub fn new(positions: Vec<Position>) -> Result<Self, GeometryError> {
        at_least(4, &positions)?;
        match (positions.first(), positions.last()) {
            (Some(first), Some(last)) if first.same_point(last) => Ok(Self(positions)),
            (Some(_) | None, Some(_) | None) => Err(GeometryError::RingNotClosed),
        }
    }

    /// Returns the positions.
    #[must_use]
    pub fn positions(&self) -> &[Position] {
        &self.0
    }
}
