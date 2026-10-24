//! Snapshot documents (NGSI-LD clauses 4.3.7, 5.2.41 and 5.16): a snapshot freezes the answer of queries inside the
//! broker, so that the operations aimed at it later work on consistent data (clause 5.5.15).
//!
//! [`Snapshot`] is the document of Create, Retrieve and Update Snapshot Status, in the direction its type parameter
//! names; [`SnapshotClone`] is the body of Clone Snapshot, [`SnapshotFragment`] the body of Update Snapshot Status,
//! and [`SnapshotNotification`] what a broker sends to the snapshot's endpoint. The queries a snapshot holds are
//! [`QueryDocument`]s.
//!
//! ```
//! use ngsi_ld::snapshot::{Snapshot, QueryDocument};
//!
//! let vehicles = QueryDocument::builder().entities(["Vehicle"]).build()?;
//! let snapshot = Snapshot::builder().query(vehicles).build()?;
//! assert_eq!(
//!     snapshot.render().to_string(),
//!     r#"{"type":"Snapshot","snapshotQueries":[{"type":"Query","entities":[{"type":"Vehicle"}]}]}"#
//! );
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! [`SnapshotClone`]: crate::snapshot::SnapshotClone
//! [`SnapshotFragment`]: crate::snapshot::SnapshotFragment
//! [`SnapshotNotification`]: crate::snapshot::SnapshotNotification
//! [`QueryDocument`]: crate::snapshot::QueryDocument
//!
//! # Snapshots
//!
//! [`Snapshot`], [`SnapshotId`], [`SnapshotClone`], [`SnapshotFragment`], [`SnapshotDirection`],
//! [`SnapshotOutputMembers`], [`SnapshotStatus`], [`SettledStatus`], [`SnapshotPriority`], [`SnapshotPriorityError`].
//!
//! # Queries
//!
//! [`QueryDocument`], [`QueryKind`], [`EntityQuery`], [`TemporalEvolutionQuery`], [`AggregationParams`],
//! [`TemporalQueryMembers`].
//!
//! # Execution and notifications
//!
//! [`ExecutionResultDetails`], [`ExecutionStatus`], [`SnapshotNotification`].
//!
//! # Members and errors
//!
//! [`SnapshotMember`], [`SnapshotMemberIter`], [`SnapshotField`], [`QueryMember`], [`QueryMemberIter`],
//! [`AggregationMember`], [`AggregationMemberIter`], [`Problem`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::snapshot::{
    build_error::Problem,
    clone::SnapshotClone,
    direction::{SnapshotDirection, SnapshotOutputMembers},
    execution::{ExecutionResultDetails, ExecutionStatus},
    fragment::SnapshotFragment,
    identifier::SnapshotId,
    member::{SnapshotField, SnapshotMember, SnapshotMemberIter},
    notification::SnapshotNotification,
    query_document::{
        QueryDocument,
        kind::{AggregationParams, EntityQuery, QueryKind, TemporalEvolutionQuery, TemporalQueryMembers},
        member::{AggregationMember, AggregationMemberIter, QueryMember, QueryMemberIter},
    },
    status::{SettledStatus, SnapshotPriority, SnapshotPriorityError, SnapshotStatus},
};
use crate::{
    model::{Context, DateTime, DocumentError, Duration, Input, KeyValuePair, Output, RawJson, keyword::CONTEXT, reader::parse_json},
    snapshot::{reader::read_snapshot, settings::Settings, writer::snapshot_object},
    subscription::ReadErrorKind,
};
use jstrict::Value as Json;
use url::Url;

mod build_error;
mod builder;
pub mod builders;
mod clone;
mod direction;
mod execution;
mod fragment;
mod identifier;
mod member;
#[cfg(feature = "linked-data")]
mod names;
mod notification;
pub(crate) mod query_document;
mod reader;
mod settings;
mod status;
mod writer;

/// A snapshot (NGSI-LD clause 5.2.41) in direction `D`: [`Input`] for what Create Snapshot sends, built with
/// [`Snapshot::builder`], or [`Output`] for what Retrieve and Update Snapshot Status return, with the members only
/// a broker writes (Table 5.2.41-2).
///
/// A snapshot holds queries of entities (`snapshotQueries`) and queries of their temporal evolution
/// (`snapshotTemporalQueries`), two kinds of [`QueryDocument`], so a query of one kind cannot land in the other
/// list (Table 5.2.41-1). It is read strictly: a member the tables do not define fails the read.
///
/// ```
/// use ngsi_ld::snapshot::{Snapshot, QueryDocument};
///
/// let vehicles = QueryDocument::builder().entities(["Vehicle"]).build()?;
/// let snapshot = Snapshot::builder().query(vehicles).lifetime("PT1H".parse()?).priority(7).build()?;
/// assert_eq!(
///     snapshot.render().to_string(),
///     r#"{"type":"Snapshot","snapshotQueries":[{"type":"Query","entities":[{"type":"Vehicle"}]}],"snapshotLifetime":"PT1H","snapshotPriority":7}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot<D: SnapshotDirection> {
    /// `id`.
    id: D::Id,
    /// `snapshotQueries`, `None` when the member is absent. Table 5.2.41-1 sets no minimum length, so a present,
    /// empty list is kept apart from an absent one: it meets the creation rule, which an absent list does not.
    queries: Option<Vec<QueryDocument<EntityQuery>>>,
    /// `snapshotTemporalQueries`, `None` when the member is absent; a present list may be empty, as `queries`.
    temporal_queries: Option<Vec<QueryDocument<TemporalEvolutionQuery>>>,
    /// `snapshotLifetime`, `snapshotPriority`, `endpoint`, `receiverInfo`.
    settings: Settings,
    /// The members only a broker writes.
    output: D::SnapshotOutput,
    /// The `@context` the document carried.
    context: Option<Context>,
}

impl<D: SnapshotDirection> Snapshot<D> {
    /// Returns the queries of entities (`snapshotQueries`), `None` when the member is absent and an empty slice
    /// when it is present and empty.
    #[must_use]
    pub fn queries(&self) -> Option<&[QueryDocument<EntityQuery>]> {
        self.queries.as_deref()
    }

    /// Returns the queries of the temporal evolution of entities (`snapshotTemporalQueries`), `None` when the
    /// member is absent and an empty slice when it is present and empty.
    #[must_use]
    pub fn temporal_queries(&self) -> Option<&[QueryDocument<TemporalEvolutionQuery>]> {
        self.temporal_queries.as_deref()
    }

    /// Returns the lifetime asked for (`snapshotLifetime`); the broker sets the actual expiry (clause 5.16.1.4).
    #[must_use]
    pub const fn lifetime(&self) -> Option<&Duration> {
        self.settings.lifetime.as_ref()
    }

    /// Returns the priority (`snapshotPriority`); absent, it is 5 (clause 5.16.1.4).
    #[must_use]
    pub const fn priority(&self) -> Option<SnapshotPriority> {
        self.settings.priority
    }

    /// Returns the URL snapshot notifications are sent to (`endpoint`, clause 5.16.6).
    #[must_use]
    pub const fn endpoint(&self) -> Option<&Url> {
        self.settings.endpoint.as_ref()
    }

    /// Returns the information added to the notifications for their receiver (`receiverInfo`).
    #[must_use]
    pub fn receiver_info(&self) -> &[KeyValuePair] {
        self.settings.receiver_info.as_deref().unwrap_or_default()
    }

    /// Returns the `@context` the document carried.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Writes the snapshot in canonical member order (see the crate's design document), with its `@context` last
    /// when it has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = snapshot_object(self);
        if let Some(context) = &self.context {
            object.push(CONTEXT.into(), context.to_json());
        }
        RawJson::from_json(Json::Object(object))
    }

    /// Reads a snapshot in direction `D`, keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a member the tables do not define, an
    /// output-only member in an [`Input`] document, a query of the wrong kind, and a request without a query.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        read_snapshot(parse_json(text)?)
    }

    /// Reads a snapshot from JSON already parsed, such as a response body.
    #[cfg(any(feature = "http", feature = "linked-data"))]
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        read_snapshot(json)
    }

    /// Writes the snapshot without its `@context`: the body of a request, whose context the request pipeline
    /// delivers (clause 6.3.5).
    #[cfg(feature = "http")]
    pub(crate) fn to_object(&self) -> jstrict::Object {
        snapshot_object(self)
    }
}

impl Snapshot<Input> {
    /// Returns the identifier the client chose (`id`); `None` lets the broker generate one (clause 5.16.1.4).
    #[must_use]
    pub const fn id(&self) -> Option<&SnapshotId> {
        self.id.as_ref()
    }
}

impl Snapshot<Output> {
    /// Returns the identifier (`id`).
    #[must_use]
    pub const fn id(&self) -> &SnapshotId {
        &self.id
    }

    /// Returns the members only a broker writes (Table 5.2.41-2).
    #[must_use]
    pub const fn output(&self) -> &SnapshotOutputMembers {
        &self.output
    }

    /// Returns the status (`snapshotStatus`): `preparing` while the queries run, then how they ended.
    #[must_use]
    pub const fn status(&self) -> SnapshotStatus {
        self.output.status
    }

    /// Returns when the snapshot expires (`expiresAt`).
    #[must_use]
    pub const fn expires_at(&self) -> &DateTime {
        &self.output.expires_at
    }
}

#[cfg(test)]
mod tests;
