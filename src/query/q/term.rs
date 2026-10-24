//! Query terms of the NGSI-LD Query Language (the `QueryTerm` production of NGSI-LD clause 4.9).

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping},
};
use crate::query::q::{AttributePath, ComparableValue, EqualityValue, Pattern};
use std::fmt;

/// A comparison operator of the query language.
///
/// NGSI-LD clause 4.9 defines the operators.
///
/// The enum is exhaustive: clause 4.9 closes the set of operators.
///
/// ```
/// use ngsi_ld::query::q::Operator;
///
/// assert_eq!(Operator::NotMatches.to_string(), "!~=");
/// assert_eq!("<=".parse::<Operator>(), Ok(Operator::LessOrEqual));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum Operator {
    /// `==` (`equal`).
    #[strum(serialize = "==")]
    Equal,
    /// `!=` (`unequal`).
    #[strum(serialize = "!=")]
    Unequal,
    /// `>` (`greater`).
    #[strum(serialize = ">")]
    Greater,
    /// `>=` (`greaterEq`).
    #[strum(serialize = ">=")]
    GreaterOrEqual,
    /// `<` (`less`).
    #[strum(serialize = "<")]
    Less,
    /// `<=` (`lessEq`).
    #[strum(serialize = "<=")]
    LessOrEqual,
    /// `~=` (`patternOp`).
    #[strum(serialize = "~=")]
    Matches,
    /// `!~=` (`notPatternOp`).
    #[strum(serialize = "!~=")]
    NotMatches,
}

/// An operator with its value: what a query term compares the target value with.
///
/// NGSI-LD clause 4.9.
///
/// Each operator carries the kind of value clause 4.9 allows for it: `==` and `!=` take any value, a value
/// list, a range or a URI; the ordering operators take a comparable value; the pattern operators take a
/// regular expression.
///
/// The enum is exhaustive: clause 4.9 closes the set of operators.
///
/// ```
/// use ngsi_ld::query::q::{ComparableValue, Comparison};
///
/// let comparison = Comparison::Greater(ComparableValue::from(50));
/// assert_eq!(comparison.to_string(), ">50");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Comparison {
    /// The target value equals the value, any value of a list, or lies in a range.
    Equal(EqualityValue),
    /// The target value differs from the value, from every value of a list, or lies outside a range.
    Unequal(EqualityValue),
    /// The target value is greater than the value.
    Greater(ComparableValue),
    /// The target value is greater than or equal to the value.
    GreaterOrEqual(ComparableValue),
    /// The target value is less than the value.
    Less(ComparableValue),
    /// The target value is less than or equal to the value.
    LessOrEqual(ComparableValue),
    /// The target value is a string the regular expression matches.
    Matches(Pattern),
    /// The target value is a string the regular expression does not match.
    NotMatches(Pattern),
}

impl Comparison {
    /// Returns the operator.
    #[must_use]
    pub const fn operator(&self) -> Operator {
        match self {
            Self::Equal(_) => Operator::Equal,
            Self::Unequal(_) => Operator::Unequal,
            Self::Greater(_) => Operator::Greater,
            Self::GreaterOrEqual(_) => Operator::GreaterOrEqual,
            Self::Less(_) => Operator::Less,
            Self::LessOrEqual(_) => Operator::LessOrEqual,
            Self::Matches(_) => Operator::Matches,
            Self::NotMatches(_) => Operator::NotMatches,
        }
    }
}

// The operator is printed from `Operator`, the single place its token is spelled, followed by the value.
impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.operator().into())?;
        match self {
            Self::Equal(value) | Self::Unequal(value) => value.fmt(f),
            Self::Greater(value) | Self::GreaterOrEqual(value) | Self::Less(value) | Self::LessOrEqual(value) => value.fmt(f),
            Self::Matches(pattern) | Self::NotMatches(pattern) => pattern.fmt(f),
        }
    }
}

/// A query term: an attribute path, alone or followed by a comparison.
///
/// An attribute path alone tests that the target element exists (the `QueryTerm` production of NGSI-LD clause 4.9).
///
/// ```
/// use ngsi_ld::query::q::{AttributePath, Comparison, EqualityValue, QueryTerm, ValuePath};
///
/// let path = AttributePath::from(ValuePath::new("temperature".parse()?));
/// let term = QueryTerm {
///     path,
///     comparison: Some(Comparison::Equal(EqualityValue::from(20))),
/// };
/// assert_eq!(term.to_string(), "temperature==20");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryTerm {
    /// The attribute path addressing the target element.
    pub path: AttributePath,
    /// The comparison of the target value; `None` tests that the target element exists.
    pub comparison: Option<Comparison>,
}

// The comparison is optional, which a derive cannot express.
impl fmt::Display for QueryTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.fmt(f)?;
        self.comparison.as_ref().map_or(Ok(()), |comparison| comparison.fmt(f))
    }
}

#[cfg(feature = "linked-data")]
/// The values compared are data; only the path names attributes.
impl MapNames for QueryTerm {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            path: self.path.map_names(mapping)?,
            ..self
        })
    }
}
