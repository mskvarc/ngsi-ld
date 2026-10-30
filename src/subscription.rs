//! Subscriptions and notifications.
//!
//! They are the documents a subscriber sends and receives, independent of any transport (NGSI-LD clauses 5.2.12 to
//! 5.2.15, 5.2.22, 5.3.1 and 5.8).
//!
//! A [`Subscription`] holds query languages ([`q`](crate::query::q), [`geoquery`](crate::query::geoquery),
//! [`scope_query`](crate::query::scope_query), [`temporal_query`](crate::query::temporal_query)), so this module
//! sits above both `model` and `query`. It needs no feature: a notification receiver reads a
//! [`Notification`] without an HTTP client.
//!
//! `Subscription<Input>` is what Create Subscription sends, `Subscription<Output>` what Retrieve and Query
//! Subscriptions return, and a [`SubscriptionFragment`] what Update Subscription sends. A second type parameter, the
//! [`SubscriptionKind`], tells a context information subscription from a Context Source Registration Subscription
//! (clause 5.11).
//!
//! The readers and errors of this module also serve the context source registrations of
//! [`registration`](crate::registration), which are read and checked the same way.
//!
//! ```
//! use ngsi_ld::{model::Input, subscription::Subscription};
//!
//! let text = concat!(
//!     r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"#,
//!     r#""notification":{"endpoint":{"uri":"http://my.endpoint.org"#,
//!     r#"/notify"}}}"#
//! );
//! let subscription = Subscription::<Input>::parse(text)?;
//! assert_eq!(subscription.render().to_string(), text);
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Subscriptions
//!
//! [`Subscription`], [`SubscriptionFragment`], [`SubscriptionId`], [`SubscriptionKind`], [`EntitySubscription`],
//! [`RegistrationSubscription`], [`EntitySubscriptionMembers`], [`EntitySubscriptionChanges`],
//! [`RegistrationSubscriptionMembers`], [`RegistrationSubscriptionChanges`], [`SubscriptionDirection`],
//! [`SubscriptionOutputMembers`], [`SubscriptionStatus`], [`Activation`], [`NotificationTriggers`],
//! [`NotificationTrigger`], [`NotificationTriggerIter`], [`NotificationTriggersError`].
//!
//! # Notification parameters
//!
//! [`NotificationParams`], [`Endpoint`], [`ChangeReporting`], [`SystemAttributeReporting`], [`NotificationStatus`],
//! [`NotificationOutputMembers`].
//!
//! # Notifications
//!
//! [`Notification`], [`NotificationId`], [`DocumentType`], [`MqttNotification`].
//!
//! # Members and errors
//!
//! [`SubscriptionMember`], [`SubscriptionMemberIter`], [`NotificationMember`], [`NotificationMemberIter`],
//! [`EndpointMember`], [`EndpointMemberIter`], [`SubscriptionField`], [`Problem`], [`ReadErrorKind`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::subscription::{
    build_error::Problem,
    direction::{NotificationOutputMembers, SubscriptionDirection, SubscriptionOutputMembers},
    document_type::DocumentType,
    endpoint::Endpoint,
    fragment::SubscriptionFragment,
    identifier::{NotificationId, SubscriptionId},
    kind::{
        EntitySubscription,
        EntitySubscriptionChanges,
        EntitySubscriptionMembers,
        RegistrationSubscription,
        RegistrationSubscriptionChanges,
        RegistrationSubscriptionMembers,
        SubscriptionKind,
    },
    member::{EndpointMember, EndpointMemberIter, NotificationMember, NotificationMemberIter, SubscriptionField, SubscriptionMember, SubscriptionMemberIter},
    mqtt::MqttNotification,
    notification::Notification,
    notification_params::{ChangeReporting, NotificationParams, NotificationStatus, SystemAttributeReporting},
    read_error::ReadErrorKind,
    status::{Activation, SubscriptionStatus},
    trigger::{NotificationTrigger, NotificationTriggerIter, NotificationTriggers, NotificationTriggersError},
};
use crate::{
    model::{
        AttributeName,
        ConformanceVersion,
        Context,
        DatasetId,
        DateTime,
        DocumentError,
        Input,
        Output,
        PositiveNumber,
        RawJson,
        keyword::CONTEXT,
        written_url::WrittenUrl,
    },
    query::{
        EntitySelector,
        EntitySplitting,
        Locality,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        q::Query,
        scope_query::ScopeQuery,
        temporal_query::TemporalQuery,
    },
    subscription::{reader::read_subscription, writer::subscription_object},
};
use jstrict::Value as Json;
use url::Url;

pub(crate) mod attribute_list;
mod build_error;
mod builder;

pub mod builders;
pub(crate) mod direction;
mod document_type;
pub(crate) mod endpoint;
mod fragment;
mod identifier;
mod kind;
mod member;
mod mqtt;
#[cfg(feature = "linked-data")]
mod names;
mod notification;
mod notification_params;
pub(crate) mod read_error;
pub(crate) mod reader;
mod rules;
mod status;
mod trigger;
pub(crate) mod writer;

/// A subscription to changes of entities.
///
/// NGSI-LD clause 5.2.12 defines it: which entities and changes a subscriber is notified about, and how.
///
/// `D` is the direction: [`Input`] for what Create Subscription sends, with an optional `id`; [`Output`] for what
/// Retrieve and Query Subscriptions return, with an `id` and the members only a broker writes. `K` is the kind:
/// [`EntitySubscription`], the default, or [`RegistrationSubscription`] for a Context Source Registration
/// Subscription (clause 5.11). Build one with [`Subscription::builder`] or [`Subscription::registration_builder`];
/// read one with [`Subscription::parse`].
///
/// ```
/// use ngsi_ld::{
///     query::q,
///     subscription::{Endpoint, NotificationParams, Subscription},
/// };
///
/// let subscription = Subscription::builder()
///     .entities(["Vehicle"])
///     .watched_attributes(["speed"])
///     .q(q::attr("speed").gt(50).build()?)
///     .notification(
///         NotificationParams::builder(
///             Endpoint::builder("http://my.endpoint.org/notify").build()?,
///         )
///         .build()?,
///     )
///     .build()?;
/// assert_eq!(
///     subscription.render().to_string(),
///     concat!(
///         r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"#,
///         r#""watchedAttributes":["speed"],"q":"speed>50","#,
///         r#""notification":{"endpoint":{"uri":"http://my.endpoint.org"#,
///         r#"/notify"}}}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription<D: SubscriptionDirection, K: SubscriptionKind = EntitySubscription> {
    /// `id`.
    pub(crate) id: D::Id,
    /// `subscriptionName`.
    pub(crate) name: Option<String>,
    /// `description`.
    pub(crate) description: Option<String>,
    /// `entities`; empty when absent.
    pub(crate) entities: Vec<EntitySelector>,
    /// `watchedAttributes`; empty when absent.
    pub(crate) watched_attributes: Vec<AttributeName>,
    /// `notificationTrigger`.
    pub(crate) notification_triggers: Option<NotificationTriggers>,
    /// `timeInterval`, in seconds.
    pub(crate) time_interval: Option<PositiveNumber>,
    /// `throttling`, in seconds.
    pub(crate) throttling: Option<PositiveNumber>,
    /// `q`.
    pub(crate) q: Option<Query>,
    /// `geoQ`.
    pub(crate) geo_q: Option<GeoQuery>,
    /// `scopeQ`.
    pub(crate) scope_q: Option<ScopeQuery>,
    /// The members only kind `K` has: `temporalQ`.
    pub(crate) kind_members: K::Members,
    /// `csf`.
    pub(crate) csf: Option<Query>,
    /// `datasetId`; an empty array is kept, as no rule of Table 5.2.12-1 refuses it.
    pub(crate) dataset_ids: Option<Vec<DatasetId>>,
    /// `lang`.
    pub(crate) lang: Option<LanguageFilter>,
    /// `expandValues`; empty when absent.
    pub(crate) expand_values: Vec<AttributeName>,
    /// `jsonKeys`; empty when absent.
    pub(crate) json_keys: Vec<AttributeName>,
    /// `localOnly`.
    pub(crate) locality: Locality,
    /// `splitEntities`.
    pub(crate) split_entities: EntitySplitting,
    /// `notification`.
    pub(crate) notification: NotificationParams<D>,
    /// `isActive`.
    pub(crate) activation: Activation,
    /// `expiresAt`.
    pub(crate) expires_at: Option<DateTime>,
    /// `jsonldContext`.
    pub(crate) jsonld_context: Option<WrittenUrl>,
    /// `ngsildConformance`.
    pub(crate) conformance: Option<ConformanceVersion>,
    /// The members only a broker writes; `()` in a request.
    pub(crate) output: D::SubscriptionOutput,
    /// The `@context` the document carried (clause 5.2.3).
    pub(crate) context: Option<Context>,
}

impl<D: SubscriptionDirection, K: SubscriptionKind> Subscription<D, K> {
    /// Returns the name (`subscriptionName`).
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the description (`description`).
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the subscribed entities (`entities`); empty when the member is absent.
    #[must_use]
    pub fn entities(&self) -> &[EntitySelector] {
        &self.entities
    }

    /// Returns the watched attributes (`watchedAttributes`); empty means every attribute.
    #[must_use]
    pub fn watched_attributes(&self) -> &[AttributeName] {
        &self.watched_attributes
    }

    /// Returns the kinds of change that notify (`notificationTrigger`); `None` means the default,
    /// `attributeCreated` and `attributeUpdated`.
    #[must_use]
    pub const fn notification_triggers(&self) -> Option<&NotificationTriggers> {
        self.notification_triggers.as_ref()
    }

    /// Returns the period of a periodic subscription, in seconds (`timeInterval`).
    #[must_use]
    pub const fn time_interval(&self) -> Option<&PositiveNumber> {
        self.time_interval.as_ref()
    }

    /// Returns the least time between two notifications, in seconds (`throttling`).
    #[must_use]
    pub const fn throttling(&self) -> Option<&PositiveNumber> {
        self.throttling.as_ref()
    }

    /// Returns the query the entities must meet (`q`).
    #[must_use]
    pub const fn q(&self) -> Option<&Query> {
        self.q.as_ref()
    }

    /// Returns the geoquery the entities must meet (`geoQ`).
    #[must_use]
    pub const fn geo_q(&self) -> Option<&GeoQuery> {
        self.geo_q.as_ref()
    }

    /// Returns the scope query the entities must meet (`scopeQ`).
    #[must_use]
    pub const fn scope_q(&self) -> Option<&ScopeQuery> {
        self.scope_q.as_ref()
    }

    /// Returns the context source filter the registrations used for the subscription must meet (`csf`, Table
    /// 5.2.12-1). A Context Source Registration Subscription matches it against the properties of the registrations
    /// (clause 5.11.2.4).
    #[must_use]
    pub const fn csf(&self) -> Option<&Query> {
        self.csf.as_ref()
    }

    /// Returns the dataset ids of the attribute instances notified (`datasetId`).
    #[must_use]
    pub fn dataset_ids(&self) -> Option<&[DatasetId]> {
        self.dataset_ids.as_deref()
    }

    /// Returns the language filter (`lang`).
    #[must_use]
    pub const fn lang(&self) -> Option<&LanguageFilter> {
        self.lang.as_ref()
    }

    /// Returns the attributes whose values are expanded (`expandValues`).
    #[must_use]
    pub fn expand_values(&self) -> &[AttributeName] {
        &self.expand_values
    }

    /// Returns the attributes whose values are never expanded (`jsonKeys`).
    #[must_use]
    pub fn json_keys(&self) -> &[AttributeName] {
        &self.json_keys
    }

    /// Returns whether the subscription is limited to the broker's own entities (`localOnly`).
    #[must_use]
    pub const fn locality(&self) -> Locality {
        self.locality
    }

    /// Returns whether entities may be split over context sources (`splitEntities`).
    #[must_use]
    pub const fn split_entities(&self) -> EntitySplitting {
        self.split_entities
    }

    /// Returns how the subscription notifies (`notification`).
    #[must_use]
    pub const fn notification(&self) -> &NotificationParams<D> {
        &self.notification
    }

    /// Returns whether the subscriber runs or pauses the subscription (`isActive`).
    #[must_use]
    pub const fn activation(&self) -> Activation {
        self.activation
    }

    /// Returns when the subscription expires (`expiresAt`).
    #[must_use]
    pub const fn expires_at(&self) -> Option<&DateTime> {
        self.expires_at.as_ref()
    }

    /// Returns the `@context` notifications use (`jsonldContext`).
    #[must_use]
    pub const fn jsonld_context(&self) -> Option<&Url> {
        match &self.jsonld_context {
            Some(context) => Some(context.url()),
            None => None,
        }
    }

    /// Returns the version of the specification notifications conform to (`ngsildConformance`).
    #[must_use]
    pub const fn conformance(&self) -> Option<ConformanceVersion> {
        self.conformance
    }

    /// Returns the `@context` the document carried.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Writes the subscription with its members in the order Annex C.4 writes them: `id`, `type`, `subscriptionName`,
    /// `description`, `entities`, `watchedAttributes`, `notificationTrigger`, `timeInterval`, `throttling`, `q`,
    /// `geoQ`, `scopeQ`, `temporalQ`, `csf`, `datasetId`, `lang`, `expandValues`, `jsonKeys`, `localOnly`,
    /// `splitEntities`, `notification`, `isActive`, `expiresAt`, `jsonldContext`, `ngsildConformance`, `status`,
    /// `createdAt`, `modifiedAt`, and its `@context` last when it has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = subscription_object(self);
        if let Some(context) = &self.context {
            object.push(CONTEXT.into(), context.to_json());
        }
        RawJson::from_json(Json::Object(object))
    }

    /// Reads a subscription of kind `K` in direction `D`, checking the rules of Tables 5.2.12-1, 5.2.14.1-1 and
    /// 5.2.15-1 and keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a member the tables do not define, a member of
    /// the other kind of subscription, an output-only member in an [`Input`] document, and the broken rules.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        read_subscription(text)
    }

    /// Reads a subscription from JSON already parsed, as [`parse`](Self::parse) reads its text.
    #[cfg(any(feature = "http", feature = "linked-data"))]
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        reader::read_subscription_json(json)
    }

    /// Writes the subscription without its `@context`: the body of a request, whose context the request pipeline
    /// delivers (clause 6.3.5).
    #[cfg(feature = "http")]
    pub(crate) fn to_object(&self) -> jstrict::Object {
        subscription_object(self)
    }
}

impl<D: SubscriptionDirection> Subscription<D, RegistrationSubscription> {
    /// Returns the temporal query the registrations must match (`temporalQ`, Table 5.2.12-1): with it, only
    /// registrations of context sources providing temporal information match (clause 5.11.2.4).
    #[must_use]
    pub const fn temporal_q(&self) -> Option<&TemporalQuery> {
        self.kind_members.temporal_q.as_ref()
    }
}

impl<K: SubscriptionKind> Subscription<Input, K> {
    /// Returns the identifier the subscriber chose (`id`); `None` lets the broker generate one (clause 5.8.1.4).
    #[must_use]
    pub const fn id(&self) -> Option<&SubscriptionId> {
        self.id.as_ref()
    }
}

impl<K: SubscriptionKind> Subscription<Output, K> {
    /// Returns the identifier (`id`).
    #[must_use]
    pub const fn id(&self) -> &SubscriptionId {
        &self.id
    }

    /// Returns the members only a broker writes: `status`, `createdAt`, `modifiedAt`.
    #[must_use]
    pub const fn output(&self) -> &SubscriptionOutputMembers {
        &self.output
    }

    /// Converts a retrieved subscription into one a client can send again, keeping its `id` and `@context` and
    /// dropping the members only a broker writes (clauses 5.2.12 and 5.2.14.2).
    #[must_use]
    pub fn into_input(self) -> Subscription<Input, K> {
        Subscription {
            id: Some(self.id),
            name: self.name,
            description: self.description,
            entities: self.entities,
            watched_attributes: self.watched_attributes,
            notification_triggers: self.notification_triggers,
            time_interval: self.time_interval,
            throttling: self.throttling,
            q: self.q,
            geo_q: self.geo_q,
            scope_q: self.scope_q,
            kind_members: self.kind_members,
            csf: self.csf,
            dataset_ids: self.dataset_ids,
            lang: self.lang,
            expand_values: self.expand_values,
            json_keys: self.json_keys,
            locality: self.locality,
            split_entities: self.split_entities,
            notification: self.notification.into_input(),
            activation: self.activation,
            expires_at: self.expires_at,
            jsonld_context: self.jsonld_context,
            conformance: self.conformance,
            output: (),
            context: self.context,
        }
    }
}

#[cfg(test)]
mod tests;
