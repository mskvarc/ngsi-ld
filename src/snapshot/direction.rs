//! What each direction of a snapshot document carries: the identifier a client may leave out, the members only a
//! broker writes (NGSI-LD Table 5.2.41-2), and the rule only a creation obeys (Table 5.2.41-1).

use crate::{
    model::{DateTime, DocumentError, Input, Output},
    snapshot::{ExecutionResultDetails, SnapshotId, SnapshotStatus},
    subscription::{
        ReadErrorKind,
        direction::sealed::{IdMember, Sealed, required_id},
    },
};
use std::{error::Error, fmt::Debug};

/// The crate-only side of the snapshot directions: the sealing supertrait, and the members and rules each
/// direction reads and writes.
pub(crate) mod sealed {

    use crate::{
        model::{
            DateTime,
            DocumentError,
            reader::{date_time, once, required, token, within},
            representation::write::push_display,
        },
        snapshot::{
            ExecutionResultDetails,
            SnapshotOutputMembers,
            SnapshotStatus,
            execution::{push_details_list, read_details_list},
        },
        subscription::ReadErrorKind,
    };
    use jstrict::{Object, Value as Json};
    use std::error::Error;

    /// Whether a document of a direction must hold a query: a request creates the snapshot, and Table 5.2.41-1
    /// requires "at least one of `snapshotQueries` or `snapshotTemporalQueries` [...] when creating the snapshot".
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum QueryRequirement {
        /// At least one query of either kind.
        AtLeastOne,
        /// Any number of queries, none included.
        Any,
    }

    /// The members of a snapshot only a broker writes, as a direction carries them.
    pub trait OutputMembers: Sized {
        /// What a reader gathers before the members are complete.
        type Pending: Default;

        /// Whether a document of the direction must hold a query.
        const QUERIES: QueryRequirement;

        /// Reads `member` into `pending` when it is one of these members, or gives the value back.
        fn read<P: Error>(pending: &mut Self::Pending, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>>;

        /// The members gathered, or the error naming the mandatory one missing.
        fn complete<P: Error>(pending: Self::Pending) -> Result<Self, DocumentError<ReadErrorKind<P>>>;

        /// Writes the members in the order of Table 5.2.41-2.
        fn write(&self, object: &mut Object);
    }

    impl OutputMembers for () {
        type Pending = ();

        const QUERIES: QueryRequirement = QueryRequirement::AtLeastOne;

        fn read<P: Error>((): &mut (), _: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
            Ok(Some(json))
        }

        fn complete<P: Error>((): ()) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
            Ok(())
        }

        fn write(&self, _: &mut Object) {}
    }

    /// The members of Table 5.2.41-2 gathered so far.
    #[derive(Debug, Default)]
    pub struct PendingOutput {
        /// `snapshotStatus`.
        status: Option<SnapshotStatus>,
        /// `snapshotQueriesDetails`.
        queries_details: Option<Vec<ExecutionResultDetails>>,
        /// `snapshotTemporalQueriesDetails`.
        temporal_queries_details: Option<Vec<ExecutionResultDetails>>,
        /// `createdAt`.
        created_at: Option<DateTime>,
        /// `modifiedAt`.
        modified_at: Option<DateTime>,
        /// `expiresAt`.
        expires_at: Option<DateTime>,
        /// `lastUsedAt`.
        last_used_at: Option<DateTime>,
    }

    impl OutputMembers for SnapshotOutputMembers {
        type Pending = PendingOutput;

        const QUERIES: QueryRequirement = QueryRequirement::Any;

        fn read<P: Error>(pending: &mut PendingOutput, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
            match member {
                "snapshotStatus" => once(&mut pending.status, member, within(member, token(json, "snapshotStatus"))?)?,
                "snapshotQueriesDetails" => once(&mut pending.queries_details, member, within(member, read_details_list(json))?)?,
                "snapshotTemporalQueriesDetails" => once(&mut pending.temporal_queries_details, member, within(member, read_details_list(json))?)?,
                "createdAt" => once(&mut pending.created_at, member, within(member, date_time(json, "createdAt"))?)?,
                "modifiedAt" => once(&mut pending.modified_at, member, within(member, date_time(json, "modifiedAt"))?)?,
                "expiresAt" => once(&mut pending.expires_at, member, within(member, date_time(json, "expiresAt"))?)?,
                "lastUsedAt" => once(&mut pending.last_used_at, member, within(member, date_time(json, "lastUsedAt"))?)?,
                _ => return Ok(Some(json)),
            }
            Ok(None)
        }

        fn complete<P: Error>(pending: PendingOutput) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
            Ok(Self {
                status: required(pending.status, "snapshotStatus")?,
                queries_details: pending.queries_details.unwrap_or_default(),
                temporal_queries_details: pending.temporal_queries_details.unwrap_or_default(),
                created_at: pending.created_at,
                modified_at: pending.modified_at,
                expires_at: required(pending.expires_at, "expiresAt")?,
                last_used_at: pending.last_used_at,
            })
        }

        fn write(&self, object: &mut Object) {
            object.push("snapshotStatus".into(), Json::from(self.status.to_string()));
            push_details_list(object, "snapshotQueriesDetails", &self.queries_details);
            push_details_list(object, "snapshotTemporalQueriesDetails", &self.temporal_queries_details);
            push_display(object, "createdAt", self.created_at.as_ref());
            push_display(object, "modifiedAt", self.modified_at.as_ref());
            object.push("expiresAt".into(), Json::from(self.expires_at.to_string()));
            push_display(object, "lastUsedAt", self.last_used_at.as_ref());
        }
    }
}

/// A direction of a snapshot document: [`Input`] for what Create Snapshot sends (clause 5.16.1.3), [`Output`] for
/// what Retrieve and Update Snapshot Status return (clauses 5.16.3.5 and 5.16.4.5).
///
/// The trait is sealed by the same seal as the subscription directions; its associated types decide what a
/// direction carries.
pub trait SnapshotDirection: Sealed + Debug + Clone + PartialEq + Eq {
    /// The snapshot's `id`: optional in a request, since the broker generates one when it is absent (clause
    /// 5.16.1.4), and always present in a response.
    type Id: IdMember<SnapshotId> + Debug + Clone + PartialEq + Eq;
    /// The members only a broker writes; `()` in a request.
    type SnapshotOutput: sealed::OutputMembers + Debug + Clone + PartialEq + Eq;
}

impl SnapshotDirection for Input {
    type Id = Option<SnapshotId>;
    type SnapshotOutput = ();
}

impl SnapshotDirection for Output {
    type Id = SnapshotId;
    type SnapshotOutput = SnapshotOutputMembers;
}

impl IdMember<SnapshotId> for SnapshotId {
    fn from_read<P: Error>(id: Option<SnapshotId>) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
        required_id(id)
    }

    fn get(&self) -> Option<&SnapshotId> {
        Some(self)
    }
}

/// The members of a snapshot only a broker writes (Table 5.2.41-2): `snapshotStatus`, the details of each query's
/// execution, `createdAt`, `modifiedAt`, `expiresAt` and `lastUsedAt`.
///
/// A request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SnapshotOutputMembers {
    /// `snapshotStatus`.
    pub status: SnapshotStatus,
    /// `snapshotQueriesDetails`: one result per query of `snapshotQueries`, in their order.
    pub queries_details: Vec<ExecutionResultDetails>,
    /// `snapshotTemporalQueriesDetails`: one result per query of `snapshotTemporalQueries`, in their order.
    pub temporal_queries_details: Vec<ExecutionResultDetails>,
    /// `createdAt`.
    pub created_at: Option<DateTime>,
    /// `modifiedAt`.
    pub modified_at: Option<DateTime>,
    /// `expiresAt`: when the snapshot expires, set by the broker from the requested `snapshotLifetime` (clause
    /// 5.16.1.4).
    pub expires_at: DateTime,
    /// `lastUsedAt`: when the snapshot was last used (clause 4.8).
    pub last_used_at: Option<DateTime>,
}
