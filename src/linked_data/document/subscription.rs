//! Subscriptions and their notifications as linked documents (NGSI-LD clauses 5.2.12, 5.2.14 and 5.3.1).

use crate::{
    linked_data::document::{
        printed,
        sealed::Sealed,
        shape::{ARRAY, Shape},
    },
    model::{Completeness, DocumentError, Entity, FeatureCollection, Lossless, Output, SimplifiedEntity},
    subscription::{Notification, Problem, ReadErrorKind, Subscription, SubscriptionDirection, SubscriptionKind},
};
use jstrict::Value as Json;
use std::convert::Infallible;

/// The array members of `NotificationParams` (Table 5.2.14.1-1) and of its `endpoint` (Table 5.2.15-1).
const NOTIFICATION_PARAMS: Shape = Shape::Object(&[
    ("attributes", ARRAY),
    ("pick", ARRAY),
    ("omit", ARRAY),
    ("endpoint", Shape::Object(&[("receiverInfo", ARRAY), ("notifierInfo", ARRAY)])),
]);

/// The array members of a subscription (Table 5.2.12-1).
const SUBSCRIPTION: Shape = Shape::Object(&[
    ("entities", ARRAY),
    ("watchedAttributes", ARRAY),
    ("notificationTrigger", ARRAY),
    ("datasetId", ARRAY),
    ("notification", NOTIFICATION_PARAMS),
]);

/// The `data` of a notification of entities, an array (Table 5.3.1-1).
const ENTITY_NOTIFICATION: Shape = Shape::Object(&[("data", ARRAY)]);

impl<D: SubscriptionDirection, K: SubscriptionKind> Sealed for Subscription<D, K> {
    type ReadError = DocumentError<ReadErrorKind<Problem>>;

    const SHAPE: Shape = SUBSCRIPTION;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

impl<C: Completeness> Sealed for Notification<Vec<Entity<Output, C>>> {
    type ReadError = DocumentError<ReadErrorKind<Infallible>>;

    const SHAPE: Shape = ENTITY_NOTIFICATION;

    fn write(&self) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

impl<C: Completeness> Sealed for Notification<Vec<SimplifiedEntity<C>>> {
    type ReadError = DocumentError<ReadErrorKind<Infallible>>;

    const SHAPE: Shape = ENTITY_NOTIFICATION;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}

/// `features` is a `@set` in the core context, so compaction keeps its array.
impl Sealed for Notification<FeatureCollection<Entity<Output>>> {
    type ReadError = DocumentError<ReadErrorKind<Infallible>>;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

/// `features` is a `@set` in the core context, so compaction keeps its array.
impl Sealed for Notification<FeatureCollection<SimplifiedEntity>> {
    type ReadError = DocumentError<ReadErrorKind<Infallible>>;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}
