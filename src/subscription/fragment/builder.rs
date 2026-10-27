//! The builder of a subscription fragment (NGSI-LD clauses 5.4 and 5.8.2): one builder for both kinds, with the
//! member only one kind has set through a setter that exists on that kind's builder alone.

use crate::{
    model::{
        AttributeNameDraft,
        BuildError,
        ConformanceVersion,
        DatasetId,
        DateTime,
        Input,
        MemberChange,
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
        NotificationParams,
        NotificationTrigger,
        NotificationTriggers,
        Problem,
        RegistrationSubscription,
        SubscriptionField,
        SubscriptionFragment,
        SubscriptionKind,
        SubscriptionMember,
        kind::sealed::KindMembers,
    },
};
use bon::bon;
use std::marker::PhantomData;

#[bon]
impl<K: SubscriptionKind> SubscriptionFragment<K> {
    /// Starts a builder of kind `K`; [`SubscriptionFragment::builder`] and
    /// [`SubscriptionFragment::registration_builder`] name the kind.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid attribute name or URL, every number that is not
    /// greater than 0, an empty list Table 5.2.12-1 forbids, a repeated trigger, a member both set and deleted, a
    /// deleted member of the other kind of subscription, and `watchedAttributes` or `throttling` set together with
    /// `timeInterval`.
    #[builder(start_fn(name = kind_builder, vis = "pub(crate)"))]
    pub fn new(
        #[builder(field)] deletions: Vec<SubscriptionMember>,
        #[builder(field)] kind_members: <K::Members as KindMembers>::Changes,
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
        #[builder(with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())] expand_values: Option<
            Vec<AttributeNameDraft>,
        >,
        #[builder(with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())] json_keys: Option<
            Vec<AttributeNameDraft>,
        >,
        locality: Option<Locality>,
        split_entities: Option<EntitySplitting>,
        notification: Option<NotificationParams<Input>>,
        activation: Option<Activation>,
        expires_at: Option<DateTime>,
        #[builder(into)] jsonld_context: Option<UrlDraft>,
        ngsild_conformance: Option<ConformanceVersion>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let field = SubscriptionField::Subscription;
        let member = field(SubscriptionMember::Entities);
        let entities = entities.map(|drafts| {
            resolve_required_drafts(member, Some(drafts), &mut problems, |index, source| {
                Problem::from(MemberProblem::InvalidSelectedType { member, index, source })
            })
        });
        let watched_attributes =
            watched_attributes.map(|names| resolve_required_names(field(SubscriptionMember::WatchedAttributes), Some(names), &mut problems));
        let notification_triggers = notification_triggers.and_then(|chosen| {
            NotificationTriggers::try_from_iter(chosen)
                .map_err(|source| problems.push(Problem::Triggers { source }))
                .ok()
        });
        let time_interval = resolve_positive(field(SubscriptionMember::TimeInterval), time_interval, &mut problems);
        let throttling = resolve_positive(field(SubscriptionMember::Throttling), throttling, &mut problems);
        let expand_values = expand_values.map(|names| resolve_names(field(SubscriptionMember::ExpandValues), names, &mut problems));
        let json_keys = json_keys.map(|names| resolve_names(field(SubscriptionMember::JsonKeys), names, &mut problems));
        let jsonld_context = jsonld_context.and_then(|url| url.resolve(field(SubscriptionMember::JsonldContext), &mut problems));
        let mut fragment = Self {
            name: subscription_name.map(MemberChange::Set),
            description: description.map(MemberChange::Set),
            entities: entities.map(MemberChange::Set),
            watched_attributes: watched_attributes.map(MemberChange::Set),
            notification_triggers: notification_triggers.map(MemberChange::Set),
            time_interval: time_interval.map(MemberChange::Set),
            throttling: throttling.map(MemberChange::Set),
            q: q.map(MemberChange::Set),
            geo_q: geo_q.map(MemberChange::Set),
            scope_q: scope_q.map(MemberChange::Set),
            kind_members,
            csf: csf.map(MemberChange::Set),
            dataset_ids: dataset_ids.map(MemberChange::Set),
            lang: lang.map(MemberChange::Set),
            expand_values: expand_values.map(MemberChange::Set),
            json_keys: json_keys.map(MemberChange::Set),
            locality: locality.map(MemberChange::Set),
            split_entities: split_entities.map(MemberChange::Set),
            notification,
            activation: activation.map(MemberChange::Set),
            expires_at: expires_at.map(MemberChange::Set),
            jsonld_context: jsonld_context.map(MemberChange::Set),
            conformance: ngsild_conformance.map(MemberChange::Set),
            kind: PhantomData,
        };
        for member in deletions {
            fragment.delete(member, &mut problems);
        }
        fragment.check_pairs(&mut problems);
        BuildError::finish(problems, fragment)
    }
}

impl<S: subscription_fragment_builder::State> SubscriptionFragmentBuilder<RegistrationSubscription, S> {
    /// Replaces the temporal query (`temporalQ`), which only a Context Source Registration Subscription has.
    pub fn temporal_q(mut self, temporal_q: TemporalQuery) -> Self {
        self.kind_members.temporal_q = Some(MemberChange::Set(temporal_q));
        self
    }
}

impl<K: SubscriptionKind, S: subscription_fragment_builder::State> SubscriptionFragmentBuilder<K, S> {
    /// Deletes `member` from the subscription: the fragment writes the NGSI-LD Null for it (clause 5.4).
    pub fn delete(mut self, member: SubscriptionMember) -> Self {
        self.deletions.push(member);
        self
    }
}
