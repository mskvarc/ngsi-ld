//! The builder of a subscription a client sends (NGSI-LD clauses 5.2.12, 5.8.1 and 5.11.2): one builder for both
//! kinds, with the member only one kind has set through a setter that exists on that kind's builder alone.

use crate::{
    model::{
        AttributeNameDraft,
        BuildError,
        ConformanceVersion,
        Context,
        DatasetId,
        DateTime,
        Input,
        MemberProblem,
        Number,
        UrlDraft,
        draft::resolve_required_drafts,
        name::draft::{resolve_names, resolve_required_names},
        value::positive_number::resolve_positive,
    },
    query::{
        EntitySelectorDraft,
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
        NotificationTrigger,
        NotificationTriggers,
        Problem,
        RegistrationSubscription,
        Subscription,
        SubscriptionField,
        SubscriptionId,
        SubscriptionKind,
        SubscriptionMember,
        rules,
    },
};
use bon::bon;

#[bon]
impl<K: SubscriptionKind> Subscription<Input, K> {
    /// Starts a builder of kind `K`; [`Subscription::builder`] and [`Subscription::registration_builder`] name the
    /// kind.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid attribute name or URL, every number that is not
    /// greater than 0, every empty list Table 5.2.12-1 forbids, a repeated trigger, and every rule of Tables
    /// 5.2.12-1, 5.2.14.1-1 and 5.2.33-1 the members break together.
    #[builder(start_fn(name = kind_builder, vis = "pub(crate)"))]
    pub fn new(
        #[builder(field)] kind_members: K::Members,
        #[builder(into)] id: Option<SubscriptionId>,
        #[builder(into)] subscription_name: Option<String>,
        #[builder(into)] description: Option<String>,
        #[builder(with = |selectors: impl IntoIterator<Item = impl Into<EntitySelectorDraft>>| selectors.into_iter().map(Into::into).collect())]
        entities: Option<Vec<EntitySelectorDraft>>,
        #[builder(with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())]
        watched_attributes: Option<Vec<AttributeNameDraft>>,
        #[builder(with = |triggers: impl IntoIterator<Item = NotificationTrigger>| triggers.into_iter().collect())] notification_triggers: Option<
            Vec<NotificationTrigger>,
        >,
        #[builder(into)] time_interval: Option<Number>,
        #[builder(into)] throttling: Option<Number>,
        q: Option<Query>,
        geo_q: Option<GeoQuery>,
        scope_q: Option<ScopeQuery>,
        csf: Option<Query>,
        #[builder(with = |ids: impl IntoIterator<Item = DatasetId>| ids.into_iter().collect())] dataset_ids: Option<Vec<DatasetId>>,
        lang: Option<LanguageFilter>,
        #[builder(default, with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())]
        expand_values: Vec<AttributeNameDraft>,
        #[builder(default, with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())] json_keys: Vec<
            AttributeNameDraft,
        >,
        #[builder(default)] locality: Locality,
        #[builder(default)] split_entities: EntitySplitting,
        notification: NotificationParams<Input>,
        #[builder(default)] activation: Activation,
        expires_at: Option<DateTime>,
        #[builder(into)] jsonld_context: Option<UrlDraft>,
        ngsild_conformance: Option<ConformanceVersion>,
        context: Option<Context>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let field = SubscriptionField::Subscription;
        let member = field(SubscriptionMember::Entities);
        let entities = resolve_required_drafts(member, entities, &mut problems, |index, source| {
            Problem::from(MemberProblem::InvalidSelectedType { member, index, source })
        });
        let subscription = Self {
            id,
            name: subscription_name,
            description,
            entities,
            watched_attributes: resolve_required_names(field(SubscriptionMember::WatchedAttributes), watched_attributes, &mut problems),
            notification_triggers: notification_triggers.and_then(|triggers| {
                NotificationTriggers::try_from_iter(triggers)
                    .map_err(|source| problems.push(Problem::Triggers { source }))
                    .ok()
            }),
            time_interval: resolve_positive(field(SubscriptionMember::TimeInterval), time_interval, &mut problems),
            throttling: resolve_positive(field(SubscriptionMember::Throttling), throttling, &mut problems),
            q,
            geo_q,
            scope_q,
            kind_members,
            csf,
            dataset_ids,
            lang,
            expand_values: resolve_names(field(SubscriptionMember::ExpandValues), expand_values, &mut problems),
            json_keys: resolve_names(field(SubscriptionMember::JsonKeys), json_keys, &mut problems),
            locality,
            split_entities,
            notification,
            activation,
            expires_at,
            jsonld_context: jsonld_context.and_then(|url| url.resolve(field(SubscriptionMember::JsonldContext), &mut problems)),
            conformance: ngsild_conformance,
            output: (),
            context,
        };
        problems.extend(rules::subscription_problems(&subscription));
        BuildError::finish(problems, subscription)
    }
}

impl Subscription<Input> {
    /// Starts the builder of a context information subscription (clause 5.8.1). `notification` is required; every
    /// other member is optional, and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid attribute name or URL, every number that is not
    /// greater than 0, every empty list Table 5.2.12-1 forbids, a repeated trigger, and every rule of Tables
    /// 5.2.12-1, 5.2.14.1-1 and 5.2.33-1 the members break together.
    pub fn builder() -> SubscriptionBuilder<EntitySubscription> {
        Self::kind_builder()
    }
}

impl Subscription<Input, RegistrationSubscription> {
    /// Starts the builder of a Context Source Registration Subscription (clause 5.11.2), whose notifications carry
    /// the registrations matching it. `notification` is required; every other member is optional, and every
    /// problem is reported by `build()`.
    ///
    /// ```
    /// use ngsi_ld::{
    ///     query::temporal_query::TemporalQuery,
    ///     subscription::{Endpoint, NotificationParams, Subscription},
    /// };
    ///
    /// let subscription = Subscription::registration_builder()
    ///     .entities(["Vehicle"])
    ///     .temporal_q(TemporalQuery::after("2024-05-01T10:00:00Z".parse()?))
    ///     .notification(
    ///         NotificationParams::builder(
    ///             Endpoint::builder("http://my.endpoint.org/registrations").build()?,
    ///         )
    ///         .build()?,
    ///     )
    ///     .build()?;
    /// assert!(subscription.temporal_q().is_some());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] as [`Subscription::builder`] does.
    pub fn registration_builder() -> SubscriptionBuilder<RegistrationSubscription> {
        Self::kind_builder()
    }
}

impl<S: subscription_builder::State> SubscriptionBuilder<RegistrationSubscription, S> {
    /// Sets the temporal query the registrations must match (`temporalQ`, Table 5.2.12-1), which only a Context
    /// Source Registration Subscription has.
    pub fn temporal_q(mut self, temporal_q: TemporalQuery) -> Self {
        self.kind_members.temporal_q = Some(temporal_q);
        self
    }

    /// Sets `temporalQ` when there is one.
    pub fn maybe_temporal_q(mut self, temporal_q: Option<TemporalQuery>) -> Self {
        self.kind_members.temporal_q = temporal_q;
        self
    }
}
