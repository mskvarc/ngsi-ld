//! The rules of NGSI-LD Tables 5.2.12-1 and 5.2.33-1 that relate the members of a subscription, checked by the
//! builder and by both readers.

use crate::{
    query::{Locality, SelectorType},
    subscription::{Problem, Subscription, SubscriptionDirection, SubscriptionKind},
};

/// Every rule `subscription` breaks, its notification's included.
pub(crate) fn subscription_problems<D: SubscriptionDirection, K: SubscriptionKind>(subscription: &Subscription<D, K>) -> Vec<Problem> {
    let mut problems = Vec::new();
    let local = subscription.locality == Locality::Local;
    let watches = !subscription.watched_attributes.is_empty();
    let selects = !subscription.entities.is_empty();
    let periodic = subscription.time_interval.is_some();
    if !selects && !watches && !local {
        problems.push(Problem::NoSubscribedEntities);
    }
    if periodic && !selects && !local {
        problems.push(Problem::TimeIntervalWithoutEntities);
    }
    if periodic && watches {
        problems.push(Problem::WatchedAttributesWithTimeInterval);
    }
    if periodic && subscription.throttling.is_some() {
        problems.push(Problem::ThrottlingWithTimeInterval);
    }
    if subscription.locality == Locality::Distributed && subscription.entities.iter().any(|selector| *selector.selector_type() == SelectorType::All) {
        problems.push(Problem::AllTypesNotLocal);
    }
    problems.extend(subscription.notification.rule_problems());
    problems
}
