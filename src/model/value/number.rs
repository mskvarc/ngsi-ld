//! JSON numbers, kept in the lexical form they were read or written with.

use jstrict::{NumberBuf, Value as Json};
use std::str::FromStr;

/// A floating-point number is NaN or infinite, which JSON cannot write; the variant says which.
///
/// The enum is exhaustive: IEEE 754 has no other value that is not finite.
///
/// ```
/// use ngsi_ld::model::{NotFinite, Number};
///
/// assert_eq!(
///     Number::try_from(f64::NEG_INFINITY),
///     Err(NotFinite::NegativeInfinity)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum NotFinite {
    /// NaN.
    #[error("NaN cannot be written as a JSON number")]
    Nan,
    /// Positive infinity.
    #[error("inf cannot be written as a JSON number")]
    PositiveInfinity,
    /// Negative infinity.
    #[error("-inf cannot be written as a JSON number")]
    NegativeInfinity,
}

impl NotFinite {
    /// Classifies `value`, which is NaN or infinite.
    pub(crate) const fn of(value: f64) -> Self {
        if value.is_nan() {
            Self::Nan
        } else if value.is_sign_negative() {
            Self::NegativeInfinity
        } else {
            Self::PositiveInfinity
        }
    }
}

/// The text is not a JSON number (IETF RFC 8259 section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("not a JSON number")]
pub struct NumberError;

/// A JSON number, kept with the exact lexical form it was read or written with, so `1.10` stays `1.10`.
///
/// A number the Rust primitives cannot hold exactly, such as a decimal beyond the precision of an `f64` or an
/// integer beyond the range of a `u64`, is parsed from its lexical form (clause 4.6.3 takes JSON numbers as IETF RFC
/// 8259 defines them, with no limit on range or precision).
///
/// ```
/// use ngsi_ld::model::Number;
///
/// let integer = Number::from(80);
/// assert_eq!(integer.to_string(), "80");
/// let decimal = Number::try_from(0.5)?;
/// assert_eq!(decimal.to_string(), "0.5");
/// let exact: Number = "3.14159265358979323846264338327950288".parse()?;
/// assert_eq!(exact.as_str(), "3.14159265358979323846264338327950288");
/// let large: Number = "123456789012345678901234567890".parse()?;
/// assert_eq!(large.as_i64(), None);
/// assert!("1.".parse::<Number>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
pub struct Number(NumberBuf);

impl Number {
    /// Returns the number as an `i64` when it is an integer in range.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        self.0.as_i64()
    }

    /// Returns the nearest `f64`.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.0.as_f64_lossy()
    }

    /// Returns the lexical form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Wraps a number read from a JSON tree.
    pub(crate) const fn from_json(number: NumberBuf) -> Self {
        Self(number)
    }

    /// Returns the number as a JSON number, consuming it.
    pub(crate) fn into_json(self) -> Json {
        Json::Number(self.0)
    }

    /// Returns the number as a JSON number.
    pub(crate) fn to_json(&self) -> Json {
        // Clone: the number is written from a borrow, and the JSON tree owns its numbers.
        Json::Number(self.0.clone())
    }
}

/// Generates the lossless `From` conversions from Rust integers; they all go through `jstrict`'s own. No derive
/// converts a primitive into the wrapped number.
macro_rules! number_from_integers {
    ($($integer:ty),+) => {$(
        impl From<$integer> for Number {
            fn from(value: $integer) -> Self {
                Self(value.into())
            }
        }
    )+};
}

number_from_integers!(u8, i8, u16, i16, u32, i32, u64, i64);

/// Fails on NaN and the infinities, which JSON cannot write.
impl TryFrom<f64> for Number {
    type Error = NotFinite;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        NumberBuf::try_from(value).map(Self).map_err(|_| NotFinite::of(value))
    }
}

// No derive: `derive_more::FromStr` would expose `jstrict`'s error type, which carries the refused bytes.
/// Parses the lexical form of a JSON number exactly, keeping every digit.
impl FromStr for Number {
    type Err = NumberError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self).map_err(|_| NumberError)
    }
}
