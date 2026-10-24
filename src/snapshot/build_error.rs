//! Why a snapshot, a query it holds, the body of Clone Snapshot or a fragment of a snapshot could not be built, and
//! the rules of NGSI-LD Tables 5.2.23-1 and 5.2.41-1 a read document breaks. `build()` returns every problem at once
//! as a [`BuildError`](crate::model::BuildError) of [`Problem`].

use crate::{
    model::MemberProblem,
    snapshot::{SnapshotField, SnapshotPriorityError},
};

/// One rule a snapshot document breaks, or one invalid input given to its builders.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members and restrictions.
///
/// ```
/// use ngsi_ld::snapshot::{EntityQuery, Problem, QueryDocument};
///
/// let error = QueryDocument::<EntityQuery>::builder()
///     .join_level(0)
///     .build()
///     .err();
/// let problems = error.as_ref().map(|error| error.problems());
/// assert!(matches!(problems, Some([Problem::Member(_)])));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Problem {
    /// A member is invalid, empty, missing, or both set and deleted.
    #[error(transparent)]
    Member(#[from] MemberProblem<SnapshotField>),
    /// A snapshot created with neither `snapshotQueries` nor `snapshotTemporalQueries` (Table 5.2.41-1).
    #[error("a snapshot needs snapshotQueries or snapshotTemporalQueries")]
    NoSnapshotQueries,
    /// A `snapshotPriority` that is not from 1 to 10 (Table 5.2.41-1).
    #[error("invalid snapshot priority")]
    Priority {
        /// Why.
        #[source]
        source: SnapshotPriorityError,
    },
}
