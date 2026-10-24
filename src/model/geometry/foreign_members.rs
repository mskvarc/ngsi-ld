//! The foreign members of a geometry object (IETF RFC 7946 section 6.1): members RFC 7946 does not describe, which
//! a `GeoJSON` document may carry and which `GeoJSON` semantics never apply to.

use crate::model::{RawJson, geometry::GeometryMember};
use indexmap::IndexMap;

/// A member of other `GeoJSON` objects that RFC 7946 section 7.1 forbids on a geometry object: "`FeatureCollection` and
/// Geometry objects, respectively, MUST NOT contain a "geometry" or "properties" member", and "Feature and Geometry
/// objects, respectively, MUST NOT contain a "features" member".
///
/// The enum is exhaustive: section 7.1 closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum ForbiddenGeometryMember {
    /// `geometry`, a member of a Feature.
    #[strum(serialize = "geometry")]
    Geometry,
    /// `properties`, a member of a Feature.
    #[strum(serialize = "properties")]
    Properties,
    /// `features`, a member of a `FeatureCollection`.
    #[strum(serialize = "features")]
    Features,
}

/// Why a name is not the name of a foreign member of a geometry object.
///
/// The enum is exhaustive: a name is refused because RFC 7946 defines it for a geometry or because section 7.1
/// forbids it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum ForeignMemberNameError {
    /// The name is one RFC 7946 defines for a geometry object (sections 3.1 and 5).
    #[error("`{member}` is a member GeoJSON defines for a geometry, not a foreign one")]
    Defined {
        /// The member.
        member: GeometryMember,
    },
    /// The name is one RFC 7946 section 7.1 forbids on a geometry object.
    #[error("a geometry cannot have the `{member}` member")]
    Forbidden {
        /// The member.
        member: ForbiddenGeometryMember,
    },
}

/// Checks that a name is neither a member a geometry object defines nor one it must not contain.
fn validate_name(name: &str) -> Result<(), ForeignMemberNameError> {
    if let Ok(member) = name.parse::<GeometryMember>() {
        Err(ForeignMemberNameError::Defined { member })
    } else if let Ok(member) = name.parse::<ForbiddenGeometryMember>() {
        Err(ForeignMemberNameError::Forbidden { member })
    } else {
        Ok(())
    }
}

/// The name of a foreign member of a geometry object (RFC 7946 section 6.1): any member name other than `type`,
/// `coordinates` and `bbox`, which a geometry defines, and `geometry`, `properties` and `features`, which section 7.1
/// forbids on it. The name is otherwise free-form, so `plot name` is one.
///
/// ```
/// use ngsi_ld::model::geometry::ForeignMemberName;
///
/// let name: ForeignMemberName = "plot name".parse()?;
/// assert_eq!(name.as_ref(), "plot name");
/// assert!("bbox".parse::<ForeignMemberName>().is_err());
/// assert!("properties".parse::<ForeignMemberName>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_name, error = ForeignMemberNameError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ForeignMemberName(String);

/// The foreign members of a geometry object (RFC 7946 section 6.1), in document order. RFC 7946 defines no
/// processing model for them, so each value is raw JSON, kept as written: a value that is an object makes every
/// descendant member foreign too. Two sets of foreign members are equal when they hold the same members, whatever
/// their order.
///
/// ```
/// use ngsi_ld::model::{
///     RawJson,
///     geometry::{ForeignMemberName, Geometry, Position, Shape},
/// };
///
/// let point = Geometry::from(Shape::Point(Position::try_new(1.5, 2.5)?))
///     .with_foreign_member("plot name".parse()?, r#""7""#.parse()?);
/// let name: ForeignMemberName = "plot name".parse()?;
/// assert_eq!(
///     point.foreign_members().get(&name),
///     Some(&r#""7""#.parse::<RawJson>()?)
/// );
/// assert_eq!(
///     point.to_string(),
///     r#"{"type":"Point","coordinates":[1.5,2.5],"plot name":"7"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ForeignMembers(IndexMap<ForeignMemberName, RawJson>);

impl ForeignMembers {
    /// No foreign member.
    #[must_use]
    pub fn new() -> ForeignMembers {
        ForeignMembers(IndexMap::new())
    }

    /// Returns the value of the member `name`.
    #[must_use]
    pub fn get(&self, name: &ForeignMemberName) -> Option<&RawJson> {
        self.0.get(name)
    }

    /// Iterates over the members, in document order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ForeignMemberName, &RawJson)> {
        self.0.iter()
    }

    /// The number of members.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is no member.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Sets the member `name`, keeping its place when it is already set, and returns the value it had.
    pub fn insert(&mut self, name: ForeignMemberName, value: RawJson) -> Option<RawJson> {
        self.0.insert(name, value)
    }
}
