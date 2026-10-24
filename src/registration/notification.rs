//! Context source notifications (NGSI-LD clauses 5.3.2, 5.3.3 and 5.11.7): what a broker sends a Context Source
//! Registration Subscription when the registrations matching it change, or periodically.

#[cfg(feature = "linked-data")]
use crate::{
    model::name::mapping::{CarriesContext, MapNames, NameMapping},
    seal::Seal,
};
use crate::{
    model::{
        Context,
        DateTime,
        DocumentError,
        Output,
        RawJson,
        reader::{array, once, required, token, within},
    },
    registration::{CSourceRegistration, Problem, writer::registration_object},
    subscription::{DocumentType, Notification, NotificationId, ReadErrorKind, SubscriptionId},
};
use bon::bon;
use jstrict::{Object, Value as Json};

/// Why a context source notification was sent.
///
/// NGSI-LD clause 5.3.3.
///
/// The enum is exhaustive: clause 5.3.3 enumerates exactly these three reasons.
///
/// ```
/// use ngsi_ld::registration::TriggerReason;
///
/// assert_eq!(
///     TriggerReason::NoLongerMatching.to_string(),
///     "noLongerMatching"
/// );
/// assert_eq!(
///     "newlyMatching".parse::<TriggerReason>()?,
///     TriggerReason::NewlyMatching
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum TriggerReason {
    /// `newlyMatching`: the registrations match the subscription for the first time, on the initial notification,
    /// on their creation, or after an update made them match.
    NewlyMatching,
    /// `updated`: the registrations were notified before and still match after an update.
    Updated,
    /// `noLongerMatching`: the registrations were notified before and match no more, after an update or because
    /// they were deleted.
    NoLongerMatching,
}

/// A context source notification: the registrations a subscription reports and why.
///
/// NGSI-LD clause 5.3.2 defines it. The subscription is a Context Source Registration Subscription.
///
/// The registrations are read as [`CSourceRegistration<Output>`], since a broker writes them; it may filter each to
/// the `information` elements relevant to the subscription (clause 5.11.7).
///
/// ```
/// use ngsi_ld::registration::{CSourceNotification, TriggerReason};
///
/// let text = r#"{
///   "id": "urn:ngsi-ld:Notification:2",
///   "type": "ContextSourceNotification",
///   "subscriptionId": "urn:ngsi-ld:Subscription:registrations",
///   "notifiedAt": "2024-05-01T10:00:00Z",
///   "triggerReason": "newlyMatching",
///   "data": [
///     {
///       "id": "urn:ngsi-ld:ContextSourceRegistration:csr1",
///       "type": "ContextSourceRegistration",
///       "information": [{"entities": [{"type": "Vehicle"}]}],
///       "endpoint": "http://my.csource.org:1026/"
///     }
///   ]
/// }"#;
/// let notification = CSourceNotification::parse(text)?;
/// assert_eq!(notification.trigger_reason(), TriggerReason::NewlyMatching);
/// assert_eq!(notification.data().len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSourceNotification {
    /// `id`, `subscriptionId`, `notifiedAt`, `data` and `@context`, the members it shares with a notification of
    /// entities.
    notification: Notification<Vec<CSourceRegistration<Output>>>,
    /// `triggerReason`.
    trigger_reason: TriggerReason,
}

#[bon]
impl CSourceNotification {
    /// Starts a builder of the context source notification a broker sends (Table 5.3.2-1). `id`,
    /// `subscription_id`, `notified_at`, `trigger_reason` and `data`, the members the table makes mandatory, are
    /// required, which the compiler checks; `type` is always `ContextSourceNotification`.
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::registration::{CSourceNotification, TriggerReason};
    ///
    /// let notification = CSourceNotification::builder()
    ///     .id(iri!("urn:ngsi-ld:Notification:2"))
    ///     .subscription_id(iri!("urn:ngsi-ld:Subscription:registrations"))
    ///     .notified_at("2024-05-01T10:00:00Z".parse()?)
    ///     .trigger_reason(TriggerReason::NoLongerMatching)
    ///     .data(Vec::new())
    ///     .build();
    /// assert_eq!(
    ///     notification.render().to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Notification:2","#,
    ///         r#""type":"ContextSourceNotification","#,
    ///         r#""subscriptionId":"urn:ngsi-ld:Subscription:registrations","#,
    ///         r#""notifiedAt":"2024-05-01T10:00:00Z","#,
    ///         r#""triggerReason":"noLongerMatching","data":[]}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[builder]
    pub fn new(
        #[builder(into)] id: NotificationId,
        #[builder(into)] subscription_id: SubscriptionId,
        notified_at: DateTime,
        trigger_reason: TriggerReason,
        data: Vec<CSourceRegistration<Output>>,
        #[builder(into)] context: Option<Context>,
    ) -> Self {
        Self {
            notification: Notification::builder()
                .id(id)
                .subscription_id(subscription_id)
                .notified_at(notified_at)
                .data(data)
                .maybe_context(context)
                .build(),
            trigger_reason,
        }
    }
}

impl CSourceNotification {
    /// Returns the notification's identifier (`id`).
    #[must_use]
    pub const fn id(&self) -> &NotificationId {
        self.notification.id()
    }

    /// Returns the identifier of the subscription that caused the notification (`subscriptionId`).
    #[must_use]
    pub const fn subscription_id(&self) -> &SubscriptionId {
        self.notification.subscription_id()
    }

    /// Returns when the broker generated the notification (`notifiedAt`).
    #[must_use]
    pub const fn notified_at(&self) -> &DateTime {
        self.notification.notified_at()
    }

    /// Returns why the notification was sent (`triggerReason`).
    #[must_use]
    pub const fn trigger_reason(&self) -> TriggerReason {
        self.trigger_reason
    }

    /// Returns the notified registrations (`data`).
    #[must_use]
    pub fn data(&self) -> &[CSourceRegistration<Output>] {
        self.notification.data()
    }

    /// Takes the notified registrations.
    #[must_use]
    pub fn into_data(self) -> Vec<CSourceRegistration<Output>> {
        self.notification.into_data()
    }

    /// Returns the `@context` of a JSON-LD notification.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.notification.context()
    }

    /// Reads a context source notification (Table 5.3.2-1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, a registration's error under `data` and its index.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        let mut trigger_reason = None;
        let notification = Notification::read_document(text, DocumentType::ContextSourceNotification, registrations, |member, json| {
            if member != "triggerReason" {
                return Ok(Some(json));
            }
            once(&mut trigger_reason, member, within(member, token(json, "triggerReason"))?)?;
            Ok(None)
        })?;
        Ok(Self {
            notification,
            trigger_reason: required(trigger_reason, "triggerReason")?,
        })
    }

    /// Writes the notification in canonical member order: `id`, `type`, `subscriptionId`, `notifiedAt`,
    /// `triggerReason`, `data`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        self.notification.write_document(
            DocumentType::ContextSourceNotification,
            |object: &mut Object| {
                object.push("triggerReason".into(), Json::from(<&str>::from(self.trigger_reason)));
            },
            |data| Json::Array(data.iter().map(|registration| Json::Object(registration_object(registration))).collect()),
        )
    }
}

/// Reads `data` as an array of registrations, placing an error under the registration's index.
fn registrations(json: Json) -> Result<Vec<CSourceRegistration<Output>>, DocumentError<ReadErrorKind<Problem>>> {
    array(json)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| CSourceRegistration::from_json(item).map_err(|error| error.at_index(index)))
        .collect()
}

#[cfg(feature = "linked-data")]
impl MapNames for CSourceNotification {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            notification: self.notification.map_names(mapping)?,
            ..self
        })
    }
}

// Not `carries_context!`: the `@context` belongs to the notification the envelope wraps.
#[cfg(feature = "linked-data")]
impl CarriesContext for CSourceNotification {
    fn set_context(&mut self, _: Seal, context: Option<Context>) {
        self.notification.set_context(Seal::new(), context);
    }
}
