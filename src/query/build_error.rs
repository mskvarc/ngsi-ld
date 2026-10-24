//! Why a query, type selection, projection, language filter, entity ordering, geoquery, scope query or temporal
//! query builder could not produce a tree.

use crate::{
    model::{BuildError, LanguageTagError, NameError, Number, ScopeError, Term},
    query::{
        geoquery::DistanceError,
        projection::EntityMember,
        q::{AttributePath, MemberError, PatternError, ValuePath},
        type_selection::SelectableTypeError,
    },
};

/// One invalid input given to a query-language builder. A problem names what is wrong, not the text given, which the
/// caller has.
///
/// The builders of the query languages take names as plain strings so the common case reads in one line, and
/// they never panic on an invalid one. Each builder carries the problems it found along with the tree it is
/// building; the final `build()` (or `TryFrom`) step returns them all at once as a [`BuildError`] of [`Problem`].
///
/// The enum is `#[non_exhaustive]`: new builder operations bring new ways to misuse them.
///
/// ```
/// use ngsi_ld::query::{Problem, q};
///
/// let error = q::attr("speed limit")
///     .gt(50)
///     .and(q::attr("9lives").exists())
///     .build()
///     .err();
/// let problems = error.as_ref().map(|error| error.problems());
/// assert!(matches!(
///     problems,
///     Some([Problem::InvalidName { .. }, Problem::InvalidName { .. }])
/// ));
/// assert_eq!(
///     error.map(|error| error.to_string()).as_deref(),
///     Some(
///         "2 problem(s): not a valid name: ` ` at byte 5 is not a name character; \
///          not a valid name: a name must start with a letter, found `9` at byte 0"
///     )
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum Problem {
    /// A string is not a valid attribute or entity type name (the `AttrName` production of clause 4.9).
    #[error("not a valid name")]
    InvalidName {
        /// Why it is invalid.
        #[source]
        source: NameError,
    },
    /// A string is not a valid member of a member path.
    #[error("not a valid member name")]
    InvalidMember {
        /// Why it is invalid.
        #[source]
        source: MemberError,
    },
    /// A string is not a regular expression that can be written in a query.
    #[error("not a regular expression a query can hold")]
    InvalidPattern {
        /// Why it is invalid.
        #[source]
        source: PatternError,
    },
    /// A string is not an entity type a type selection can hold.
    #[error("not an entity type a type selection can hold")]
    InvalidEntityType {
        /// Why it is invalid.
        #[source]
        source: SelectableTypeError,
    },
    /// A sub-attribute was added after a member path, which always ends an attribute path.
    #[error("the sub-attribute `{name}` cannot follow a member path")]
    SubAttributeAfterMember {
        /// The sub-attribute.
        name: Term,
    },
    /// A linked entity path was started from a dotted path or a member path; clause 4.9 starts it from the name
    /// of a single relationship.
    #[error("a linked entity path starts from a relationship name, not from `{path}`")]
    LinkFromCompoundPath {
        /// The path the link was started from.
        path: ValuePath,
    },
    /// A value list was given no values.
    #[error("a value list needs at least one value")]
    EmptyValueList,
    /// A conjunction of entity types was given no types.
    #[error("a conjunction of entity types needs at least one type")]
    EmptyConjunction,
    /// A projection was given no terms.
    #[error("a projection needs at least one term")]
    EmptyProjection,
    /// A linked projection was attached to `id`, `type` or `scope`, which are not relationships.
    #[error("`{member}` is an entity member and has no linked entity to project")]
    LinkedEntityMember {
        /// The member.
        member: EntityMember,
    },
    /// A string is not a language tag (IETF RFC 5646).
    #[error("not a valid language tag")]
    InvalidLanguageTag {
        /// Why it is invalid.
        #[source]
        source: LanguageTagError,
    },
    /// An entity ordering was given a path through a linked entity, which the ordering language of clause
    /// 4.23.3 cannot express.
    #[error("an entity ordering cannot follow the linked entity path `{path}`")]
    LinkedOrderingKey {
        /// The path given.
        path: AttributePath,
    },
    /// An entity ordering sorts by distance from no origin; clause 4.23.3 requires `orderFrom` then.
    #[error("an entity ordering by distance needs an origin")]
    DistanceWithoutOrigin,
    /// An entity ordering sorts `id`, `type` or `scope` by distance, which clause 4.23.1 limits to
    /// `GeoProperty` attributes.
    #[error("`{member}` is an entity member and cannot be sorted by distance")]
    DistanceOnEntityMember {
        /// The member.
        member: EntityMember,
    },
    /// The distance of a `near` relation is not a number greater than zero (clause 4.10).
    #[error("`{distance}` is not a distance greater than zero")]
    InvalidDistance {
        /// The number given.
        distance: Number,
        /// Why it is invalid.
        #[source]
        source: DistanceError,
    },
    /// A string is not a scope pattern of the scope query language (clauses 4.18 and 4.19).
    #[error("not a valid scope pattern")]
    InvalidScopePattern {
        /// Why it is invalid.
        #[source]
        source: ScopeError,
    },
    /// A conjunction of scopes was given no scopes.
    #[error("a conjunction of scopes needs at least one scope")]
    EmptyScopeConjunction,
    /// Alternatives were added to `/#`, which clause 4.19 only writes on its own.
    #[error("`/#` matches any scope and cannot be combined with other alternatives")]
    AnyScopeWithAlternatives,
    /// A temporal query keeps the last 0 instances; `lastN` is a positive integer (Table 5.2.21-1).
    #[error("`lastN` must be greater than zero")]
    ZeroLastN,
    /// A temporal query was given no aggregation method.
    #[error("a temporal query needs at least one aggregation method")]
    EmptyAggregationMethods,
    /// An ICU collation (clause 4.23.3) is not a language tag (IETF RFC 5646), which carries it.
    #[error("not a valid collation")]
    InvalidCollation {
        /// Why it is invalid.
        #[source]
        source: LanguageTagError,
    },
}

/// A value under construction, or the problems that prevent it.
///
/// Combining two deferred values keeps the problems of both, so a builder reports every invalid input instead
/// of stopping at the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Deferred<T>(Result<T, Vec<Problem>>);

impl<T> Deferred<T> {
    /// A valid value.
    pub(crate) const fn valid(value: T) -> Self {
        Self(Ok(value))
    }

    /// A value prevented by `problem`.
    pub(crate) fn invalid(problem: Problem) -> Self {
        Self(Err(vec![problem]))
    }

    /// A value, or the single problem that prevents it.
    pub(crate) fn from_result(result: Result<T, Problem>) -> Self {
        result.map_or_else(Self::invalid, Self::valid)
    }

    /// Pairs two values, keeping the problems of both.
    pub(crate) fn zip<U>(self, other: Deferred<U>) -> Deferred<(T, U)> {
        match (self.0, other.0) {
            (Ok(left), Ok(right)) => Deferred(Ok((left, right))),
            (Err(problems), Ok(_)) | (Ok(_), Err(problems)) => Deferred(Err(problems)),
            (Err(mut left), Err(right)) => {
                left.extend(right);
                Deferred(Err(left))
            }
        }
    }

    /// Transforms the value.
    pub(crate) fn map<U>(self, transform: impl FnOnce(T) -> U) -> Deferred<U> {
        Deferred(self.0.map(transform))
    }

    /// Transforms the value with a step that can itself fail.
    pub(crate) fn and_then<U>(self, step: impl FnOnce(T) -> Result<U, Problem>) -> Deferred<U> {
        Deferred(self.0.and_then(|value| step(value).map_err(|problem| vec![problem])))
    }

    /// Returns the value, or every problem found.
    pub(crate) fn finish(self) -> Result<T, BuildError<Problem>> {
        self.0.map_err(BuildError::new)
    }
}

impl<T> FromIterator<Deferred<T>> for Deferred<Vec<T>> {
    fn from_iter<I: IntoIterator<Item = Deferred<T>>>(iter: I) -> Self {
        iter.into_iter().fold(Self::valid(Vec::new()), |all, next| {
            all.zip(next).map(|(mut values, value)| {
                values.push(value);
                values
            })
        })
    }
}
