//! What each direction of a subscription document carries: the identifier a client may leave out, and the
//! members only a broker writes (NGSI-LD Tables 5.2.12-2, 5.2.2-1 and 5.2.14.2-1).

use crate::{
    model::{DateTime, Input, Output},
    subscription::{NotificationStatus, SubscriptionId, SubscriptionStatus},
};
use std::fmt::Debug;

/// Keeps the directions closed, and gives each direction's associated types the reading and writing of their
/// members. The traits are public so the public trait can name them, inside a module no caller can reach.
pub(crate) mod sealed {
    use crate::{
        model::{
            DocumentError,
            Input,
            Output,
            reader::{count, date_time, once, required, token, within},
            representation::write::push_display,
        },
        seal::Seal,
        subscription::{NotificationOutputMembers, ReadErrorKind, SubscriptionId, SubscriptionOutputMembers},
    };
    use jstrict::{Object, Value as Json};

    use std::error::Error;
    /// The sealing trait.
    pub trait Sealed {}

    impl Sealed for Input {}
    impl Sealed for Output {}

    /// The `id` of a direction, an identifier `I`: optional in a request, required in a response.
    pub trait IdMember<I>: Sized {
        /// The `id` a document held, or the error of its absence.
        fn from_read<P: Error>(seal: Seal, id: Option<I>) -> Result<Self, DocumentError<ReadErrorKind<P>>>;

        /// The `id`, when there is one.
        fn get(&self, seal: Seal) -> Option<&I>;
    }

    impl<I> IdMember<I> for Option<I> {
        fn from_read<P: Error>(_: Seal, id: Option<I>) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
            Ok(id)
        }

        fn get(&self, _: Seal) -> Option<&I> {
            self.as_ref()
        }
    }

    impl IdMember<SubscriptionId> for SubscriptionId {
        fn from_read<P: Error>(_: Seal, id: Option<SubscriptionId>) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
            required_id(id)
        }

        fn get(&self, _: Seal) -> Option<&SubscriptionId> {
            Some(self)
        }
    }

    /// The `id` a response requires, or the error of its absence.
    pub(crate) fn required_id<I, P: Error>(id: Option<I>) -> Result<I, DocumentError<ReadErrorKind<P>>> {
        required(id, "id")
    }

    /// A block of members only a broker writes.
    pub trait OutputBlock: Default {
        /// Reads `member` into the block, or returns `json` when the block has no such member.
        fn read<P: Error>(&mut self, seal: Seal, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>>;

        /// Writes the members the block holds, in canonical order.
        fn write(&self, seal: Seal, object: &mut Object);
    }

    /// A request has no member only a broker writes.
    impl OutputBlock for () {
        fn read<P: Error>(&mut self, _: Seal, _: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
            Ok(Some(json))
        }

        fn write(&self, _: Seal, _: &mut Object) {}
    }

    impl OutputBlock for SubscriptionOutputMembers {
        fn read<P: Error>(&mut self, _: Seal, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
            match member {
                "status" => once(&mut self.status, member, within(member, token(json, "status"))?)?,
                "createdAt" => once(&mut self.created_at, member, within(member, date_time(json, "createdAt"))?)?,
                "modifiedAt" => once(&mut self.modified_at, member, within(member, date_time(json, "modifiedAt"))?)?,
                _ => return Ok(Some(json)),
            }
            Ok(None)
        }

        fn write(&self, _: Seal, object: &mut Object) {
            if let Some(status) = self.status {
                object.push("status".into(), Json::from(<&str>::from(status)));
            }
            push_display(object, "createdAt", self.created_at.as_ref());
            push_display(object, "modifiedAt", self.modified_at.as_ref());
        }
    }

    impl OutputBlock for NotificationOutputMembers {
        fn read<P: Error>(&mut self, _: Seal, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
            match member {
                "status" => once(&mut self.status, member, within(member, token(json, "status"))?)?,
                "timesSent" => once(&mut self.times_sent, member, within(member, count(json, "timesSent"))?)?,
                "timesFailed" => once(&mut self.times_failed, member, within(member, count(json, "timesFailed"))?)?,
                "lastNotification" => once(&mut self.last_notification, member, within(member, date_time(json, "lastNotification"))?)?,
                "lastSuccess" => once(&mut self.last_success, member, within(member, date_time(json, "lastSuccess"))?)?,
                "lastFailure" => once(&mut self.last_failure, member, within(member, date_time(json, "lastFailure"))?)?,
                _ => return Ok(Some(json)),
            }
            Ok(None)
        }

        fn write(&self, _: Seal, object: &mut Object) {
            if let Some(status) = self.status {
                object.push("status".into(), Json::from(<&str>::from(status)));
            }
            if let Some(sent) = self.times_sent {
                object.push("timesSent".into(), Json::from(sent));
            }
            if let Some(failed) = self.times_failed {
                object.push("timesFailed".into(), Json::from(failed));
            }
            push_display(object, "lastNotification", self.last_notification.as_ref());
            push_display(object, "lastSuccess", self.last_success.as_ref());
            push_display(object, "lastFailure", self.last_failure.as_ref());
        }
    }
}

/// A direction of a subscription document.
///
/// It is [`Input`] for what Create Subscription sends (NGSI-LD clause 5.8.1.3), and [`Output`] for what Retrieve and
/// Query Subscriptions return (clauses 5.8.3 and 5.8.4).
///
/// The trait is sealed; its associated types decide what a direction carries.
pub trait SubscriptionDirection: sealed::Sealed + Debug + Clone + PartialEq + Eq {
    /// The subscription's `id`: optional in a request, since the broker generates one when it is absent (clause
    /// 5.8.1.4), and always present in a response.
    type Id: sealed::IdMember<SubscriptionId> + Debug + Clone + PartialEq + Eq;
    /// The members of the subscription only a broker writes; `()` in a request.
    type SubscriptionOutput: sealed::OutputBlock + Debug + Clone + PartialEq + Eq;
    /// The members of the notification parameters only a broker writes; `()` in a request.
    type NotificationOutput: sealed::OutputBlock + Debug + Clone + PartialEq + Eq;
}

impl SubscriptionDirection for Input {
    type Id = Option<SubscriptionId>;
    type NotificationOutput = ();
    type SubscriptionOutput = ();
}

impl SubscriptionDirection for Output {
    type Id = SubscriptionId;
    type NotificationOutput = NotificationOutputMembers;
    type SubscriptionOutput = SubscriptionOutputMembers;
}

/// The members of a subscription only a broker writes.
///
/// They are `status` (NGSI-LD Table 5.2.12-2), `createdAt` and `modifiedAt` (Table 5.2.2-1).
///
/// A request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SubscriptionOutputMembers {
    /// `status`: whether the subscription is active, paused or expired.
    pub status: Option<SubscriptionStatus>,
    /// `createdAt`.
    pub created_at: Option<DateTime>,
    /// `modifiedAt`.
    pub modified_at: Option<DateTime>,
}

/// The members of the notification parameters only a broker writes.
///
/// NGSI-LD Table 5.2.14.2-1.
///
/// A request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NotificationOutputMembers {
    /// `status`: whether the last notification succeeded.
    pub status: Option<NotificationStatus>,
    /// `timesSent`: how many notifications were sent.
    pub times_sent: Option<u64>,
    /// `timesFailed`: how many notifications failed.
    pub times_failed: Option<u64>,
    /// `lastNotification`: when the last notification was sent.
    pub last_notification: Option<DateTime>,
    /// `lastSuccess`: when the last notification succeeded.
    pub last_success: Option<DateTime>,
    /// `lastFailure`: when the last notification failed.
    pub last_failure: Option<DateTime>,
}
