//! The NGSI-LD Entity Ordering Language.
//!
//! It is written in the `orderBy` parameter (NGSI-LD clause 4.23). The module also holds the parameters that complete
//! an ordering: `collation`, `orderFrom` and `orderGeometry` (the `OrderingParams` of clause 5.2.43).
//!
//! An ordering lists the members entities are sorted by, separated by `,` and applied in turn, each an attribute
//! path or one of the entity members `id`, `type` and `scope`, optionally followed by `;` and a direction. The
//! paths are the value paths of the query language (clause 4.9): `address[city]`, `name.observedAt` (clause 4.23.3,
//! examples 4 and 5).
//!
//! ```
//! use ngsi_ld::query::ordering;
//!
//! let ordering = ordering::by("name")
//!     .then_descending("age")
//!     .collation("de-u-co-phonebk")
//!     .build()?;
//! assert_eq!(ordering.order_by().to_string(), "name,age;desc");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Orderings
//!
//! [`EntityOrdering`], [`OrderBy`], [`OrderingTerm`], [`OrderingKey`], [`SortDirection`].
//!
//! # Start functions
//!
//! [`by`], [`ascending`], [`descending`], [`nearest`], [`farthest`].
//!
//! # Ordering parameters
//!
//! [`OrderingParamsMember`], [`OrderingParamsError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, each_in_text, member},
};
pub use crate::query::ordering::json::{OrderingParamsError, OrderingParamsMember};
use crate::{
    model::geometry::Shape,
    query::{
        Expected,
        ParseError,
        grammar::{self, write_separated},
        ordering::builders::EntityOrderingBuilder,
        projection::EntityMember,
        q::{ValuePath, builders::PathBuilder},
    },
};
use langtag::LangTagBuf;
use std::{fmt, str::FromStr};

pub mod builders;
mod json;
mod parser;

/// The direction of one ordering term.
///
/// It is the `directionOp` production of NGSI-LD clause 4.23.3.
///
/// The enum is exhaustive: clause 4.23.1 defines four sort orders.
///
/// ```
/// use ngsi_ld::query::ordering::SortDirection;
///
/// assert_eq!(SortDirection::DistanceAscending.to_string(), "dist-asc");
/// assert_eq!(
///     "desc".parse::<SortDirection>(),
///     Ok(SortDirection::Descending)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum SortDirection {
    /// `asc`: ascending by value, the default when a term names no direction.
    #[strum(serialize = "asc")]
    Ascending,
    /// `desc`: descending by value.
    #[strum(serialize = "desc")]
    Descending,
    /// `dist-asc`: nearest first, from the origin of the ordering; `GeoProperty` attributes only (clause 4.23.1).
    #[strum(serialize = "dist-asc")]
    DistanceAscending,
    /// `dist-desc`: farthest first, from the origin of the ordering; `GeoProperty` attributes only (clause 4.23.1).
    #[strum(serialize = "dist-desc")]
    DistanceDescending,
}

impl SortDirection {
    /// Whether the direction sorts by distance, which needs an origin.
    #[must_use]
    pub const fn is_distance(self) -> bool {
        match self {
            Self::Ascending | Self::Descending => false,
            Self::DistanceAscending | Self::DistanceDescending => true,
        }
    }
}

/// What an ordering term sorts by: an entity member or an attribute path.
///
/// It keys a term of the `orderBy` parameter (NGSI-LD clause 6.4.3.2).
///
/// The enum is exhaustive: clause 6.4.3.2 names `id`, `type`, `scope` or an attribute.
///
/// ```
/// use ngsi_ld::query::{ordering::OrderingKey, projection::EntityMember, q::ValuePath};
///
/// let id = OrderingKey::from(ValuePath::new("id".parse()?));
/// assert_eq!(id, OrderingKey::Member(EntityMember::Id));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum OrderingKey {
    /// `id`, `type` or `scope`.
    #[display("{_0}")]
    Member(EntityMember),
    /// An attribute path.
    #[display("{_0}")]
    Attribute(ValuePath),
}

/// A path that is a single name spelling an entity member is that member; any other path is an attribute.
impl From<ValuePath> for OrderingKey {
    fn from(path: ValuePath) -> Self {
        match path.into_single_name() {
            Ok(name) => match name.as_ref().parse::<EntityMember>() {
                Ok(member) => Self::Member(member),
                Err(_) => Self::Attribute(ValuePath::new(name)),
            },
            Err(path) => Self::Attribute(path),
        }
    }
}

/// One term of an ordering: what it sorts by and, optionally, in which direction.
///
/// It is the `OrderingTerm` production of NGSI-LD clause 4.23.3, without its list.
///
/// ```
/// use ngsi_ld::query::{
///     ordering::{OrderingKey, OrderingTerm, SortDirection},
///     q::ValuePath,
/// };
///
/// let term = OrderingTerm::new(OrderingKey::from(ValuePath::new("age".parse()?)))
///     .with_direction(SortDirection::Descending);
/// assert_eq!(term.to_string(), "age;desc");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderingTerm {
    /// What the term sorts by.
    key: OrderingKey,
    /// The direction, when one is written; ascending otherwise (clause 4.23.3).
    direction: Option<SortDirection>,
}

impl OrderingTerm {
    /// A term sorting by `key`, without a written direction.
    #[must_use]
    pub const fn new(key: OrderingKey) -> OrderingTerm {
        OrderingTerm { key, direction: None }
    }

    /// Writes the direction `direction`.
    #[must_use]
    pub fn with_direction(self, direction: SortDirection) -> Self {
        Self {
            direction: Some(direction),
            ..self
        }
    }

    /// Returns what the term sorts by.
    #[must_use]
    pub const fn key(&self) -> &OrderingKey {
        &self.key
    }

    /// Returns the written direction; `None` sorts ascending (clause 4.23.3).
    #[must_use]
    pub const fn direction(&self) -> Option<SortDirection> {
        self.direction
    }

    /// Whether the term sorts by distance.
    fn is_distance(&self) -> bool {
        self.direction.is_some_and(SortDirection::is_distance)
    }
}

// The direction is optional and introduced by `;`, which a derive cannot express.
impl fmt::Display for OrderingTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.key.fmt(f)?;
        self.direction.map_or(Ok(()), |direction| write!(f, ";{direction}"))
    }
}

/// The value of the `orderBy` parameter: a non-empty list of terms, applied in turn.
///
/// It is written in the NGSI-LD Entity Ordering Language of clause 4.23.3.
///
/// ```
/// use ngsi_ld::query::ordering::OrderBy;
///
/// let order_by: OrderBy = "name,age;desc".parse()?;
/// assert_eq!(order_by.terms().len(), 2);
/// assert_eq!(order_by.to_string(), "name,age;desc");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderBy(Vec<OrderingTerm>);

impl OrderBy {
    /// Returns the terms, in the order they apply; there is at least one.
    #[must_use]
    pub fn terms(&self) -> &[OrderingTerm] {
        &self.0
    }
}

// The terms are joined with `,`, which a derive cannot express.
impl fmt::Display for OrderBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated(f, &self.0, ',')
    }
}

// No derive parses the `orderBy` grammar.
impl FromStr for OrderBy {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::order_by, Expected::OrderingKey, Expected::OrderingSeparator)
    }
}

/// An entity ordering, with its collation and the origin of its distances.
///
/// It holds the `orderBy` terms, the collation strings are compared with, and the origin distances are measured from
/// (the `OrderingParams` of NGSI-LD clause 5.2.43).
///
/// Built by [`by`] and the other starting functions, or from a parsed [`OrderBy`] through
/// [`EntityOrderingBuilder`]. Every ordering that sorts by distance has an origin, and only attributes are sorted by
/// distance (clause 4.23.1).
///
/// ```
/// use ngsi_ld::{
///     model::geometry::{Position, Shape},
///     query::ordering,
/// };
///
/// let origin = Shape::Point(Position::try_new(8.0, 40.0)?);
/// let nearest = ordering::nearest("location").origin(origin).build()?;
/// assert_eq!(nearest.order_by().to_string(), "location;dist-asc");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityOrdering {
    /// The terms.
    order_by: OrderBy,
    /// The ICU collation (IETF RFC 6067), when not the broker's default `und-x-icu`.
    collation: Option<LangTagBuf>,
    /// The geometry distances are measured from.
    origin: Option<Shape>,
}

impl EntityOrdering {
    /// Returns the terms (`orderBy`).
    #[must_use]
    pub const fn order_by(&self) -> &OrderBy {
        &self.order_by
    }

    /// Returns the collation (`collation`), a BCP 47 tag with a `-u-` extension such as `de-u-co-phonebk`.
    #[must_use]
    pub const fn collation(&self) -> Option<&LangTagBuf> {
        self.collation.as_ref()
    }

    /// Returns the geometry distances are measured from (`orderFrom` and `orderGeometry`).
    #[must_use]
    pub const fn origin(&self) -> Option<&Shape> {
        self.origin.as_ref()
    }
}
/// Starts an ordering with `key`, in the default ascending direction.
///
/// The direction is not written.
///
/// `key` is an attribute name (or `id`, `type`, `scope`), or a path built with [`q::attr`](crate::query::q::attr):
///
/// ```
/// use ngsi_ld::query::{ordering, q};
///
/// let ordering = ordering::by(q::attr("address").member("city")).build()?;
/// assert_eq!(ordering.order_by().to_string(), "address[city]");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn by(key: impl Into<PathBuilder>) -> EntityOrderingBuilder {
    EntityOrderingBuilder::empty().then(key)
}

/// Starts an ordering with `key`, ascending (`;asc`).
#[must_use]
pub fn ascending(key: impl Into<PathBuilder>) -> EntityOrderingBuilder {
    EntityOrderingBuilder::empty().then_ascending(key)
}

/// Starts an ordering with `key`, descending (`;desc`).
#[must_use]
pub fn descending(key: impl Into<PathBuilder>) -> EntityOrderingBuilder {
    EntityOrderingBuilder::empty().then_descending(key)
}

/// Starts an ordering with the `GeoProperty` `key`, nearest to the origin first.
///
/// It writes `;dist-asc`.
#[must_use]
pub fn nearest(key: impl Into<PathBuilder>) -> EntityOrderingBuilder {
    EntityOrderingBuilder::empty().then_nearest(key)
}

/// Starts an ordering with the `GeoProperty` `key`, farthest from the origin first.
///
/// It writes `;dist-desc`.
#[must_use]
pub fn farthest(key: impl Into<PathBuilder>) -> EntityOrderingBuilder {
    EntityOrderingBuilder::empty().then_farthest(key)
}
#[cfg(feature = "linked-data")]
impl MapNames for EntityOrdering {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            order_by: member(self.order_by, "orderBy", mapping)?,
            ..self
        })
    }
}

#[cfg(feature = "linked-data")]
impl MapNames for OrderBy {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        each_in_text(self.0, mapping).map(Self)
    }
}

/// `id`, `type` and `scope` are members of every entity, never terms of a context.
#[cfg(feature = "linked-data")]
impl MapNames for OrderingTerm {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        let key = match self.key {
            OrderingKey::Member(member) => OrderingKey::Member(member),
            OrderingKey::Attribute(path) => OrderingKey::Attribute(path.map_names(mapping)?),
        };
        Ok(Self { key, ..self })
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// An ordering carries no `@context`: it is written under the context of the request that carries it.
    impl for EntityOrdering
);

#[cfg(test)]
mod tests;
