//! Unit tests of subscriptions: building, reading, fragments and the two kinds of subscription.

use crate::{
    model::{BuildError, DocumentError, Input},
    query::EntitySelector,
    subscription::{
        Endpoint,
        NotificationParams,
        Problem as SubscriptionProblem,
        Problem,
        ReadErrorKind,
        Subscription,
        SubscriptionDirection,
        read_error::tests::broken_rules,
    },
};
use std::fmt::Debug;

mod building;
mod fragments;
mod kinds;
mod reading;
mod verbatim;

/// The endpoint of Annex C.4.
fn endpoint() -> Endpoint {
    Endpoint::builder("http://my.endpoint.org/notify").build().unwrap()
}

/// Notification parameters with nothing but the endpoint of Annex C.4.
fn notification() -> NotificationParams<Input> {
    NotificationParams::builder(endpoint()).build().unwrap()
}

/// The selector of the entities of type `Vehicle`.
fn vehicles() -> EntitySelector {
    EntitySelector::new("Vehicle".parse().unwrap())
}

/// The problems of a build that must fail.
fn problems<T: Debug>(result: Result<T, BuildError<Problem>>) -> Vec<Problem> {
    result.unwrap_err().problems().to_vec()
}

/// Reads a subscription in direction `D` and checks that it is written back as the same text.
fn round_trip<D: SubscriptionDirection>(text: &str) -> Subscription<D> {
    let subscription = Subscription::<D>::parse(text).unwrap();
    pretty_assertions::assert_eq!(subscription.render().to_string(), text);
    subscription
}

/// The error of a read that must fail.
fn read_error<D: SubscriptionDirection>(text: &str) -> DocumentError<ReadErrorKind<SubscriptionProblem>> {
    Subscription::<D>::parse(text).unwrap_err()
}

/// The problems of a read refused for breaking the rules.
fn read_problems<D: SubscriptionDirection>(text: &str) -> Vec<Problem> {
    broken_rules(read_error::<D>(text))
}
