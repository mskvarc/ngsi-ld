//! The time intervals of a registration of a context source providing temporal information (NGSI-LD clause
//! 5.2.11).

use crate::model::DateTime;

/// A time interval: its start, and its end unless it is open.
///
/// NGSI-LD clause 5.2.11.
///
/// A registration's `observationInterval` and `managementInterval` are time intervals; whether the end follows the
/// start is not a rule of Table 5.2.11-1, so it is not checked.
///
/// ```
/// use ngsi_ld::{model::DateTime, registration::TimeInterval};
///
/// let start: DateTime = "2017-11-29T14:53:15Z".parse()?;
/// let open = TimeInterval::starting_at(start);
/// assert_eq!(open.end_at(), None);
/// let closed = TimeInterval::between(start, "2017-12-29T14:53:15Z".parse()?);
/// assert_eq!(closed.start_at(), &start);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeInterval {
    /// `startAt`.
    start_at: DateTime,
    /// `endAt`; `None` for an open interval.
    end_at: Option<DateTime>,
}

impl TimeInterval {
    /// The open interval starting at `start`.
    #[must_use]
    pub const fn starting_at(start: DateTime) -> TimeInterval {
        TimeInterval { start_at: start, end_at: None }
    }

    /// The interval from `start` to `end`.
    #[must_use]
    pub const fn between(start: DateTime, end: DateTime) -> TimeInterval {
        TimeInterval {
            start_at: start,
            end_at: Some(end),
        }
    }

    /// Returns the start of the interval (`startAt`).
    #[must_use]
    pub const fn start_at(&self) -> &DateTime {
        &self.start_at
    }

    /// Returns the end of the interval (`endAt`); `None` when it is open.
    #[must_use]
    pub const fn end_at(&self) -> Option<&DateTime> {
        self.end_at.as_ref()
    }
}
