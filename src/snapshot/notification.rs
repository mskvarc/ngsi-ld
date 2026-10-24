//! The notification a broker sends to the endpoint of a snapshot when its status changes (NGSI-LD clauses 5.3.4 and
//! 5.16.6).

use crate::{
    model::{
        Context,
        DateTime,
        DocumentError,
        RawJson,
        keyword::CONTEXT,
        reader::{context, date_time, identifier, object, once, parse_json, required, token, unexpected, within},
    },
    snapshot::{
        ExecutionResultDetails,
        Problem,
        SettledStatus,
        SnapshotId,
        SnapshotPriority,
        execution::{push_details_list, read_details_list},
        settings::read_priority,
    },
    subscription::{DocumentType, NotificationId, ReadErrorKind, reader::expect_type},
};
use bon::bon;
use jstrict::{Object, Value as Json, object::Entry};

/// A snapshot notification (NGSI-LD clause 5.3.4): sent once the queries of a snapshot have run and after every
/// update of its status (clause 5.16.6), to the snapshot's `endpoint`.
///
/// The status is a [`SettledStatus`]: Table 5.3.4-1 does not list `preparing`. The table spells the details of the
/// temporal queries `temporalSnapshotQueriesDetails`, unlike the `snapshotTemporalQueriesDetails` of a snapshot
/// (Table 5.2.41-2); each document is read with its own spelling. It is read strictly: a member the table does not
/// define fails the read.
///
/// ```
/// use ngsi_ld::snapshot::{SettledStatus, SnapshotNotification};
///
/// let notification = SnapshotNotification::parse(
///     r#"{
///       "id": "urn:ngsi-ld:Notification:1",
///       "type": "SnapshotNotification",
///       "expiresAt": "2030-01-01T00:00:00Z",
///       "notifiedAt": "2024-05-01T10:05:00Z",
///       "snapshotId": "urn:ngsi-ld:Snapshot:1",
///       "snapshotPriority": 5,
///       "snapshotStatus": "success"
///     }"#,
/// )?;
/// assert_eq!(notification.status(), SettledStatus::Success);
/// assert!(!notification.announces_deletion());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotNotification {
    /// `id`.
    id: NotificationId,
    /// `expiresAt`.
    expires_at: DateTime,
    /// `notifiedAt`.
    notified_at: DateTime,
    /// `snapshotId`.
    snapshot_id: SnapshotId,
    /// `snapshotPriority`.
    priority: SnapshotPriority,
    /// `snapshotQueriesDetails`.
    queries_details: Vec<ExecutionResultDetails>,
    /// `snapshotStatus`.
    status: SettledStatus,
    /// `temporalSnapshotQueriesDetails`.
    temporal_queries_details: Vec<ExecutionResultDetails>,
    /// The `@context` of a JSON-LD notification.
    context: Option<Context>,
}

#[bon]
impl SnapshotNotification {
    /// Starts a builder of the snapshot notification a broker sends (Table 5.3.4-1). `id`, `expires_at`,
    /// `notified_at`, `snapshot_id`, `priority` and `status`, the members the table makes mandatory, are required,
    /// which the compiler checks; the details of the queries default to none, and `type` is always
    /// `SnapshotNotification`.
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::snapshot::{SettledStatus, SnapshotNotification, SnapshotPriority};
    ///
    /// let notification = SnapshotNotification::builder()
    ///     .id(iri!("urn:ngsi-ld:Notification:1"))
    ///     .expires_at("2030-01-01T00:00:00Z".parse()?)
    ///     .notified_at("2024-05-01T10:05:00Z".parse()?)
    ///     .snapshot_id(iri!("urn:ngsi-ld:Snapshot:1"))
    ///     .priority(SnapshotPriority::try_new(5)?)
    ///     .status(SettledStatus::Success)
    ///     .build();
    /// assert_eq!(
    ///     notification.render().to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Notification:1","#,
    ///         r#""type":"SnapshotNotification","#,
    ///         r#""expiresAt":"2030-01-01T00:00:00Z","#,
    ///         r#""notifiedAt":"2024-05-01T10:05:00Z","#,
    ///         r#""snapshotId":"urn:ngsi-ld:Snapshot:1","#,
    ///         r#""snapshotPriority":5,"snapshotStatus":"success"}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[builder]
    pub const fn new(
        #[builder(into)] id: NotificationId,
        expires_at: DateTime,
        notified_at: DateTime,
        #[builder(into)] snapshot_id: SnapshotId,
        priority: SnapshotPriority,
        status: SettledStatus,
        #[builder(default)] queries_details: Vec<ExecutionResultDetails>,
        #[builder(default)] temporal_queries_details: Vec<ExecutionResultDetails>,
        #[builder(into)] context: Option<Context>,
    ) -> Self {
        Self {
            id,
            expires_at,
            notified_at,
            snapshot_id,
            priority,
            queries_details,
            status,
            temporal_queries_details,
            context,
        }
    }
}

impl SnapshotNotification {
    /// Returns the identifier of the notification (`id`).
    #[must_use]
    pub const fn id(&self) -> &NotificationId {
        &self.id
    }

    /// Returns when the snapshot expires (`expiresAt`).
    #[must_use]
    pub const fn expires_at(&self) -> &DateTime {
        &self.expires_at
    }

    /// Returns when the notification was generated (`notifiedAt`).
    #[must_use]
    pub const fn notified_at(&self) -> &DateTime {
        &self.notified_at
    }

    /// Returns the snapshot whose status changed (`snapshotId`).
    #[must_use]
    pub const fn snapshot_id(&self) -> &SnapshotId {
        &self.snapshot_id
    }

    /// Returns the priority of the snapshot (`snapshotPriority`).
    #[must_use]
    pub const fn priority(&self) -> SnapshotPriority {
        self.priority
    }

    /// Returns the details of the execution of each query of entities (`snapshotQueriesDetails`).
    #[must_use]
    pub fn queries_details(&self) -> &[ExecutionResultDetails] {
        &self.queries_details
    }

    /// Returns the status of the snapshot (`snapshotStatus`).
    #[must_use]
    pub const fn status(&self) -> SettledStatus {
        self.status
    }

    /// Returns the details of the execution of each temporal query (`temporalSnapshotQueriesDetails`).
    #[must_use]
    pub fn temporal_queries_details(&self) -> &[ExecutionResultDetails] {
        &self.temporal_queries_details
    }

    /// Returns the `@context` of a JSON-LD notification.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Whether the notification announces that the snapshot was deleted: Table 5.3.4-1 says so of an `expiresAt`
    /// before `notifiedAt`.
    #[must_use]
    pub fn announces_deletion(&self) -> bool {
        self.expires_at < self.notified_at
    }

    /// Reads a snapshot notification (Table 5.3.4-1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault: a missing or unexpected member, a member written twice or
    /// out of its type, a status the table does not list.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        let mut id = None;
        let mut typed = None;
        let mut expires_at = None;
        let mut notified_at = None;
        let mut snapshot_id = None;
        let mut priority = None;
        let mut queries_details = None;
        let mut status = None;
        let mut temporal_queries_details = None;
        let mut notification_context = None;
        for Entry { key, value } in object(parse_json(text)?)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, identifier(value, "id"))?)?,
                "type" => once(&mut typed, key, within(key, expect_type(value, DocumentType::SnapshotNotification))?)?,
                "expiresAt" => once(&mut expires_at, key, within(key, date_time(value, "expiresAt"))?)?,
                "notifiedAt" => once(&mut notified_at, key, within(key, date_time(value, "notifiedAt"))?)?,
                "snapshotId" => once(&mut snapshot_id, key, within(key, identifier(value, "snapshotId"))?)?,
                "snapshotPriority" => once(&mut priority, key, within(key, read_priority(value))?)?,
                "snapshotQueriesDetails" => once(&mut queries_details, key, within(key, read_details_list(value))?)?,
                "snapshotStatus" => once(&mut status, key, within(key, token(value, "snapshotStatus"))?)?,
                "temporalSnapshotQueriesDetails" => once(&mut temporal_queries_details, key, within(key, read_details_list(value))?)?,
                CONTEXT => once(&mut notification_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            expires_at: required(expires_at, "expiresAt")?,
            notified_at: required(notified_at, "notifiedAt")?,
            snapshot_id: required(snapshot_id, "snapshotId")?,
            priority: required(priority, "snapshotPriority")?,
            queries_details: queries_details.unwrap_or_default(),
            status: required(status, "snapshotStatus")?,
            temporal_queries_details: temporal_queries_details.unwrap_or_default(),
            context: notification_context,
        })
    }

    /// Writes the notification in the order of Table 5.3.4-1, then its `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Object::new();
        object.push("id".into(), Json::from(self.id.as_str()));
        object.push("type".into(), Json::from(<&str>::from(DocumentType::SnapshotNotification)));
        object.push("expiresAt".into(), Json::from(self.expires_at.to_string()));
        object.push("notifiedAt".into(), Json::from(self.notified_at.to_string()));
        object.push("snapshotId".into(), Json::from(self.snapshot_id.as_str()));
        object.push("snapshotPriority".into(), self.priority.to_number().into_json());
        push_details_list(&mut object, "snapshotQueriesDetails", &self.queries_details);
        object.push("snapshotStatus".into(), Json::from(<&str>::from(self.status)));
        push_details_list(&mut object, "temporalSnapshotQueriesDetails", &self.temporal_queries_details);
        if let Some(context) = &self.context {
            object.push(CONTEXT.into(), context.to_json());
        }
        RawJson::from_json(Json::Object(object))
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    impl for SnapshotNotification => context
);
