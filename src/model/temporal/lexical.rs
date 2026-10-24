//! The lexical forms of clause 4.6.3: `YYYY-MM-DD`, `hh:mm:ss[.s{1,6}]Z` and their combination.
//!
//! The parser is deliberately strict: every component has a fixed number of digits, the separator characters
//! are exactly the ones the clause names, the zone designator is `Z` and nothing else, and reduced
//! representations are rejected. Range checks (month 13, February 30) are left to `jiff`.

use crate::model::{TemporalError, TemporalToken};
use std::fmt;

/// The maximum number of fractional second digits clause 4.6.3 allows.
pub(super) const MAX_FRACTION_DIGITS: u8 = 6;

/// A calendar date as written: year, month, day.
pub(super) struct DateParts {
    /// The year as written, four digits.
    pub(super) year: i16,
    /// The month as written, two digits, not range-checked here.
    pub(super) month: i8,
    /// The day of the month as written, two digits, not range-checked here.
    pub(super) day: i8,
}

/// A time of day as written: hour, minute, second, nanoseconds and the number of fraction digits written.
pub(super) struct TimeParts {
    /// The hour as written, two digits.
    pub(super) hour: i8,
    /// The minute as written, two digits.
    pub(super) minute: i8,
    /// The second as written, two digits.
    pub(super) second: i8,
    /// The fraction of the second, in nanoseconds; `0` when no fraction is written.
    pub(super) nanosecond: i32,
    /// The number of fraction digits written, at most `MAX_FRACTION_DIGITS`.
    pub(super) fraction_digits: u8,
}

/// A read position over the input.
pub(super) struct Cursor<'a> {
    /// The text being read, as bytes.
    input: &'a [u8],
    /// The index of the next byte to read.
    position: usize,
}

impl<'a> Cursor<'a> {
    /// Starts reading `input` from its first byte.
    pub(super) const fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    /// The error for finding something other than `expected` at the current position.
    const fn unexpected(&self, expected: TemporalToken) -> TemporalError {
        TemporalError::Unexpected {
            expected,
            position: self.position,
        }
    }

    /// Reads exactly `count` ASCII digits as a number.
    fn digits(&mut self, count: usize) -> Result<i32, TemporalError> {
        (0..count).try_fold(0_i32, |number, _| match self.input.get(self.position) {
            Some(&byte) if byte.is_ascii_digit() => {
                self.position += 1;
                Ok(number * 10 + i32::from(byte - b'0'))
            }
            Some(_) | None => Err(self.unexpected(TemporalToken::Digit)),
        })
    }

    /// Consumes `expected`, or fails.
    fn literal(&mut self, expected: u8) -> Result<(), TemporalError> {
        if self.input.get(self.position) == Some(&expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.unexpected(TemporalToken::Character(char::from(expected))))
        }
    }

    /// Reads `YYYY-MM-DD`.
    pub(super) fn date(&mut self) -> Result<DateParts, TemporalError> {
        let year = self.digits(4)?;
        self.literal(b'-')?;
        let month = self.digits(2)?;
        self.literal(b'-')?;
        let day = self.digits(2)?;
        Ok(DateParts {
            year: narrow(year)?,
            month: narrow(month)?,
            day: narrow(day)?,
        })
    }

    /// Reads `hh:mm:ss[(.|,)s{1,6}]Z`.
    pub(super) fn time(&mut self) -> Result<TimeParts, TemporalError> {
        let hour = self.digits(2)?;
        self.literal(b':')?;
        let minute = self.digits(2)?;
        self.literal(b':')?;
        let second = self.digits(2)?;
        let (nanosecond, fraction_digits) = self.fraction()?;
        self.zone()?;
        Ok(TimeParts {
            hour: narrow(hour)?,
            minute: narrow(minute)?,
            second: narrow(second)?,
            nanosecond,
            fraction_digits,
        })
    }

    /// Reads the optional fraction. Clause 4.6.3 accepts a comma in requests "for compatibility reasons".
    fn fraction(&mut self) -> Result<(i32, u8), TemporalError> {
        if !matches!(self.input.get(self.position), Some(b'.' | b',')) {
            return Ok((0, 0));
        }
        self.position += 1;
        let start = self.position;
        let mut nanosecond = 0_i32;
        let mut count = 0_u8;
        while let Some(&byte) = self.input.get(self.position).filter(|byte| byte.is_ascii_digit()) {
            if count == MAX_FRACTION_DIGITS {
                return Err(TemporalError::TooManyFractionDigits { position: start });
            }
            nanosecond = nanosecond * 10 + i32::from(byte - b'0');
            count += 1;
            self.position += 1;
        }
        if count == 0 {
            return Err(self.unexpected(TemporalToken::Digit));
        }
        Ok((nanosecond * 10_i32.pow(9 - u32::from(count)), count))
    }

    /// Reads the zone designator, which clause 4.6.3 fixes to `Z`.
    fn zone(&mut self) -> Result<(), TemporalError> {
        match self.input.get(self.position) {
            Some(b'Z') => {
                self.position += 1;
                Ok(())
            }
            Some(b'+' | b'-') => Err(TemporalError::NotUtc { position: self.position }),
            Some(_) | None => Err(self.unexpected(TemporalToken::Character('Z'))),
        }
    }

    /// Consumes the `T` between date and time.
    pub(super) fn time_designator(&mut self) -> Result<(), TemporalError> {
        self.literal(b'T')
    }

    /// Fails unless the whole input has been read.
    pub(super) const fn end(&self) -> Result<(), TemporalError> {
        if self.position == self.input.len() {
            Ok(())
        } else {
            Err(self.unexpected(TemporalToken::End))
        }
    }
}

/// Narrows a parsed component to the integer type `jiff` takes for it.
fn narrow<T: TryFrom<i32>>(value: i32) -> Result<T, TemporalError> {
    T::try_from(value).map_err(|_| TemporalError::ComponentOutOfRange)
}

/// Writes the fraction of `nanosecond` with exactly `digits` digits, or nothing when `digits` is zero.
pub(super) fn write_fraction(f: &mut fmt::Formatter<'_>, nanosecond: i32, digits: u8) -> fmt::Result {
    if digits == 0 {
        return Ok(());
    }
    let scaled = nanosecond / 10_i32.pow(9 - u32::from(digits));
    write!(f, ".{scaled:0width$}", width = usize::from(digits))
}

/// The smallest number of fraction digits, at most six, that writes `nanosecond` exactly; `None` when the value
/// has sub-microsecond precision.
pub(super) fn exact_fraction_digits(nanosecond: i32) -> Option<u8> {
    (0..=MAX_FRACTION_DIGITS).find(|&digits| nanosecond % 10_i32.pow(9 - u32::from(digits)) == 0)
}
