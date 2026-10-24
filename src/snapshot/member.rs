//! The members of a snapshot a fragment changes (NGSI-LD Table 5.2.41-1), named once: a fragment sets or deletes
//! them, and the readers and writers of snapshots spell them through [`SnapshotMember`].

use crate::snapshot::QueryMember;

/// A member of a snapshot a fragment may set or delete (Table 5.2.41-1), in the order the canonical form writes
/// them.
///
/// ```
/// use ngsi_ld::snapshot::SnapshotMember;
///
/// assert_eq!(
///     SnapshotMember::SnapshotLifetime.to_string(),
///     "snapshotLifetime"
/// );
/// ```
///
/// The enum is `#[non_exhaustive]`: later versions of the specification may add members to the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum SnapshotMember {
    /// `snapshotLifetime`.
    SnapshotLifetime,
    /// `snapshotPriority`.
    SnapshotPriority,
    /// `endpoint`.
    Endpoint,
    /// `receiverInfo`.
    ReceiverInfo,
}

/// A member a snapshot problem names: a member of the snapshot or of one of the queries it holds. It displays as the
/// member's own name.
///
/// The enum is exhaustive: a snapshot has no other part.
///
/// ```
/// use ngsi_ld::snapshot::{QueryMember, SnapshotField};
///
/// assert_eq!(
///     SnapshotField::from(QueryMember::JoinLevel).to_string(),
///     "joinLevel"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum SnapshotField {
    /// A member of the snapshot (Table 5.2.41-1).
    Snapshot(SnapshotMember),
    /// A member of a query the snapshot holds (Table 5.2.23-1).
    Query(QueryMember),
}
