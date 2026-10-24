//! The builders of NGSI-LD queries (NGSI-LD clause 4.9), started by [`attr`](crate::query::q::attr).
//!
//! [`PathBuilder`], [`QueryBuilder`].

use crate::{
    model::{BuildError, Term},
    query::{
        Problem,
        build_error::Deferred,
        q::{AttributePath, ComparableValue, Comparison, EqualityValue, Link, MemberName, Pattern, Query, QueryTerm, Range, Value, ValueList, ValuePath},
    },
};

/// Validates an attribute or entity type name.
fn parse_name(name: &str) -> Deferred<Term> {
    Deferred::from_result(name.parse().map_err(|source| Problem::InvalidName { source }))
}

/// Validates a regular expression.
fn parse_pattern(pattern: &str) -> Deferred<Pattern> {
    Deferred::from_result(pattern.parse().map_err(|source| Problem::InvalidPattern { source }))
}

/// An attribute path under construction: the links followed so far and the value path inside the last linked
/// entity.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PathState {
    /// The relationships followed, outermost first.
    links: Vec<Link>,
    /// The path inside the innermost entity.
    current: ValuePath,
}

impl PathState {
    /// Builds the attribute path.
    fn into_path(self) -> AttributePath {
        AttributePath::through(self.links, self.current)
    }
}

/// An attribute path under construction (the `Attribute` production of NGSI-LD clause 4.9), started by
/// [`attr`](crate::query::q::attr).
///
/// Its comparison methods ([`eq`](Self::eq), [`gt`](Self::gt), ...) turn it into a query term.
///
/// ```
/// use ngsi_ld::query::q;
///
/// let query = q::attr("sensor")
///     .sub_attr("rawdata")
///     .member("airquality")
///     .member("particulate")
///     .eq(40)
///     .build()?;
/// assert_eq!(
///     query.to_string(),
///     "sensor.rawdata[airquality.particulate]==40"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBuilder(Deferred<PathState>);

impl PathBuilder {
    /// Starts a path at the attribute `attribute`.
    pub(crate) fn new(attribute: &str) -> PathBuilder {
        Self(parse_name(attribute).map(|attribute| PathState {
            links: Vec::new(),
            current: ValuePath::new(attribute),
        }))
    }

    /// Descends into the sub-attribute `attribute` (a Property or Relationship of the attribute addressed so
    /// far, clause 4.9 example 8).
    // Not named `sub`, which would read as `std::ops::Sub::sub`.
    #[must_use]
    pub fn sub_attr(self, attribute: impl AsRef<str>) -> Self {
        Self(self.0.zip(parse_name(attribute.as_ref())).and_then(|(state, attribute)| {
            if state.current.members().is_empty() {
                Ok(PathState {
                    current: state.current.with_sub_attribute(attribute),
                    ..state
                })
            } else {
                Err(Problem::SubAttributeAfterMember { name: attribute })
            }
        }))
    }

    /// Descends into the member `member` of the compound value of the attribute (clause 4.9, example 9).
    ///
    /// Successive members form one member path: `.member("airquality").member("particulate")` prints
    /// `[airquality.particulate]`.
    #[must_use]
    pub fn member(self, member: impl AsRef<str>) -> Self {
        let member = member.as_ref();
        let member = Deferred::from_result(member.parse::<MemberName>().map_err(|source| Problem::InvalidMember { source }));
        Self(self.0.zip(member).map(|(state, member)| PathState {
            current: state.current.with_member(member),
            ..state
        }))
    }

    /// Follows the relationship addressed so far into the entity it targets, and continues at its attribute
    /// `attribute` (clause 4.9, example 13).
    #[must_use]
    pub fn linked(self, attribute: impl AsRef<str>) -> Self {
        self.linked_typed(<[&str; 0]>::default(), attribute)
    }

    /// Like [`linked`](Self::linked), with a hint of the types of the targeted entity (clause 4.9, example 14).
    #[must_use]
    pub fn linked_typed(self, entity_types: impl IntoIterator<Item = impl AsRef<str>>, attribute: impl AsRef<str>) -> Self {
        let entity_types: Deferred<Vec<Term>> = entity_types.into_iter().map(|entity_type| parse_name(entity_type.as_ref())).collect();
        Self(
            self.0
                .zip(entity_types)
                .zip(parse_name(attribute.as_ref()))
                .and_then(|((state, entity_types), attribute)| {
                    let PathState { mut links, current } = state;
                    let relationship = current.into_single_name().map_err(|path| Problem::LinkFromCompoundPath { path })?;
                    links.push(Link::new(relationship).with_entity_types(entity_types));
                    Ok(PathState {
                        links,
                        current: ValuePath::new(attribute),
                    })
                }),
        )
    }

    /// The value path, for a language that cannot follow a linked entity (the ordering of clause 4.23.3).
    pub(crate) fn into_value_path(self) -> Deferred<ValuePath> {
        self.0.and_then(|state| {
            if state.links.is_empty() {
                Ok(state.current)
            } else {
                Err(Problem::LinkedOrderingKey { path: state.into_path() })
            }
        })
    }

    /// Ends the path with `comparison`, or with none to test for existence.
    fn compare(self, comparison: Deferred<Option<Comparison>>) -> QueryBuilder {
        QueryBuilder(self.0.zip(comparison).map(|(state, comparison)| {
            Query::Term(QueryTerm {
                path: state.into_path(),
                comparison,
            })
        }))
    }

    /// Ends the path with a comparison that cannot be invalid.
    fn compare_with(self, comparison: Comparison) -> QueryBuilder {
        self.compare(Deferred::valid(Some(comparison)))
    }

    /// Matches entities that have the target element (clause 4.9, example 0).
    #[must_use]
    pub fn exists(self) -> QueryBuilder {
        self.compare(Deferred::valid(None))
    }

    /// Matches a target value equal to `value` (`==`).
    #[must_use]
    pub fn eq(self, value: impl Into<EqualityValue>) -> QueryBuilder {
        self.compare_with(Comparison::Equal(value.into()))
    }

    /// Matches a target value different from `value` (`!=`).
    #[must_use]
    pub fn ne(self, value: impl Into<EqualityValue>) -> QueryBuilder {
        self.compare_with(Comparison::Unequal(value.into()))
    }

    /// Matches a target value greater than `value` (`>`).
    #[must_use]
    pub fn gt(self, value: impl Into<ComparableValue>) -> QueryBuilder {
        self.compare_with(Comparison::Greater(value.into()))
    }

    /// Matches a target value greater than or equal to `value` (`>=`).
    #[must_use]
    pub fn ge(self, value: impl Into<ComparableValue>) -> QueryBuilder {
        self.compare_with(Comparison::GreaterOrEqual(value.into()))
    }

    /// Matches a target value less than `value` (`<`).
    #[must_use]
    pub fn lt(self, value: impl Into<ComparableValue>) -> QueryBuilder {
        self.compare_with(Comparison::Less(value.into()))
    }

    /// Matches a target value less than or equal to `value` (`<=`).
    #[must_use]
    pub fn le(self, value: impl Into<ComparableValue>) -> QueryBuilder {
        self.compare_with(Comparison::LessOrEqual(value.into()))
    }

    /// Matches a target value between `min` and `max`, both included (`==min..max`).
    #[must_use]
    pub fn within(self, min: impl Into<ComparableValue>, max: impl Into<ComparableValue>) -> QueryBuilder {
        self.eq(Range::new(min, max))
    }

    /// Matches a target value equal to any of `values` (`==v1,v2`).
    ///
    /// A single value is written as a plain equality; no value at all is a [`Problem::EmptyValueList`].
    #[must_use]
    pub fn any_of(self, values: impl IntoIterator<Item = impl Into<Value>>) -> QueryBuilder {
        let values: Vec<Value> = values.into_iter().map(Into::into).collect();
        let value = match <[Value; 1]>::try_from(values) {
            Ok([single]) => Deferred::valid(EqualityValue::Single(single)),
            // Two or more values form a list; the conversion fails only when there are none.
            Err(values) => Deferred::from_result(ValueList::try_from(values).map(EqualityValue::List).map_err(|_| Problem::EmptyValueList)),
        };
        self.compare(value.map(|value| Some(Comparison::Equal(value))))
    }

    /// Matches a string target value the regular expression `pattern` matches (`~=`).
    #[must_use]
    pub fn matches(self, pattern: impl AsRef<str>) -> QueryBuilder {
        self.compare(parse_pattern(pattern.as_ref()).map(|pattern| Some(Comparison::Matches(pattern))))
    }

    /// Matches a string target value the regular expression `pattern` does not match (`!~=`).
    #[must_use]
    pub fn not_matches(self, pattern: impl AsRef<str>) -> QueryBuilder {
        self.compare(parse_pattern(pattern.as_ref()).map(|pattern| Some(Comparison::NotMatches(pattern))))
    }
}

/// A path at the attribute named by the string, validated as the builder goes.
impl From<&str> for PathBuilder {
    fn from(attribute: &str) -> Self {
        Self::new(attribute)
    }
}

/// A query under construction, combined with [`and`](Self::and) and [`or`](Self::or).
///
/// [`attr`](crate::query::q::attr) starts a [`PathBuilder`]; a comparison method turns it into a query builder.
///
/// Names arrive as strings and are validated on the spot, never with a panic. An invalid name, member or regular
/// expression is recorded as a [`Problem`] that travels with the builder; combining two builders keeps the problems of
/// both. [`build`](Self::build) returns the query, or a [`BuildError`] listing every problem. A client builder can
/// therefore take the builder as it is, `.q(q::attr("speed").gt(50))`, and report the problems when the request is
/// built.
///
/// ```
/// use ngsi_ld::query::q;
///
/// let query = q::attr("temperature")
///     .ge(20)
///     .and(q::attr("temperature").le(25))
///     .or(q::attr("capacity").le(10))
///     .build()?;
/// assert_eq!(
///     query.to_string(),
///     "temperature>=20;temperature<=25|capacity<=10"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBuilder(Deferred<Query>);

impl QueryBuilder {
    /// Combines with `other` under AND (`;`).
    #[must_use]
    pub fn and(self, other: impl Into<QueryBuilder>) -> Self {
        Self(self.0.zip(other.into().0).map(|(left, right)| left.and(right)))
    }

    /// Combines with `other` under OR (`|`).
    #[must_use]
    pub fn or(self, other: impl Into<QueryBuilder>) -> Self {
        Self(self.0.zip(other.into().0).map(|(left, right)| left.or(right)))
    }

    /// Returns the query.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid name, member, value list and regular expression given to
    /// the builder.
    pub fn build(self) -> Result<Query, BuildError<Problem>> {
        self.0.finish()
    }
}

impl From<Query> for QueryBuilder {
    fn from(query: Query) -> Self {
        Self(Deferred::valid(query))
    }
}

/// An attribute path alone tests that the target element exists.
impl From<PathBuilder> for QueryBuilder {
    fn from(path: PathBuilder) -> Self {
        path.exists()
    }
}
