//! The NGSI-LD Value (NGSI-LD clause 3.1): the JSON data a Property holds.
//!
//! Clause 3.1 defines a Value as JSON (RFC 8259) minus `null`, which clause 5.2.1 excludes because JSON-LD
//! expansion drops it. Clause C.6 additionally writes typed values (a `Date` or `DateTime` Property value) as JSON-LD
//! value objects, `{"@type": "DateTime", "@value": "2018-12-04T12:00:00Z"}`, which this type models explicitly.

use crate::model::{CoreDatatype, Date, DateTime, NotFinite, Number, Time, TypedValue, TypedValueError, nesting::grow};
use indexmap::IndexMap;
use jstrict::{Object, Print, Value as Json, object::Entry};
use std::fmt::{self, Display};

pub(super) mod number;
pub(super) mod patch;
pub(crate) mod positive_number;
pub(super) mod typed_value;
mod walk;

/// Why JSON data is not an NGSI-LD Value.
// `TemporalError` carries no equality, so neither can the error that chains it.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ValueError {
    /// The data contains a JSON `null`, which clause 5.2.1 excludes.
    #[error("a value cannot contain null")]
    Null,
    /// An object repeats a member name.
    #[error("an object repeats a member name")]
    DuplicateMember,
    /// A JSON-LD value object names a core datatype its `@value` is not a lexical form of.
    #[error("invalid typed value")]
    Typed {
        /// Why the lexical form is refused.
        #[from]
        source: TypedValueError,
    },
}

/// An NGSI-LD Value.
///
/// NGSI-LD clause 3.1.
///
/// Objects keep their member order. Everything that converts into a JSON value without loss converts into a
/// `Value` with `From`; floating-point numbers, which may be NaN, go through [`Number::try_from`].
///
/// ```
/// use ngsi_ld::model::{DateTime, Value};
///
/// assert_eq!(Value::from("Mercedes"), Value::Text("Mercedes".to_owned()));
/// assert_eq!(
///     Value::from(vec![300, 300, 120, 6]).to_string(),
///     "[300,300,120,6]"
/// );
/// let observed: DateTime = "2018-12-04T12:00:00Z".parse()?;
/// assert_eq!(
///     Value::from(observed).to_string(),
///     r#"{"@type":"DateTime","@value":"2018-12-04T12:00:00Z"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// A value nests as deep as the document that carries it, so cloning, comparing, debug-printing and dropping it run
/// on a stack that grows with the depth. Because the type implements `Drop`, a caller takes a member out of an
/// array or object by borrowing it, not by destructuring the value.
// `Debug`, `Clone` and `PartialEq` are written out in the `walk` submodule, next to `Drop`.
#[derive(Eq, derive_more::From, derive_more::IsVariant)]
pub enum Value {
    /// `true` or `false`.
    #[from(bool)]
    Boolean(bool),
    /// A number.
    #[from(Number)]
    Number(Number),
    /// A string.
    #[from(String, &str)]
    Text(String),
    /// An ordered array.
    #[from(skip)]
    Array(Vec<Value>),
    /// An object. Member names are free-form user data, so they are plain strings; their order is kept.
    #[from(IndexMap<String, Value>)]
    Object(IndexMap<String, Value>),
    /// A JSON-LD typed value, `{"@type": ..., "@value": ...}` (clause C.6).
    #[from(TypedValue)]
    Typed(TypedValue),
}

/// Generates the `From` conversions from the clause 4.6.3 types into their typed values. No derive writes a
/// timestamp as a JSON-LD value object.
macro_rules! value_from_temporal {
    ($($temporal:ident),+) => {$(
        impl From<$temporal> for Value {
            fn from(value: $temporal) -> Self {
                Self::Typed(TypedValue::of_core(CoreDatatype::$temporal, value.to_string()))
            }
        }
    )+};
}

value_from_temporal!(DateTime, Date, Time);

/// Generates `From` for the Rust integer types through [`Number`]; no derive converts a primitive into a variant
/// through another type.
macro_rules! value_from_integers {
    ($($integer:ty),+) => {$(
        impl From<$integer> for Value {
            fn from(value: $integer) -> Self {
                Self::Number(Number::from(value))
            }
        }
    )+};
}

value_from_integers!(u8, i8, u16, i16, u32, i32, u64, i64);

// No derive converts every element.
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(values: Vec<T>) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

// No derive converts every element.
impl<T: Into<Value>, const N: usize> From<[T; N]> for Value {
    fn from(values: [T; N]) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

/// Fails on NaN and the infinities, which JSON cannot write.
impl TryFrom<f64> for Value {
    type Error = NotFinite;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Number::try_from(value).map(Self::Number)
    }
}

impl Value {
    /// Reads a value from a JSON tree.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError`] when the tree contains `null`, an object repeats a member, or a value object names a
    /// core datatype its `@value` is not a lexical form of.
    ///
    /// Every level of nesting passes through here, so each one is read on a stack that grows with the depth (see
    /// [`grow`]).
    pub(crate) fn from_json(json: Json) -> Result<Self, ValueError> {
        grow(|| match json {
            Json::Null => Err(ValueError::Null),
            Json::Boolean(boolean) => Ok(Self::Boolean(boolean)),
            Json::Number(number) => Ok(Self::Number(Number::from_json(number))),
            Json::String(text) => Ok(Self::Text(text.into_string())),
            Json::Array(items) => items.into_iter().map(Self::from_json).collect::<Result<_, _>>().map(Self::Array),
            Json::Object(object) => Self::from_json_object(object),
        })
    }

    /// Reads an object, recognising a JSON-LD value object with a string `@value`.
    fn from_json_object(object: Object) -> Result<Self, ValueError> {
        if let Some(typed) = TypedValue::from_object(&object) {
            return Ok(Self::Typed(typed?));
        }
        let mut members = IndexMap::with_capacity(object.len());
        for Entry { key, value } in object {
            if members.insert(key.into_string(), Self::from_json(value)?).is_some() {
                return Err(ValueError::DuplicateMember);
            }
        }
        Ok(Self::Object(members))
    }

    /// Writes the value as a JSON tree, each level on a stack that grows with the depth (see [`grow`]).
    pub(crate) fn to_json(&self) -> Json {
        grow(|| match self {
            Self::Boolean(boolean) => Json::Boolean(*boolean),
            Self::Number(number) => number.to_json(),
            Self::Text(text) => Json::from(text.as_str()),
            Self::Array(items) => Json::Array(items.iter().map(Self::to_json).collect()),
            Self::Object(members) => Json::Object(members.iter().map(|(key, value)| (key.as_str().into(), value.to_json())).collect::<Object>()),
            Self::Typed(typed) => typed.to_json(),
        })
    }
}

// The text form is the compact JSON form, which only exists once the value is turned into a JSON tree.
impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_json().compact_print(), f)
    }
}

#[cfg(test)]
mod tests;
