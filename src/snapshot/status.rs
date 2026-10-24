//! The state of a snapshot: its status (NGSI-LD Tables 5.2.41-2 and 5.3.4-1) and its priority (Table 5.2.41-1).

use crate::model::Number;
use std::str::FromStr;

/// The status a snapshot settles in once its queries have run.
///
/// It is the `snapshotStatus` of NGSI-LD clause 5.16.1.4, and the status a snapshot notification reports (Table
/// 5.3.4-1).
///
/// The enum is exhaustive: Tables 5.2.41-2 and 5.3.4-1 close the set.
///
/// ```
/// use ngsi_ld::snapshot::SettledStatus;
///
/// assert_eq!("partial".parse::<SettledStatus>()?, SettledStatus::Partial);
/// assert_eq!(SettledStatus::Empty.to_string(), "empty");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum SettledStatus {
    /// `success`: every query ran and yielded at least one result.
    Success,
    /// `partial`: at least one query ran and yielded a result.
    Partial,
    /// `failure`: no query ran successfully.
    Failure,
    /// `empty`: at least one query ran, and none yielded a result.
    Empty,
}

/// The status of a snapshot.
///
/// It is the `snapshotStatus` of NGSI-LD Table 5.2.41-2: `preparing` while its queries run (clauses 5.16.1.4 and
/// 5.16.2.4), then the status it settles in.
///
/// The enum is exhaustive: Table 5.2.41-2 closes the set.
///
/// ```
/// use ngsi_ld::snapshot::{SettledStatus, SnapshotStatus};
///
/// assert_eq!(
///     "preparing".parse::<SnapshotStatus>()?,
///     SnapshotStatus::Preparing
/// );
/// assert_eq!(
///     "success".parse::<SnapshotStatus>()?,
///     SnapshotStatus::Settled(SettledStatus::Success)
/// );
/// assert_eq!(
///     SnapshotStatus::Settled(SettledStatus::Failure).to_string(),
///     "failure"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
pub enum SnapshotStatus {
    /// `preparing`: the queries of the snapshot are still running.
    #[display("preparing")]
    Preparing,
    /// The queries have run, with this outcome.
    #[display("{_0}")]
    Settled(SettledStatus),
}

// The token `preparing` or a settled status; no derive reads a variant that wraps another enum's tokens.
impl FromStr for SnapshotStatus {
    type Err = strum::ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text == "preparing" {
            Ok(Self::Preparing)
        } else {
            text.parse().map(Self::Settled)
        }
    }
}

/// The priority of a snapshot (the `snapshotPriority` of Table 5.2.41-1): an integer from 1 (minimum) to 10
/// (maximum), which a broker low on resources considers when it chooses snapshots to delete (clause 5.5.15). A
/// snapshot created without one has the priority 5 (clause 5.16.1.4).
///
/// ```
/// use ngsi_ld::snapshot::SnapshotPriority;
///
/// let priority = SnapshotPriority::try_new(7)?;
/// assert_eq!(priority.into_inner(), 7);
/// assert!(SnapshotPriority::try_new(11).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(greater_or_equal = 1, less_or_equal = 10),
    derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Display, TryFrom)
)]
pub struct SnapshotPriority(u8);

impl SnapshotPriority {
    /// The priority an integer states: one from 1 to 10. An integer beyond the range of `u8` is refused on the side
    /// of the range it lies on.
    pub(crate) fn from_integer(integer: i64) -> Result<Self, SnapshotPriorityError> {
        // Saturating keeps the side of the range, which decides the error.
        Self::try_new(u8::try_from(integer).unwrap_or(if integer.is_negative() { u8::MIN } else { u8::MAX }))
    }

    /// The priority as a JSON number.
    pub(crate) fn to_number(self) -> Number {
        Number::from(self.into_inner())
    }
}
