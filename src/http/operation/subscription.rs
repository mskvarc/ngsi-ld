//! The subscription operations (NGSI-LD clauses 5.8.1 to 5.8.5 and 5.11.2 to 5.11.6), bound to `subscriptions`
//! and `subscriptions/{subscriptionId}` (clauses 6.10 and 6.11) for context information subscriptions, and to
//! `csourceSubscriptions` and `csourceSubscriptions/{subscriptionId}` (clauses 6.12 and 6.13) for Context Source
//! Registration Subscriptions: one request type per operation, generic over the
//! [kind of subscription](crate::subscription::SubscriptionKind), which decides the resource. The tables of
//! the two resources list the same requests and the same responses.

use crate::{
    http::{BodyError, operation::stored_document::DocumentReader},
    model::Output,
    subscription::{Problem, ReadErrorKind, Subscription, SubscriptionKind},
};

pub(super) mod create;
pub(super) mod delete;
pub(super) mod query;
pub(super) mod retrieve;
pub(super) mod update;

/// The resource each kind of subscription is bound to. The trait is public so the operations can name it in their
/// bounds, inside a module no caller can reach; both kinds implement it.
pub(crate) mod resource {
    use crate::{
        http::path::SubscriptionCollection,
        subscription::{EntitySubscription, RegistrationSubscription, SubscriptionKind},
    };

    /// A kind of subscription and the collection of Table 6.2-1 that holds it.
    pub trait SubscriptionResource: SubscriptionKind {
        /// The collection.
        const COLLECTION: SubscriptionCollection;
    }

    impl SubscriptionResource for EntitySubscription {
        const COLLECTION: SubscriptionCollection = SubscriptionCollection::Subscriptions;
    }

    impl SubscriptionResource for RegistrationSubscription {
        const COLLECTION: SubscriptionCollection = SubscriptionCollection::CsourceSubscriptions;
    }
}

/// Reads subscriptions of kind `K` strictly: a member Table 5.2.12-1 does not define, or a member of the other kind,
/// fails the response.
fn reader<K: SubscriptionKind>() -> DocumentReader<Subscription<Output, K>, ReadErrorKind<Problem>> {
    DocumentReader {
        read: Subscription::from_json,
        error: BodyError::Subscription,
    }
}
