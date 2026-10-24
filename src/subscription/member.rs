//! The optional members of a subscription (NGSI-LD Table 5.2.12-1), named once: a fragment sets or deletes them, and
//! the readers and writers of subscriptions spell them through [`SubscriptionMember`].

/// An optional member of a subscription (Table 5.2.12-1), which a fragment may set or delete; they are ordered as
/// the canonical form writes them.
///
/// `id` and `type` cannot be changed, and `notification` can be replaced but not removed, so none of them is a
/// variant. `temporalQ` belongs to a Context Source Registration Subscription only and `csf` to an entity
/// subscription only (Table 5.2.12-1); a fragment of the other kind refuses to delete them.
///
/// ```
/// use ngsi_ld::subscription::SubscriptionMember;
///
/// assert_eq!(
///     SubscriptionMember::WatchedAttributes.to_string(),
///     "watchedAttributes"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum SubscriptionMember {
    /// `subscriptionName`.
    SubscriptionName,
    /// `description`.
    Description,
    /// `entities`.
    Entities,
    /// `watchedAttributes`.
    WatchedAttributes,
    /// `notificationTrigger`.
    NotificationTrigger,
    /// `timeInterval`.
    TimeInterval,
    /// `throttling`.
    Throttling,
    /// `q`.
    Q,
    /// `geoQ`.
    GeoQ,
    /// `scopeQ`.
    ScopeQ,
    /// `temporalQ`.
    TemporalQ,
    /// `csf`.
    Csf,
    /// `datasetId`.
    DatasetId,
    /// `lang`.
    Lang,
    /// `expandValues`.
    ExpandValues,
    /// `jsonKeys`.
    JsonKeys,
    /// `localOnly`.
    LocalOnly,
    /// `splitEntities`.
    SplitEntities,
    /// `isActive`.
    IsActive,
    /// `expiresAt`.
    ExpiresAt,
    /// `jsonldContext`.
    JsonldContext,
    /// `ngsildConformance`.
    NgsildConformance,
}

/// A mandatory member of a subscription (Table 5.2.12-1): those [`SubscriptionMember`] leaves out, as the readers
/// and writers spell them. The table closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub(crate) enum SubscriptionMandatoryMember {
    /// `id`.
    Id,
    /// `type`.
    Type,
    /// `notification`.
    Notification,
}

/// A member of the notification parameters (Table 5.2.14.1-1), as the readers and writers spell it and as a problem
/// names it.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::subscription::NotificationMember;
///
/// assert_eq!(NotificationMember::JoinLevel.to_string(), "joinLevel");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum NotificationMember {
    /// `attributes`.
    Attributes,
    /// `sysAttrs`.
    SysAttrs,
    /// `format`.
    Format,
    /// `showChanges`.
    ShowChanges,
    /// `pick`.
    Pick,
    /// `omit`.
    Omit,
    /// `join`.
    Join,
    /// `joinLevel`.
    JoinLevel,
    /// `endpoint`.
    Endpoint,
    /// `status`.
    Status,
    /// `timesSent`.
    TimesSent,
    /// `timesFailed`.
    TimesFailed,
    /// `lastNotification`.
    LastNotification,
    /// `lastFailure`.
    LastFailure,
    /// `lastSuccess`.
    LastSuccess,
}

/// A member of an endpoint (Table 5.2.15-1), as the readers and writers spell it and as a problem names it.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::subscription::EndpointMember;
///
/// assert_eq!(EndpointMember::ReceiverInfo.to_string(), "receiverInfo");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum EndpointMember {
    /// `uri`.
    Uri,
    /// `accept`.
    Accept,
    /// `timeout`.
    Timeout,
    /// `cooldown`.
    Cooldown,
    /// `receiverInfo`.
    ReceiverInfo,
    /// `notifierInfo`.
    NotifierInfo,
}

/// A member a subscription problem names: a member of the subscription, of its notification parameters, or of
/// their endpoint. It displays as the member's own name.
///
/// The enum is exhaustive: a subscription has no other part.
///
/// ```
/// use ngsi_ld::subscription::{EndpointMember, SubscriptionField};
///
/// assert_eq!(
///     SubscriptionField::from(EndpointMember::Uri).to_string(),
///     "uri"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum SubscriptionField {
    /// A member of the subscription (Table 5.2.12-1).
    Subscription(SubscriptionMember),
    /// A member of the notification parameters (Table 5.2.14.1-1).
    Notification(NotificationMember),
    /// A member of the endpoint (Table 5.2.15-1).
    Endpoint(EndpointMember),
}
