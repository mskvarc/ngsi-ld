//! The builder of entity orderings (NGSI-LD clauses 4.23 and 5.2.43), started by [`by`](crate::query::ordering::by)
//! and the other starting functions of [`ordering`](crate::query::ordering).
//!
//! [`EntityOrderingBuilder`].

use crate::{
    model::{BuildError, geometry::Shape, language::parse_language_tag},
    query::{
        Problem,
        build_error::Deferred,
        ordering::{EntityOrdering, OrderBy, OrderingKey, OrderingTerm, SortDirection},
        q::builders::PathBuilder,
    },
};
use langtag::LangTagBuf;

/// Validates an ordering key: a value path, never a path through a linked entity.
fn ordering_key(path: impl Into<PathBuilder>) -> Deferred<OrderingKey> {
    path.into().into_value_path().map(OrderingKey::from)
}

/// An entity ordering under construction, started by [`by`](crate::query::ordering::by),
/// [`ascending`](crate::query::ordering::ascending), [`descending`](crate::query::ordering::descending),
/// [`nearest`](crate::query::ordering::nearest) or [`farthest`](crate::query::ordering::farthest), or from a parsed
/// [`OrderBy`].
///
/// Keys and the collation are validated on the spot, without panicking; [`build`](Self::build) reports every
/// problem at once.
///
/// ```
/// use ngsi_ld::query::ordering::{OrderBy, builders::EntityOrderingBuilder};
///
/// let order_by: OrderBy = "name.observedAt".parse()?;
/// let ordering = EntityOrderingBuilder::from(order_by)
///     .collation("und-u-ks-identic")
///     .build()?;
/// assert_eq!(ordering.order_by().to_string(), "name.observedAt");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityOrderingBuilder {
    /// The terms so far.
    terms: Deferred<Vec<OrderingTerm>>,
    /// The collation, when one is given.
    collation: Deferred<Option<LangTagBuf>>,
    /// The origin, when one is given.
    origin: Option<Shape>,
}

impl EntityOrderingBuilder {
    /// No term yet.
    pub(crate) const fn empty() -> Self {
        Self {
            terms: Deferred::valid(Vec::new()),
            collation: Deferred::valid(None),
            origin: None,
        }
    }

    /// Adds `key`, without writing a direction.
    #[must_use]
    pub fn then(self, key: impl Into<PathBuilder>) -> Self {
        self.push(ordering_key(key).map(OrderingTerm::new))
    }

    /// Adds `key`, ascending (`;asc`).
    #[must_use]
    pub fn then_ascending(self, key: impl Into<PathBuilder>) -> Self {
        self.push_directed(key, SortDirection::Ascending)
    }

    /// Adds `key`, descending (`;desc`).
    #[must_use]
    pub fn then_descending(self, key: impl Into<PathBuilder>) -> Self {
        self.push_directed(key, SortDirection::Descending)
    }

    /// Adds the `GeoProperty` `key`, nearest to the origin first (`;dist-asc`).
    #[must_use]
    pub fn then_nearest(self, key: impl Into<PathBuilder>) -> Self {
        self.push_directed(key, SortDirection::DistanceAscending)
    }

    /// Adds the `GeoProperty` `key`, farthest from the origin first (`;dist-desc`).
    #[must_use]
    pub fn then_farthest(self, key: impl Into<PathBuilder>) -> Self {
        self.push_directed(key, SortDirection::DistanceDescending)
    }

    /// Compares strings with the ICU collation `tag` (IETF RFC 6067), such as `de-u-co-phonebk` (clause 4.23.3,
    /// example 7).
    #[must_use]
    pub fn collation(self, tag: impl AsRef<str>) -> Self {
        let tag = tag.as_ref();
        // `langtag`'s error only repeats the input, which the problem already holds.
        let collation = Deferred::from_result(parse_language_tag(tag).map(Some).map_err(|source| Problem::InvalidCollation { source }));
        Self { collation, ..self }
    }

    /// Measures distances from `origin` (`orderFrom` and `orderGeometry`, clause 4.23.3, examples 8 to 10).
    #[must_use]
    pub fn origin(self, origin: Shape) -> Self {
        Self { origin: Some(origin), ..self }
    }

    /// Adds `key` with `direction`.
    fn push_directed(self, key: impl Into<PathBuilder>, direction: SortDirection) -> Self {
        self.push(ordering_key(key).map(|key| OrderingTerm::new(key).with_direction(direction)))
    }

    /// Appends a term.
    fn push(self, term: Deferred<OrderingTerm>) -> Self {
        Self {
            terms: self.terms.zip(term).map(|(mut terms, term)| {
                terms.push(term);
                terms
            }),
            ..self
        }
    }

    /// Returns the ordering.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid key and collation, every entity member sorted by distance,
    /// and a sort by distance without an origin.
    pub fn build(self) -> Result<EntityOrdering, BuildError<Problem>> {
        let origin = self.origin.as_ref();
        self.terms
            .and_then(|terms| check_distances(&terms, origin).map(|()| OrderBy(terms)))
            .zip(self.collation)
            .map(|(order_by, collation)| EntityOrdering {
                order_by,
                collation,
                origin: self.origin,
            })
            .finish()
    }
}

/// Checks the terms sorting by distance: each sorts an attribute, and `origin` is given (clauses 4.23.1, 4.23.3).
fn check_distances(terms: &[OrderingTerm], origin: Option<&Shape>) -> Result<(), Problem> {
    let mut distances = terms.iter().filter(|term| term.is_distance()).peekable();
    let sorts_by_distance = distances.peek().is_some();
    match distances.find_map(|term| match term.key {
        OrderingKey::Member(member) => Some(member),
        OrderingKey::Attribute(_) => None,
    }) {
        Some(member) => Err(Problem::DistanceOnEntityMember { member }),
        None if sorts_by_distance && origin.is_none() => Err(Problem::DistanceWithoutOrigin),
        None => Ok(()),
    }
}

/// A parsed `orderBy`, to complete with a collation or an origin.
impl From<OrderBy> for EntityOrderingBuilder {
    fn from(order_by: OrderBy) -> Self {
        Self {
            terms: Deferred::valid(order_by.0),
            ..Self::empty()
        }
    }
}

/// An ordering already built.
impl From<EntityOrdering> for EntityOrderingBuilder {
    fn from(ordering: EntityOrdering) -> Self {
        Self {
            terms: Deferred::valid(ordering.order_by.0),
            collation: Deferred::valid(ordering.collation),
            origin: ordering.origin,
        }
    }
}

impl TryFrom<EntityOrderingBuilder> for EntityOrdering {
    type Error = BuildError<Problem>;

    fn try_from(builder: EntityOrderingBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
