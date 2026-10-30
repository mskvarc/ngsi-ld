//! Timestamps, dates and times (NGSI-LD clause 4.6.3).
//!
//! Clause 4.6.3 fixes one lexical form per type: `YYYY-MM-DDThh:mm:ss[.s{1,6}]Z` for a `DateTime`, `YYYY-MM-DD`
//! for a Date and `hh:mm:ss[.s{1,6}]Z` for a Time, always in UTC, never reduced. The types here parse exactly
//! those forms, keep the number of fraction digits that was written (so `12:51:02.000Z` is written back as
//! `12:51:02.000Z`), and use `jiff` for the calendar arithmetic and range checks.

use crate::model::temporal::lexical::{Cursor, FractionMark, exact_fraction_digits, write_fraction};
use jiff::{Timestamp, civil, tz::Offset};
use std::{fmt, str::FromStr};

#[cfg(feature = "chrono")]
mod chrono;
mod lexical;

/// What a temporal parser expected where it found something else.
///
/// It is a part of a lexical form of NGSI-LD clause 4.6.3.
///
/// The enum is exhaustive: the lexical forms of clause 4.6.3 close the set of things a temporal parser expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
pub enum TemporalToken {
    /// An ASCII digit.
    #[display("a digit")]
    Digit,
    /// A specific separator or designator character.
    #[display("`{_0}`")]
    Character(char),
    /// The end of the input.
    #[display("the end of the input")]
    End,
}

/// Why a string or a value is not a valid NGSI-LD `DateTime`, `Date` or `Time`.
// `jiff::Error` carries no equality, so neither can the error that chains it.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum TemporalError {
    /// The input does not follow the lexical form.
    #[error("expected {expected} at byte {position}")]
    Unexpected {
        /// What the form requires at that position.
        expected: TemporalToken,
        /// The byte offset in the input.
        position: usize,
    },
    /// The fraction of a second has more than six digits.
    #[error("the fraction starting at byte {position} has more than six digits")]
    TooManyFractionDigits {
        /// The byte offset where the fraction starts.
        position: usize,
    },
    /// The value carries a UTC offset instead of the `Z` designator.
    #[error("the time zone at byte {position} must be `Z`")]
    NotUtc {
        /// The byte offset of the offset sign.
        position: usize,
    },
    /// A component does not fit the integer type it is held in.
    #[error("a date or time component is out of range")]
    ComponentOutOfRange,
    /// The components do not form a valid date or time (a month 13, a 30 February, ...).
    #[error("invalid date or time")]
    OutOfRange {
        /// The calendar error.
        #[from]
        source: jiff::Error,
    },
    /// The value has a precision finer than the microsecond clause 4.6.3 can express.
    #[error("the value has sub-microsecond precision, which a timestamp cannot express")]
    SubMicrosecond,
}

/// Builds a civil date from its parsed parts.
fn civil_date(cursor: &mut Cursor<'_>) -> Result<civil::Date, TemporalError> {
    let parts = cursor.date()?;
    Ok(civil::Date::new(parts.year, parts.month, parts.day)?)
}

/// Builds a civil time from its parsed parts, with the number of fraction digits written and the character that
/// opened them.
fn civil_time(cursor: &mut Cursor<'_>) -> Result<(civil::Time, u8, FractionMark), TemporalError> {
    let parts = cursor.time()?;
    Ok((
        civil::Time::new(parts.hour, parts.minute, parts.second, parts.nanosecond)?,
        parts.fraction_digits,
        parts.mark,
    ))
}

/// Writes `hh:mm:ss[.s…]Z`.
fn write_time(f: &mut fmt::Formatter<'_>, time: civil::Time, fraction_digits: u8, mark: FractionMark) -> fmt::Result {
    write!(f, "{:02}:{:02}:{:02}", time.hour(), time.minute(), time.second())?;
    write_fraction(f, time.subsec_nanosecond(), fraction_digits, mark)?;
    f.write_str("Z")
}

/// Writes `YYYY-MM-DD`.
fn write_date(f: &mut fmt::Formatter<'_>, date: civil::Date) -> fmt::Result {
    write!(f, "{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}

/// A timestamp in UTC: `YYYY-MM-DDThh:mm:ss[.s{1,6}]Z`.
///
/// NGSI-LD clause 4.6.3.
///
/// The number of fraction digits is part of the value, so equal instants written with a different number of
/// digits compare unequal: the type represents a lexical value, and a document read and written back keeps its
/// spelling. A comma before the fraction, which clause 4.6.3 accepts in requests, is read and written as a point;
/// [`DateTime::verbatim`] keeps the comma, and the value still equals the same instant written with a point.
///
/// ```
/// use ngsi_ld::model::DateTime;
///
/// let observed: DateTime = "2022-03-14T12:51:02.000Z".parse()?;
/// assert_eq!(observed.to_string(), "2022-03-14T12:51:02.000Z");
/// assert!("2017-07-29T12:00:04+01:00".parse::<DateTime>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DateTime {
    /// The instant, in UTC.
    timestamp: Timestamp,
    /// The number of fraction digits the lexical form writes, kept so that the text round-trips.
    fraction_digits: u8,
    /// The character that opens the fraction, a spelling that does not distinguish two values.
    mark: FractionMark,
}

impl DateTime {
    /// The current time, with millisecond precision.
    #[must_use]
    pub fn now() -> Self {
        let now = Timestamp::now();
        let millisecond = now.subsec_nanosecond() / 1_000_000 * 1_000_000;
        let timestamp = Timestamp::new(now.as_second(), millisecond).unwrap_or(now);
        Self {
            timestamp,
            fraction_digits: 3,
            mark: FractionMark::Point,
        }
    }

    /// Returns the instant.
    #[must_use]
    pub const fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Returns the number of fraction digits the value is written with.
    #[must_use]
    pub const fn fraction_digits(&self) -> u8 {
        self.fraction_digits
    }

    /// Parses `text` like [`FromStr`], and keeps a comma before the fraction so that it is written back as given.
    ///
    /// The value equals, hashes and orders as the same instant written with a point.
    ///
    /// ```
    /// use ngsi_ld::model::DateTime;
    ///
    /// let observed = DateTime::verbatim("2017-07-29T12:00:04,5Z")?;
    /// assert_eq!(observed.to_string(), "2017-07-29T12:00:04,5Z");
    /// assert_eq!(observed, "2017-07-29T12:00:04.5Z".parse()?);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`TemporalError`] when `text` does not follow the lexical form.
    pub fn verbatim(text: &str) -> Result<Self, TemporalError> {
        let mut cursor = Cursor::new(text);
        let date = civil_date(&mut cursor)?;
        cursor.time_designator()?;
        let (time, fraction_digits, mark) = civil_time(&mut cursor)?;
        cursor.end()?;
        let timestamp = Offset::UTC.to_timestamp(civil::DateTime::from_parts(date, time))?;
        Ok(Self {
            timestamp,
            fraction_digits,
            mark,
        })
    }
}

// No derive parses the clause 4.6.3 lexical form.
impl FromStr for DateTime {
    type Err = TemporalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::verbatim(value).map(|parsed| Self {
            mark: FractionMark::Point,
            ..parsed
        })
    }
}

// No derive writes the clause 4.6.3 lexical form.
impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let civil = Offset::UTC.to_datetime(self.timestamp);
        write_date(f, civil.date())?;
        f.write_str("T")?;
        write_time(f, civil.time(), self.fraction_digits, self.mark)
    }
}

/// Keeps the instant, with the fewest fraction digits (at most six) that write it exactly.
impl TryFrom<Timestamp> for DateTime {
    type Error = TemporalError;

    fn try_from(timestamp: Timestamp) -> Result<Self, Self::Error> {
        let fraction_digits = exact_fraction_digits(timestamp.subsec_nanosecond()).ok_or(TemporalError::SubMicrosecond)?;
        Ok(Self {
            timestamp,
            fraction_digits,
            mark: FractionMark::Point,
        })
    }
}

// No derive projects one field of two; the fraction digits are a spelling the instant does not carry.
impl From<DateTime> for Timestamp {
    fn from(value: DateTime) -> Self {
        value.timestamp
    }
}

/// A calendar date: `YYYY-MM-DD`.
///
/// NGSI-LD clause 4.6.3.
///
/// ```
/// use ngsi_ld::model::Date;
///
/// let date: Date = "2017-07-29".parse()?;
/// assert_eq!(date.to_string(), "2017-07-29");
/// assert!("2017-07".parse::<Date>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::From, derive_more::Into, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Date(civil::Date);

// No derive parses the clause 4.6.3 lexical form.
impl FromStr for Date {
    type Err = TemporalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut cursor = Cursor::new(value);
        let date = civil_date(&mut cursor)?;
        cursor.end()?;
        Ok(Self(date))
    }
}

// No derive writes the clause 4.6.3 lexical form.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_date(f, self.0)
    }
}

/// A time of day in UTC: `hh:mm:ss[.s{1,6}]Z`.
///
/// NGSI-LD clause 4.6.3.
///
/// Like [`DateTime`], the number of fraction digits written is part of the value, and [`Time::verbatim`] keeps a
/// comma before the fraction.
///
/// ```
/// use ngsi_ld::model::Time;
///
/// let time: Time = "12:00:04Z".parse()?;
/// assert_eq!(time.to_string(), "12:00:04Z");
/// assert!("12:00:04+01:00".parse::<Time>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Time {
    /// The time of day, in UTC.
    of_day: civil::Time,
    /// The number of fraction digits the lexical form writes, kept so that the text round-trips.
    fraction_digits: u8,
    /// The character that opens the fraction, a spelling that does not distinguish two values.
    mark: FractionMark,
}

impl Time {
    /// Returns the time of day.
    #[must_use]
    pub const fn time(&self) -> civil::Time {
        self.of_day
    }

    /// Parses `text` like [`FromStr`], and keeps a comma before the fraction so that it is written back as given.
    ///
    /// # Errors
    ///
    /// [`TemporalError`] when `text` does not follow the lexical form.
    pub fn verbatim(text: &str) -> Result<Self, TemporalError> {
        let mut cursor = Cursor::new(text);
        let (time, fraction_digits, mark) = civil_time(&mut cursor)?;
        cursor.end()?;
        Ok(Self {
            of_day: time,
            fraction_digits,
            mark,
        })
    }
}

// No derive parses the clause 4.6.3 lexical form.
impl FromStr for Time {
    type Err = TemporalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::verbatim(value).map(|parsed| Self {
            mark: FractionMark::Point,
            ..parsed
        })
    }
}

// No derive writes the clause 4.6.3 lexical form.
impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_time(f, self.of_day, self.fraction_digits, self.mark)
    }
}

/// Keeps the time of day, with the fewest fraction digits (at most six) that write it exactly.
impl TryFrom<civil::Time> for Time {
    type Error = TemporalError;

    fn try_from(time: civil::Time) -> Result<Self, Self::Error> {
        let fraction_digits = exact_fraction_digits(time.subsec_nanosecond()).ok_or(TemporalError::SubMicrosecond)?;
        Ok(Self {
            of_day: time,
            fraction_digits,
            mark: FractionMark::Point,
        })
    }
}

// No derive projects one field of two; the fraction digits are a spelling the time of day does not carry.
impl From<Time> for civil::Time {
    fn from(value: Time) -> Self {
        value.of_day
    }
}

/// Generates the string conversions serde reads and writes the three types through: the wire form is the
/// lexical form, so deserialising validates and serialising writes `Display`. No derive parses or displays
/// through a `String`.
macro_rules! lexical_string_conversions {
    ($($ty:ty),+) => {$(
        impl TryFrom<String> for $ty {
            type Error = TemporalError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$ty> for String {
            fn from(value: $ty) -> Self {
                value.to_string()
            }
        }
    )+};
}

lexical_string_conversions!(DateTime, Date, Time);

#[cfg(test)]
mod tests;
