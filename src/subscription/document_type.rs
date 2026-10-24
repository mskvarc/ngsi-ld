//! The `type` member of the documents of the subscription family.

/// The `type` of a document of the subscription family.
///
/// It is spelled as the specification writes it. A reader refuses a document whose `type` is another one
/// ([`ReadErrorKind::WrongType`](crate::subscription::ReadErrorKind::WrongType)).
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add documents.
///
/// ```
/// use ngsi_ld::subscription::DocumentType;
///
/// assert_eq!(
///     DocumentType::ContextSourceRegistration.to_string(),
///     "ContextSourceRegistration"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum DocumentType {
    /// `Subscription` (clause 5.2.12), of a subscription to entities or to context source registrations.
    Subscription,
    /// `Notification` (clause 5.3.1).
    Notification,
    /// `ContextSourceRegistration` (clause 5.2.9).
    ContextSourceRegistration,
    /// `ContextSourceNotification` (clause 5.3.2).
    ContextSourceNotification,
    /// `Snapshot` (clause 5.2.41).
    Snapshot,
    /// `SnapshotNotification` (clause 5.3.4).
    SnapshotNotification,
    /// `Query` (clause 5.2.23), the query a snapshot is taken with.
    Query,
}
