//! The builder of geoqueries (NGSI-LD clauses 4.10 and 5.2.13), started by
//! [`near_max`](crate::query::geoquery::near_max) and the other starting functions of
//! [`geoquery`](crate::query::geoquery).
//!
//! [`GeoQueryBuilder`].

use crate::{
    model::{AttributeName, BuildError, geometry::Shape},
    query::{
        Problem,
        build_error::Deferred,
        geoquery::{GeoQuery, GeoRelation},
    },
};

/// A geoquery under construction.
///
/// The distance and the `GeoProperty` name are validated on the spot, without panicking; [`build`](Self::build)
/// reports every problem at once.
///
/// ```
/// use ngsi_ld::{
///     model::geometry::{Position, Shape},
///     query::{Problem, geoquery},
/// };
///
/// let point = Shape::Point(Position::new(8.0, 40.0)?);
/// let error = geoquery::near_max(0, point)
///     .geoproperty("1location")
///     .build()
///     .err();
/// let problems = error.as_ref().map(|error| error.problems());
/// assert!(matches!(
///     problems,
///     Some([Problem::InvalidDistance { .. }, Problem::InvalidName { .. }])
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoQueryBuilder {
    /// The relationship, or why its distance is refused.
    relation: Deferred<GeoRelation>,
    /// The reference geometry.
    geometry: Shape,
    /// The `GeoProperty`, when one is named.
    geoproperty: Deferred<Option<AttributeName>>,
}

impl GeoQueryBuilder {
    /// A geoquery testing `relation` against `geometry`.
    pub(crate) const fn new(relation: Deferred<GeoRelation>, geometry: Shape) -> Self {
        Self {
            relation,
            geometry,
            geoproperty: Deferred::valid(None),
        }
    }

    /// Applies the geoquery to the `GeoProperty` `name` instead of `location` (`geoproperty`, clause 4.10).
    #[must_use]
    pub fn geoproperty(self, name: impl AsRef<str>) -> Self {
        let name = name.as_ref();
        let geoproperty = Deferred::from_result(name.parse().map(Some).map_err(|source| Problem::InvalidName { source }));
        Self { geoproperty, ..self }
    }

    /// Returns the geoquery.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing a distance that is not greater than zero and an invalid `GeoProperty` name.
    pub fn build(self) -> Result<GeoQuery, BuildError<Problem>> {
        self.relation
            .zip(self.geoproperty)
            .map(|(relation, geoproperty)| GeoQuery {
                relation,
                geometry: self.geometry,
                geoproperty,
            })
            .finish()
    }
}

/// A geoquery already built.
impl From<GeoQuery> for GeoQueryBuilder {
    fn from(query: GeoQuery) -> Self {
        Self {
            relation: Deferred::valid(query.relation),
            geometry: query.geometry,
            geoproperty: Deferred::valid(query.geoproperty),
        }
    }
}

impl TryFrom<GeoQueryBuilder> for GeoQuery {
    type Error = BuildError<Problem>;

    fn try_from(builder: GeoQueryBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
