//! The two kinds of subscription that share the Subscription data type (NGSI-LD clauses 5.2.12, 5.8 and 5.11).

use crate::{
    model::{
        DocumentError,
        MemberChange,
        member_change::{delete_member, read_change},
        reader::{once, within},
    },
    query::temporal_query::TemporalQuery,
    seal::Seal,
    subscription::{
        Problem,
        ReadErrorKind,
        SubscriptionField,
        SubscriptionMember,
        reader::temporal_query,
        writer::{push, push_change},
    },
};
use jstrict::{Object, Value as Json};
use std::{error::Error, fmt::Debug, hash::Hash};

/// Keeps the kinds closed, and gives each kind's member block its reading and writing. The traits are public so the
/// public trait can name them, inside a module no caller can reach.
pub(crate) mod sealed {
    use crate::{
        model::DocumentError,
        seal::Seal,
        subscription::{EntitySubscription, Problem, ReadErrorKind, RegistrationSubscription, SubscriptionMember},
    };
    use jstrict::{Object, Value as Json};
    use std::fmt::Debug;

    use std::error::Error;
    /// The sealing trait.
    pub trait Sealed {}

    impl Sealed for EntitySubscription {}
    impl Sealed for RegistrationSubscription {}

    /// The members only one kind of subscription has.
    pub trait KindMembers: Default + Debug + Clone + PartialEq + Eq + Send + Sync {
        /// What a fragment changes in the block: each member set, deleted, or left as it is.
        type Changes: KindChanges;

        /// Reads `member` into the block, or returns `json` when the block has no such member: the member of the
        /// other kind.
        fn read<P: Error>(&mut self, seal: Seal, member: SubscriptionMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>>;

        /// Writes the members the block holds, in canonical order.
        fn write(&self, seal: Seal, object: &mut Object);
    }

    /// The changes a fragment makes to the members only one kind of subscription has.
    pub trait KindChanges: Default + Debug + Clone + PartialEq + Eq + Send + Sync {
        /// Deletes `member`, one of the members only a kind has, recording a member of the other kind and a member
        /// both set and deleted.
        fn delete(&mut self, seal: Seal, member: SubscriptionMember, problems: &mut Vec<Problem>);

        /// Whether the block deletes `member`.
        fn deletes(&self, seal: Seal, member: SubscriptionMember) -> bool;

        /// Reads the change to `member` into the block, or returns `json` when the block has no such member: the
        /// member of the other kind.
        fn read<P: Error>(&mut self, seal: Seal, member: SubscriptionMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>>;

        /// Writes the changes the block holds, in canonical order.
        fn write(&self, seal: Seal, object: &mut Object);
    }
}

/// A kind of subscription: [`EntitySubscription`] or [`RegistrationSubscription`].
///
/// NGSI-LD Table 5.2.12-1.
///
/// A context information subscription ([`EntitySubscription`], clause 5.8) notifies about entities; a Context
/// Source Registration Subscription ([`RegistrationSubscription`], clause 5.11) notifies about the registrations
/// of context sources that can provide them. Table 5.2.12-1 gives the Context Source Registration Subscription one
/// member the other lacks: `temporalQ` is "to be used only in Context Registration Subscriptions". `csf` is a member
/// of both kinds: clause 5.11.2.4 has a Context Source Registration Subscription match the context source filter
/// against the properties of the registrations, and Table 5.2.12-1 says it filters the registrations to be used for
/// Entity Subscriptions. The kind is a type parameter of
/// [`Subscription`](crate::subscription::Subscription) and
/// [`SubscriptionFragment`](crate::subscription::SubscriptionFragment), so `temporalQ` has no setter and no getter on
/// an entity subscription.
///
/// The trait is sealed; its associated type decides the members only that kind has. Both kinds are `Send` and
/// `Sync`, so a subscription of either kind can be sent from any task.
pub trait SubscriptionKind: sealed::Sealed + Debug + Clone + Copy + PartialEq + Eq + Hash + Default + Send + Sync {
    /// The members only this kind has.
    type Members: sealed::KindMembers;
}

/// A context information subscription: its notifications carry entities.
///
/// NGSI-LD clause 5.8 defines it. It has every member of Table 5.2.12-1 but `temporalQ`.
///
/// ```
/// use ngsi_ld::{
///     model::Input,
///     subscription::{EntitySubscription, Subscription},
/// };
///
/// // The default kind: `Subscription<Input>` is an entity subscription.
/// fn sent(_: &Subscription<Input, EntitySubscription>) {}
/// # let _ = sent;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EntitySubscription;

impl SubscriptionKind for EntitySubscription {
    type Members = EntitySubscriptionMembers;
}

/// A Context Source Registration Subscription.
///
/// NGSI-LD clause 5.11 defines it. Its notifications carry context source registrations. It alone has `temporalQ`,
/// and it has `csf` as an entity subscription does.
///
/// ```
/// use ngsi_ld::{
///     model::Output,
///     subscription::{RegistrationSubscription, Subscription},
/// };
///
/// fn retrieved(subscription: &Subscription<Output, RegistrationSubscription>) -> bool {
///     subscription.temporal_q().is_some()
/// }
/// # let _ = retrieved;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RegistrationSubscription;

impl SubscriptionKind for RegistrationSubscription {
    type Members = RegistrationSubscriptionMembers;
}

/// The members only an entity subscription has: none.
///
/// Table 5.2.12-1 defines no member that a context information subscription has and a Context Source Registration
/// Subscription lacks, so the block holds nothing; the kind is still a type parameter so that `temporalQ` has a setter
/// and a getter on a registration subscription only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntitySubscriptionMembers(());

impl sealed::KindMembers for EntitySubscriptionMembers {
    type Changes = EntitySubscriptionChanges;

    fn read<P: Error>(&mut self, _: Seal, _: SubscriptionMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        Ok(Some(json))
    }

    fn write(&self, _: Seal, _: &mut Object) {}
}

/// The changes a fragment of an entity subscription makes to the members only an entity subscription has: none.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntitySubscriptionChanges(());

impl sealed::KindChanges for EntitySubscriptionChanges {
    fn delete(&mut self, _: Seal, member: SubscriptionMember, problems: &mut Vec<Problem>) {
        problems.push(Problem::MemberOfOtherKind { member });
    }

    fn deletes(&self, _: Seal, _: SubscriptionMember) -> bool {
        false
    }

    fn read<P: Error>(&mut self, _: Seal, _: SubscriptionMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        Ok(Some(json))
    }

    fn write(&self, _: Seal, _: &mut Object) {}
}

/// The member only a Context Source Registration Subscription has: `temporalQ`.
///
/// `temporalQ` selects the registrations of context sources providing temporal information (NGSI-LD Table 5.2.12-1,
/// clause 5.11.2.4). Read it through [`Subscription::temporal_q`](crate::subscription::Subscription::temporal_q).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegistrationSubscriptionMembers {
    /// `temporalQ`.
    pub(crate) temporal_q: Option<TemporalQuery>,
}

impl sealed::KindMembers for RegistrationSubscriptionMembers {
    type Changes = RegistrationSubscriptionChanges;

    fn read<P: Error>(&mut self, _: Seal, member: SubscriptionMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        if member != SubscriptionMember::TemporalQ {
            return Ok(Some(json));
        }
        let key = member.into();
        once(&mut self.temporal_q, key, within(key, temporal_query(json))?)?;
        Ok(None)
    }

    fn write(&self, _: Seal, object: &mut Object) {
        push(
            object,
            SubscriptionMember::TemporalQ.into(),
            self.temporal_q.as_ref().map(TemporalQuery::to_json),
        );
    }
}

/// The change a fragment of a registration subscription makes to `temporalQ`.
///
/// The subscription is a Context Source Registration Subscription, and NGSI-LD Table 5.2.12-1 defines `temporalQ`. Read
/// the change through [`SubscriptionFragment::temporal_q`](crate::subscription::SubscriptionFragment::temporal_q).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegistrationSubscriptionChanges {
    /// `temporalQ`.
    pub(crate) temporal_q: Option<MemberChange<TemporalQuery>>,
}

impl sealed::KindChanges for RegistrationSubscriptionChanges {
    fn delete(&mut self, _: Seal, member: SubscriptionMember, problems: &mut Vec<Problem>) {
        if member == SubscriptionMember::TemporalQ {
            delete_member(&mut self.temporal_q, SubscriptionField::from(member), problems);
        } else {
            problems.push(Problem::MemberOfOtherKind { member });
        }
    }

    fn deletes(&self, _: Seal, member: SubscriptionMember) -> bool {
        member == SubscriptionMember::TemporalQ && self.temporal_q.as_ref().is_some_and(MemberChange::is_delete)
    }

    fn read<P: Error>(&mut self, _: Seal, member: SubscriptionMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        if member != SubscriptionMember::TemporalQ {
            return Ok(Some(json));
        }
        let key = member.into();
        once(&mut self.temporal_q, key, within(key, read_change(json, temporal_query))?)?;
        Ok(None)
    }

    fn write(&self, _: Seal, object: &mut Object) {
        push_change(object, SubscriptionMember::TemporalQ.into(), self.temporal_q.as_ref(), |query| {
            Some(query.to_json())
        });
    }
}
