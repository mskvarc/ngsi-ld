//! ISO 8601 durations (NGSI-LD clause 4.5.19.1): the period of an aggregation (`aggrPeriodDuration`) and the
//! suggested lifetime of an entity map (`entityMapLifetime`).

use jiff::{
    Span,
    SpanFieldwise,
    fmt::{
        StdFmtWrite,
        temporal::{SpanParser, SpanPrinter},
    },
};
use std::{fmt, str::FromStr};

/// Why a value is not a duration.
// `jiff::Error` carries no equality, so neither can the error that chains it.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum DurationError {
    /// The text is not an ISO 8601 duration.
    #[error("a duration is an ISO 8601 duration")]
    Syntax {
        /// The duration parser's error.
        #[source]
        source: jiff::Error,
    },
    /// The duration is negative.
    #[error("a duration cannot be negative")]
    Negative,
    /// A number of weeks is combined with another unit; clause 4.5.19.1 writes weeks alone, as `P[n]W`.
    #[error("a duration in weeks has no other unit")]
    WeeksCombined,
}

/// A duration (NGSI-LD clause 4.5.19.1): `P[n]Y[n]M[n]DT[n]H[n]M[n]S` or `P[n]W`, never negative. Zero is a
/// duration (`PT0S`, `P0D`), which an aggregation reads as the whole time range of its temporal query.
///
/// The duration keeps its units as written, so `PT90M` is printed back as `PT90M`, not `PT1H30M`; zero, which has no
/// unit left, is printed `PT0S`.
///
/// ```
/// use ngsi_ld::model::Duration;
///
/// let period: Duration = "PT4M".parse()?;
/// assert_eq!(period.to_string(), "PT4M");
/// assert!("-PT1H".parse::<Duration>().is_err());
/// assert!("P1W2D".parse::<Duration>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration(SpanFieldwise);

impl Duration {
    /// Returns the duration.
    #[must_use]
    pub const fn span(self) -> Span {
        self.0.0
    }

    /// Whether the duration is zero.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0.0.is_zero()
    }
}

/// Whether `span` holds weeks and another unit.
fn combines_weeks(span: Span) -> bool {
    span.get_weeks() != 0
        && (span.get_years() != 0
            || span.get_months() != 0
            || span.get_days() != 0
            || span.get_hours() != 0
            || span.get_minutes() != 0
            || span.get_seconds() != 0
            || span.get_milliseconds() != 0
            || span.get_microseconds() != 0
            || span.get_nanoseconds() != 0)
}

impl TryFrom<Span> for Duration {
    type Error = DurationError;

    fn try_from(span: Span) -> Result<Self, Self::Error> {
        if span.is_negative() {
            Err(DurationError::Negative)
        } else if combines_weeks(span) {
            Err(DurationError::WeeksCombined)
        } else {
            Ok(Self(span.fieldwise()))
        }
    }
}

// The ISO 8601 parser of `jiff`; `Span`'s own `FromStr` also accepts its friendly format, which no NGSI-LD member
// allows.
impl FromStr for Duration {
    type Err = DurationError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let span = SpanParser::new().parse_span(text).map_err(|source| DurationError::Syntax { source })?;
        Self::try_from(span)
    }
}

// The ISO 8601 printer of `jiff`, with upper-case designators.
impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SpanPrinter::new().print_span(&self.0.0, StdFmtWrite(f)).map_err(|_| fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Duration, DurationError};
    use jiff::ToSpan;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_duration_round_trips_with_its_units() {
        for text in ["PT4M", "PT90M", "P3Y6M4DT12H30M5S", "P2W", "PT0S"] {
            assert_eq!(text.parse::<Duration>().unwrap().to_string(), text);
        }
        assert_eq!("P0D".parse::<Duration>().unwrap().to_string(), "PT0S");
        assert_eq!(Duration::try_from(2.hours()).unwrap().to_string(), "PT2H");
        assert!("PT0S".parse::<Duration>().unwrap().is_zero());
    }

    #[test]
    fn negative_combined_and_friendly_durations_are_refused() {
        assert!(matches!("-PT1H".parse::<Duration>(), Err(DurationError::Negative)));
        assert!(matches!("P1W2D".parse::<Duration>(), Err(DurationError::WeeksCombined)));
        assert!(matches!("4 minutes".parse::<Duration>(), Err(DurationError::Syntax { .. })));
        assert!(matches!("P".parse::<Duration>(), Err(DurationError::Syntax { .. })));
    }
}
