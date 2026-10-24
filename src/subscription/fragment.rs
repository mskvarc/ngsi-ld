//! Subscription fragments (NGSI-LD clauses 5.4, 5.5.8 and 5.8.2): the changes Update Subscription applies to a
//! subscription.

use crate::{
    model::{AttributeName, ConformanceVersion, DatasetId, DateTime, Input, MemberChange, PositiveNumber, RawJson},
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
    subscription::{
        Activation,
        EntitySubscription,
        NotificationParams,
        NotificationTriggers,
        Problem,
        RegistrationSubscription,
        SubscriptionField,
        SubscriptionKind,
        SubscriptionMember,
        builders::SubscriptionFragmentBuilder,
        kind::sealed::{KindChanges, KindMembers},
        writer::fragment_object,
    },
};
use jstrict::{Object, Value as Json};
use std::marker::PhantomData;
use url::Url;

pub(super) mod builder;

/// The changes Update Subscription applies to a subscription (NGSI-LD clauses 5.4, 5.8.2 and 5.11.3).
///
/// Each member a fragment gives replaces the subscription's member, and a member whose value is the NGSI-LD Null
/// removes it (clause 5.4). A fragment names only the optional members it deletes, through [`SubscriptionMember`],
/// so the members a subscription needs cannot be removed.
///
/// It is written with `"type": "Subscription"` and without `id`: the binding names the subscription, whose `id`
/// cannot change (Table 5.2.12-1). A replaced `notification` replaces the whole member. Like an entity fragment,
/// it is only written. `K` is the kind of subscription it changes, as for [`Subscription`]: the builder of
/// [`SubscriptionFragment::builder`] sets `csf`, the one of [`SubscriptionFragment::registration_builder`]
/// `temporalQ`.
///
/// [`Subscription`]: crate::subscription::Subscription
///
/// ```
/// use ngsi_ld::{
///     model::DateTime,
///     subscription::{Activation, SubscriptionFragment, SubscriptionMember},
/// };
///
/// let pause = SubscriptionFragment::builder()
///     .activation(Activation::Paused)
///     .expires_at("2024-05-01T10:00:00Z".parse::<DateTime>()?)
///     .delete(SubscriptionMember::Throttling)
///     .build()?;
/// assert_eq!(
///     pause.render().to_string(),
///     concat!(
///         r#"{"type":"Subscription","throttling":"urn:ngsi-ld:null","#,
///         r#""isActive":false,"expiresAt":"2024-05-01T10:00:00Z"}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionFragment<K: SubscriptionKind = EntitySubscription> {
    /// `subscriptionName`.
    pub(crate) name: Option<MemberChange<String>>,
    /// `description`.
    pub(crate) description: Option<MemberChange<String>>,
    /// `entities`.
    pub(crate) entities: Option<MemberChange<Vec<EntitySelector>>>,
    /// `watchedAttributes`.
    pub(crate) watched_attributes: Option<MemberChange<Vec<AttributeName>>>,
    /// `notificationTrigger`.
    pub(crate) notification_triggers: Option<MemberChange<NotificationTriggers>>,
    /// `timeInterval`, in seconds.
    pub(crate) time_interval: Option<MemberChange<PositiveNumber>>,
    /// `throttling`, in seconds.
    pub(crate) throttling: Option<MemberChange<PositiveNumber>>,
    /// `q`.
    pub(crate) q: Option<MemberChange<Query>>,
    /// `geoQ`.
    pub(crate) geo_q: Option<MemberChange<GeoQuery>>,
    /// `scopeQ`.
    pub(crate) scope_q: Option<MemberChange<ScopeQuery>>,
    /// The changes to the members only kind `K` has: `csf` or `temporalQ`.
    pub(crate) kind_members: <K::Members as KindMembers>::Changes,
    /// `datasetId`.
    pub(crate) dataset_ids: Option<MemberChange<Vec<DatasetId>>>,
    /// `lang`.
    pub(crate) lang: Option<MemberChange<LanguageFilter>>,
    /// `expandValues`.
    pub(crate) expand_values: Option<MemberChange<Vec<AttributeName>>>,
    /// `jsonKeys`.
    pub(crate) json_keys: Option<MemberChange<Vec<AttributeName>>>,
    /// `localOnly`.
    pub(crate) locality: Option<MemberChange<Locality>>,
    /// `splitEntities`.
    pub(crate) split_entities: Option<MemberChange<EntitySplitting>>,
    /// The replacement `notification`, which can be replaced but not removed.
    pub(crate) notification: Option<NotificationParams<Input>>,
    /// `isActive`.
    pub(crate) activation: Option<MemberChange<Activation>>,
    /// `expiresAt`.
    pub(crate) expires_at: Option<MemberChange<DateTime>>,
    /// `jsonldContext`.
    pub(crate) jsonld_context: Option<MemberChange<Url>>,
    /// `ngsildConformance`.
    pub(crate) conformance: Option<MemberChange<ConformanceVersion>>,
    /// The kind of subscription the fragment changes.
    pub(crate) kind: PhantomData<K>,
}

impl SubscriptionFragment {
    /// Starts the builder of a fragment of a context information subscription (clause 5.8.2). Each setter
    /// replaces a member; `delete` removes one, and may be called for several.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`](crate::model::BuildError) listing every invalid attribute name or
    /// URL, every number that is not greater than 0, an empty list Table 5.2.12-1 forbids, a repeated trigger, a member
    /// both set and deleted, a deleted `temporalQ`, and `watchedAttributes` or `throttling` set together with
    /// `timeInterval`.
    pub fn builder() -> SubscriptionFragmentBuilder<EntitySubscription> {
        Self::kind_builder()
    }

    /// Returns the change to the context source filter (`csf`), which only an entity subscription has.
    #[must_use]
    pub fn csf(&self) -> Option<MemberChange<&Query>> {
        self.kind_members.csf.as_ref().map(MemberChange::as_ref)
    }
}

impl SubscriptionFragment<RegistrationSubscription> {
    /// Starts the builder of a fragment of a Context Source Registration Subscription (clause 5.11.3).
    ///
    /// ```
    /// use ngsi_ld::{
    ///     model::MemberChange,
    ///     subscription::{SubscriptionFragment, SubscriptionMember},
    /// };
    ///
    /// let fragment = SubscriptionFragment::registration_builder()
    ///     .delete(SubscriptionMember::TemporalQ)
    ///     .build()?;
    /// assert_eq!(
    ///     fragment.render().to_string(),
    ///     r#"{"type":"Subscription","temporalQ":"urn:ngsi-ld:null"}"#
    /// );
    /// assert_eq!(fragment.temporal_q(), Some(MemberChange::Delete));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`](crate::model::BuildError) as [`SubscriptionFragment::builder`]
    /// does, with a deleted `csf` instead of a deleted `temporalQ`.
    pub fn registration_builder() -> SubscriptionFragmentBuilder<RegistrationSubscription> {
        Self::kind_builder()
    }

    /// Returns the change to the temporal query (`temporalQ`), which only a Context Source Registration
    /// Subscription has.
    #[must_use]
    pub fn temporal_q(&self) -> Option<MemberChange<&TemporalQuery>> {
        self.kind_members.temporal_q.as_ref().map(MemberChange::as_ref)
    }
}

/// Whether `change` sets its member.
fn set<T>(change: Option<&MemberChange<T>>) -> bool {
    change.is_some_and(MemberChange::is_set)
}

member_slots!(
    impl<K: SubscriptionKind> for SubscriptionFragment<K>;
    member: SubscriptionMember, field: SubscriptionField, problem: Problem;
    slots: SubscriptionName => name,
    Description => description,
    Entities => entities,
    WatchedAttributes => watched_attributes,
    NotificationTrigger => notification_triggers,
    TimeInterval => time_interval,
    Throttling => throttling,
    Q => q,
    GeoQ => geo_q,
    ScopeQ => scope_q,
    DatasetId => dataset_ids,
    Lang => lang,
    ExpandValues => expand_values,
    JsonKeys => json_keys,
    LocalOnly => locality,
    SplitEntities => split_entities,
    IsActive => activation,
    ExpiresAt => expires_at,
    JsonldContext => jsonld_context,
    NgsildConformance => conformance;
    delegate: TemporalQ | Csf => kind_members;
);

impl<K: SubscriptionKind> SubscriptionFragment<K> {
    /// Returns the change to the name (`subscriptionName`).
    #[must_use]
    pub fn name(&self) -> Option<MemberChange<&str>> {
        self.name.as_ref().map(|change| change.as_ref().map(String::as_str))
    }

    /// Returns the change to the description (`description`).
    #[must_use]
    pub fn description(&self) -> Option<MemberChange<&str>> {
        self.description.as_ref().map(|change| change.as_ref().map(String::as_str))
    }

    /// Returns the change to the subscribed entities (`entities`).
    #[must_use]
    pub fn entities(&self) -> Option<MemberChange<&[EntitySelector]>> {
        self.entities.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the watched attributes (`watchedAttributes`).
    #[must_use]
    pub fn watched_attributes(&self) -> Option<MemberChange<&[AttributeName]>> {
        self.watched_attributes.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the kinds of change that notify (`notificationTrigger`).
    #[must_use]
    pub fn notification_triggers(&self) -> Option<MemberChange<&NotificationTriggers>> {
        self.notification_triggers.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the period of a periodic subscription, in seconds (`timeInterval`).
    #[must_use]
    pub fn time_interval(&self) -> Option<MemberChange<&PositiveNumber>> {
        self.time_interval.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the least time between two notifications, in seconds (`throttling`).
    #[must_use]
    pub fn throttling(&self) -> Option<MemberChange<&PositiveNumber>> {
        self.throttling.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the query the entities must meet (`q`).
    #[must_use]
    pub fn q(&self) -> Option<MemberChange<&Query>> {
        self.q.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the geoquery the entities must meet (`geoQ`).
    #[must_use]
    pub fn geo_q(&self) -> Option<MemberChange<&GeoQuery>> {
        self.geo_q.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the scope query the entities must meet (`scopeQ`).
    #[must_use]
    pub fn scope_q(&self) -> Option<MemberChange<&ScopeQuery>> {
        self.scope_q.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the dataset ids of the attribute instances notified (`datasetId`).
    #[must_use]
    pub fn dataset_ids(&self) -> Option<MemberChange<&[DatasetId]>> {
        self.dataset_ids.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the language filter (`lang`).
    #[must_use]
    pub fn lang(&self) -> Option<MemberChange<&LanguageFilter>> {
        self.lang.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the attributes whose values are expanded (`expandValues`).
    #[must_use]
    pub fn expand_values(&self) -> Option<MemberChange<&[AttributeName]>> {
        self.expand_values.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the attributes whose values are never expanded (`jsonKeys`).
    #[must_use]
    pub fn json_keys(&self) -> Option<MemberChange<&[AttributeName]>> {
        self.json_keys.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to whether the subscription is limited to the broker's own entities (`localOnly`).
    #[must_use]
    pub const fn locality(&self) -> Option<MemberChange<Locality>> {
        self.locality
    }

    /// Returns the change to whether entities may be split over context sources (`splitEntities`).
    #[must_use]
    pub const fn split_entities(&self) -> Option<MemberChange<EntitySplitting>> {
        self.split_entities
    }

    /// Returns the replacement `notification`, when the fragment replaces it.
    #[must_use]
    pub const fn notification(&self) -> Option<&NotificationParams<Input>> {
        self.notification.as_ref()
    }

    /// Returns the change to whether the subscriber runs or pauses the subscription (`isActive`).
    #[must_use]
    pub const fn activation(&self) -> Option<MemberChange<Activation>> {
        self.activation
    }

    /// Returns the change to when the subscription expires (`expiresAt`).
    #[must_use]
    pub fn expires_at(&self) -> Option<MemberChange<&DateTime>> {
        self.expires_at.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the `@context` notifications use (`jsonldContext`).
    #[must_use]
    pub fn jsonld_context(&self) -> Option<MemberChange<&Url>> {
        self.jsonld_context.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the version of the specification notifications conform to (`ngsildConformance`).
    #[must_use]
    pub const fn conformance(&self) -> Option<MemberChange<ConformanceVersion>> {
        self.conformance
    }

    /// Records the pairs of members a subscription cannot hold when the fragment sets both (Table 5.2.12-1).
    fn check_pairs(&self, problems: &mut Vec<Problem>) {
        if set(self.time_interval.as_ref()) && set(self.watched_attributes.as_ref()) {
            problems.push(Problem::WatchedAttributesWithTimeInterval);
        }
        if set(self.time_interval.as_ref()) && set(self.throttling.as_ref()) {
            problems.push(Problem::ThrottlingWithTimeInterval);
        }
    }

    /// Writes the fragment: `type`, then the members in canonical order, `notification` among them.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// Writes the fragment without an `@context`: the body of a request, whose context the request pipeline
    /// delivers (clause 6.3.5).
    pub(crate) fn to_object(&self) -> Object {
        fragment_object(self)
    }
}
