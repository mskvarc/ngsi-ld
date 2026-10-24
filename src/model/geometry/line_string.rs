//! A `GeoJSON` line string (IETF RFC 7946 section 3.1.4).

use crate::model::geometry::{GeometryError, Position, position::at_least};

/// A line string: two or more positions (RFC 7946 section 3.1.4).
///
/// ```
/// use ngsi_ld::model::geometry::{LineString, Position, Shape};
///
/// let route = LineString::new(vec![
///     Position::new(13.3986, 52.5547)?,
///     Position::new(13.4050, 52.5200)?,
/// ])?;
/// assert_eq!(
///     Shape::LineString(route).to_string(),
///     r#"{"type":"LineString","coordinates":[[13.3986,52.5547],[13.405,52.52]]}"#
/// );
/// assert!(LineString::new(vec![Position::new(13.3986, 52.5547)?]).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineString(Vec<Position>);

impl LineString {
    /// A line string through `positions`.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::TooFewPositions`] for fewer than two positions.
    pub fn new(positions: Vec<Position>) -> Result<Self, GeometryError> {
        at_least(2, &positions)?;
        Ok(Self(positions))
    }

    /// Returns the positions.
    #[must_use]
    pub fn positions(&self) -> &[Position] {
        &self.0
    }
}
