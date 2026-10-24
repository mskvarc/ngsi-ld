//! A `GeoJSON` polygon (IETF RFC 7946 section 3.1.6).

use crate::model::geometry::{GeometryError, LinearRing};

/// A polygon: an exterior ring and any number of holes (RFC 7946 section 3.1.6).
///
/// ```
/// use ngsi_ld::model::geometry::{GeometryError, LinearRing, Polygon, Position, Shape};
///
/// let corner = |longitude, latitude| Position::try_new(longitude, latitude);
/// let exterior = LinearRing::try_new(vec![
///     corner(0.0, 0.0)?,
///     corner(1.0, 0.0)?,
///     corner(1.0, 1.0)?,
///     corner(0.0, 0.0)?,
/// ])?;
/// let square = Polygon::try_new(vec![exterior])?;
/// assert_eq!(
///     Shape::Polygon(square).to_string(),
///     r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#
/// );
/// assert_eq!(
///     Polygon::try_new(Vec::new()),
///     Err(GeometryError::EmptyPolygon)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Polygon(Vec<LinearRing>);

impl Polygon {
    /// A polygon bounded by `rings`, the first being the exterior.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::EmptyPolygon`] when there is no ring.
    pub fn try_new(rings: Vec<LinearRing>) -> Result<Self, GeometryError> {
        if rings.is_empty() {
            Err(GeometryError::EmptyPolygon)
        } else {
            Ok(Self(rings))
        }
    }

    /// Returns the rings, exterior first.
    #[must_use]
    pub fn rings(&self) -> &[LinearRing] {
        &self.0
    }
}
