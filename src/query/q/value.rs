//! Values of the NGSI-LD Query Language (the `ComparableValue`, `Value`, `ValueList`, `Range` and
//! `CompEqualityValue` productions of NGSI-LD clause 4.9).

use crate::{
    model::{Date, DateTime, EntityId, Number, Time, Uri},
    query::grammar::write_separated,
};
use jstrict::print::string_literal;
use std::fmt;

/// A value that can be ordered (the `ComparableValue` production of NGSI-LD clause 4.9): the operand of `>`,
/// `>=`, `<` and `<=`, and the bounds of a range.
///
/// Strings are quoted and escaped as JSON strings (IETF RFC 8259, section 7). Identifiers and URIs given to the
/// builder become quoted strings, as in clause 4.9 example 3: an unquoted URI may hold `;`, `,` or `)`, which
/// a query would read as delimiters.
///
/// The enum is exhaustive: clause 4.9 closes the list of comparable kinds.
///
/// ```
/// use ngsi_ld::{model::DateTime, query::q::ComparableValue};
///
/// assert_eq!(ComparableValue::from(20).to_string(), "20");
/// assert_eq!(
///     ComparableValue::from(r#"say "hi""#).to_string(),
///     r#""say \"hi\"""#
/// );
/// let observed: DateTime = "2017-12-24T12:00:00Z".parse()?;
/// assert_eq!(
///     ComparableValue::from(observed).to_string(),
///     "2017-12-24T12:00:00Z"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::From)]
pub enum ComparableValue {
    /// A JSON number (IETF RFC 8259, section 6).
    Number(Number),
    /// A string, written quoted.
    Text(String),
    /// A `DateTime` (clause 4.6.3), written unquoted.
    DateTime(DateTime),
    /// A Date (clause 4.6.3), written unquoted.
    Date(Date),
    /// A Time (clause 4.6.3), written unquoted.
    Time(Time),
}

// Strings are escaped as JSON strings, which a derive cannot express.
impl fmt::Display for ComparableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(f),
            Self::Text(text) => string_literal(text, f),
            Self::DateTime(date_time) => date_time.fmt(f),
            Self::Date(date) => date.fmt(f),
            Self::Time(time) => time.fmt(f),
        }
    }
}

impl From<&str> for ComparableValue {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

/// An entity identifier is compared as a quoted string (clause 4.9, example 3).
impl From<EntityId> for ComparableValue {
    fn from(id: EntityId) -> Self {
        Self::Text(id.to_string())
    }
}

/// A URI is compared as a quoted string (clause 4.9, example 3).
impl From<Uri> for ComparableValue {
    fn from(uri: Uri) -> Self {
        Self::Text(uri.to_string())
    }
}

/// A value of a value list (the `Value` production of NGSI-LD clause 4.9): a comparable value or a boolean.
///
/// The enum is exhaustive: clause 4.9 closes the list of value kinds.
///
/// ```
/// use ngsi_ld::query::q::Value;
///
/// assert_eq!(Value::from(true).to_string(), "true");
/// assert_eq!(Value::from("red").to_string(), r#""red""#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum Value {
    /// A comparable value.
    #[display("{_0}")]
    Comparable(ComparableValue),
    /// A JSON boolean (IETF RFC 8259, section 3).
    #[display("{_0}")]
    Boolean(bool),
}

/// A list of at least two values (the `ValueList` production of NGSI-LD clause 4.9), matched when the target
/// value equals any of them.
///
/// ```
/// use ngsi_ld::query::q::{Value, ValueList};
///
/// let colors = ValueList::try_from(vec![Value::from("black"), Value::from("red")])?;
/// assert_eq!(colors.to_string(), r#""black","red""#);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueList(Vec<Value>);

impl ValueList {
    /// Returns the values, in order; there are at least two.
    #[must_use]
    pub fn as_slice(&self) -> &[Value] {
        &self.0
    }
}

/// A value list was given fewer than two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("a value list needs at least two values, found {found}")]
pub struct ValueListError {
    /// The number of values given.
    pub found: usize,
}

impl TryFrom<Vec<Value>> for ValueList {
    type Error = ValueListError;

    fn try_from(values: Vec<Value>) -> Result<Self, Self::Error> {
        if values.len() < 2 {
            Err(ValueListError { found: values.len() })
        } else {
            Ok(Self(values))
        }
    }
}

// The values are joined with `,`, which a derive cannot express.
impl fmt::Display for ValueList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated(f, &self.0, ',')
    }
}

/// An inclusive range (the `Range` production of NGSI-LD clause 4.9), matched when the target value lies
/// between the minimum and the maximum, both included.
///
/// ```
/// use ngsi_ld::query::q::Range;
///
/// assert_eq!(Range::new(10, 20).to_string(), "10..20");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{min}..{max}")]
pub struct Range {
    /// The minimum.
    min: ComparableValue,
    /// The maximum.
    max: ComparableValue,
}

impl Range {
    /// The range from `min` to `max`, both included.
    #[must_use]
    pub fn new(min: impl Into<ComparableValue>, max: impl Into<ComparableValue>) -> Range {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }

    /// Returns the minimum.
    #[must_use]
    pub const fn min(&self) -> &ComparableValue {
        &self.min
    }

    /// Returns the maximum.
    #[must_use]
    pub const fn max(&self) -> &ComparableValue {
        &self.max
    }
}

/// A URI written unquoted after `==` or `!=` (the `URI` alternative of the `CompEqualityValue` production of
/// NGSI-LD clause 4.9).
///
/// Clause 4.9 does not say where an unquoted URI ends, and a URI may hold the delimiters of the query
/// language. An unquoted URI here ends at the first `;`, `|` or `)`, so it cannot hold those characters. The
/// builder never writes one (it quotes identifiers and URIs); this type exists so that a query that uses one
/// parses and prints back.
///
/// ```
/// use ngsi_ld::{model::Uri, query::q::UnquotedUri};
///
/// let uri: Uri = "http://example.org/red".parse()?;
/// assert_eq!(
///     UnquotedUri::try_from(uri).map(|uri| uri.to_string()),
///     Ok("http://example.org/red".to_owned())
/// );
/// let delimited: Uri = "http://example.org/a;b".parse()?;
/// assert!(UnquotedUri::try_from(delimited).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::AsRef)]
#[display("{_0}")]
pub struct UnquotedUri(Uri);

/// A URI holds a character that delimits an unquoted URI in a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("`{found}` at byte {position} cannot appear in an unquoted URI")]
pub struct UnquotedUriError {
    /// The delimiting character.
    pub found: char,
    /// Its byte offset in the URI.
    pub position: usize,
}

/// Whether `c` ends an unquoted URI in a query.
pub(crate) const fn ends_unquoted_uri(c: char) -> bool {
    matches!(c, ';' | '|' | ')')
}

impl TryFrom<Uri> for UnquotedUri {
    type Error = UnquotedUriError;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        match uri.as_str().char_indices().find(|&(_, c)| ends_unquoted_uri(c)) {
            Some((position, found)) => Err(UnquotedUriError { found, position }),
            None => Ok(Self(uri)),
        }
    }
}

/// The value of `==` and `!=` (the `ComparableValue` and `CompEqualityValue` productions of NGSI-LD clause 4.9).
///
/// The enum is exhaustive: clause 4.9 closes the list of equality operands.
///
/// ```
/// use ngsi_ld::query::q::{EqualityValue, Range};
///
/// assert_eq!(
///     EqualityValue::from(Range::new(10, 20)).to_string(),
///     "10..20"
/// );
/// assert_eq!(EqualityValue::from(false).to_string(), "false");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum EqualityValue {
    /// A single value.
    #[display("{_0}")]
    Single(Value),
    /// A list of values, any of which matches.
    #[display("{_0}")]
    List(ValueList),
    /// A range of values.
    #[display("{_0}")]
    Range(Range),
    /// An unquoted URI.
    #[display("{_0}")]
    Uri(UnquotedUri),
}

/// Lifts the conversions into [`ComparableValue`] to [`Value`] and [`EqualityValue`], so the builder takes
/// plain Rust values.
macro_rules! lift_comparable {
    ($($source:ty),+ $(,)?) => {$(
        impl From<$source> for Value {
            fn from(value: $source) -> Self {
                Self::Comparable(value.into())
            }
        }

        impl From<$source> for EqualityValue {
            fn from(value: $source) -> Self {
                Self::Single(value.into())
            }
        }
    )+};
}

/// Converts Rust integers into numbers.
macro_rules! comparable_from_integers {
    ($($integer:ty),+ $(,)?) => {$(
        impl From<$integer> for ComparableValue {
            fn from(value: $integer) -> Self {
                Self::Number(value.into())
            }
        }
    )+};
}

comparable_from_integers!(u8, i8, u16, i16, u32, i32, u64, i64);
lift_comparable!(u8, i8, u16, i16, u32, i32, u64, i64, Number, String, &str, DateTime, Date, Time, EntityId, Uri);

impl From<ComparableValue> for EqualityValue {
    fn from(value: ComparableValue) -> Self {
        Self::Single(Value::Comparable(value))
    }
}

impl From<bool> for EqualityValue {
    fn from(value: bool) -> Self {
        Self::Single(Value::Boolean(value))
    }
}
