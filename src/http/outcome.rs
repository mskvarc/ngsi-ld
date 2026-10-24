//! The success outcomes the clause 6 tables list, one enum per family of operations.

use crate::{
    http::{BatchOperationResult, ResponseError, UpdateResult, headers::OptionalHeader, path::id_from_location, response::Success},
    model::EntityId,
};
use http::StatusCode;
use iri_rs::IriRefBuf;
use serde::de::DeserializeOwned;
use std::{marker::PhantomData, str::FromStr};

/// The outcome of Create Entity (NGSI-LD clause 6.4.3.1).
///
/// Clause 6.4.3.1 requires a `Location` header on both outcomes; a broker that omits it, or sends one that is not a
/// URI reference, still created the entity, so the location is an [`OptionalHeader`] rather than the success being
/// reported as a failure, which a retry would turn into `AlreadyExists`.
///
/// The enum is exhaustive: its variants are the success statuses the operation's table in clause 6 lists.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum CreateOutcome {
    /// 201: the entity was created.
    Created {
        /// The path of the created entity, relative to the broker.
        location: OptionalHeader<IriRefBuf>,
    },
    /// 207: the entity was created locally, but a distributed part of the operation failed.
    PartiallyCreated {
        /// The path of the created entity, relative to the broker.
        location: OptionalHeader<IriRefBuf>,
        /// What succeeded and what failed.
        result: BatchOperationResult,
    },
}

/// The outcome of the operations that create a document the broker names in the `Location` of a 201: Create
/// Subscription (NGSI-LD clause 6.10.3.1), Create Context Source Registration Subscription (clause 6.12.3.1),
/// Register Context Source (clause 6.8.3.1) and Add @context (clause 6.29.3.1). `I` is the identifier of the
/// document: [`SubscriptionId`](crate::subscription::SubscriptionId),
/// [`RegistrationId`](crate::registration::RegistrationId) or
/// [`ContextId`](crate::model::ContextId).
///
/// As for [`CreateOutcome`], the location is an [`OptionalHeader`]: a broker that omits the required header, or
/// sends one that is not a URI reference, still created the document.
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::{
///     http::{ResourceCreated, headers::OptionalHeader},
///     subscription::SubscriptionId,
/// };
///
/// let location =
///     IriRefBuf::new("/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:mySubscription")?;
/// let created = ResourceCreated::<SubscriptionId>::new(OptionalHeader::Valid(location));
/// assert_eq!(
///     created.id().map(|id| id.to_string()).as_deref(),
///     Some("urn:ngsi-ld:Subscription:mySubscription")
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCreated<I> {
    /// The path of the created document, relative to the broker.
    pub location: OptionalHeader<IriRefBuf>,
    /// The identifier type.
    id: PhantomData<fn() -> I>,
}

impl<I> ResourceCreated<I> {
    /// The outcome whose `Location` is `location`.
    #[must_use]
    pub const fn new(location: OptionalHeader<IriRefBuf>) -> Self {
        Self { location, id: PhantomData }
    }

    /// Reads 201; the location never fails the outcome.
    pub(crate) fn decode(response: &Success) -> Result<Self, ResponseError> {
        match response.status() {
            StatusCode::CREATED => Ok(Self::new(response.location())),
            _ => Err(response.unexpected()),
        }
    }
}

impl<I: FromStr> ResourceCreated<I> {
    /// The id of the created document, read from the last path segment of the location, which Table 6.2-1 makes the
    /// document's id (`/subscriptions/{subscriptionId}`, `/csourceSubscriptions/{subscriptionId}`,
    /// `/csourceRegistrations/{registrationId}`, `/jsonldContexts/{contextId}`), percent-decoded; `None` when the
    /// location is absent or unreadable, or its last segment is not a URI. A document created without an `id` gets one
    /// from the broker (clauses 5.8.1.4, 5.9.2.4 and 5.11.2.4), and this is how the caller learns it.
    #[must_use]
    pub fn id(&self) -> Option<I> {
        id_from_location(self.location.valid()?.as_str())
    }
}

/// The outcome of Create or Update Temporal Evolution of an Entity (NGSI-LD clause 6.18.3.1).
///
/// The location is an [`OptionalHeader`] for the reason [`CreateOutcome`] gives: a broker that omits the required
/// header, or sends one that is not a URI reference, still created the temporal evolution.
///
/// The enum is exhaustive: its variants are the success statuses the operation's table in clause 6 lists.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum TemporalUpsertOutcome {
    /// 201: the temporal evolution was created.
    Created {
        /// The path of the created temporal evolution, relative to the broker.
        location: OptionalHeader<IriRefBuf>,
    },
    /// 204: the temporal evolution existed and was updated.
    Updated,
}

/// The outcome of an operation on whole entities: Delete, Replace and Merge Entity, Purge Entities, and Batch
/// Entity Update, Delete and Merge (NGSI-LD clauses 6.4.3.3, 6.5.3.2 to 6.5.3.4, 6.16.3.1, 6.17.3.1 and
/// 6.31.3.1).
///
/// The enum is exhaustive: its variants are the success statuses the operation's table in clause 6 lists.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum BatchOutcome {
    /// 204: everything was done.
    Done,
    /// 207: some entities, or a distributed part of the operation, failed; the result says which, entity by
    /// entity.
    Partial(BatchOperationResult),
}

/// The outcome of Batch Entity Creation (NGSI-LD clause 6.14.3.1).
///
/// The broker lists ids in any order (Table 6.14.3.1-1), so they are kept as received, never matched to positions
/// in the request.
///
/// The enum is exhaustive: its variants are the success statuses the operation's table in clause 6 lists.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum BatchCreateOutcome {
    /// 201: every entity was created; the ids of the created entities.
    Created(Vec<EntityId>),
    /// 207: only some or none of the entities were created; the result says which, entity by entity.
    Partial(BatchOperationResult),
}

/// The outcome of Batch Entity Upsert (NGSI-LD clause 6.15.3.1).
///
/// The enum is exhaustive: its variants are the success statuses the operation's table in clause 6 lists.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum UpsertOutcome {
    /// 201: the ids of the entities that were created, in the order received; every other entity existed and was
    /// updated.
    Created(Vec<EntityId>),
    /// 204: every entity existed and was updated.
    Updated,
    /// 207: only some or none of the entities were created or updated; the result says which, entity by entity.
    Partial(BatchOperationResult),
}

/// The outcome of an operation on attributes: Update and Append Attributes, Partial Attribute Update, Replace
/// and Delete Attribute (NGSI-LD clauses 6.6.3 and 6.7.3).
///
/// The enum is exhaustive: its variants are the success statuses the operation's table in clause 6 lists.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum UpdateOutcome {
    /// 204: every attribute was changed.
    Done,
    /// 207: only the attributes the result lists as updated were changed.
    Partial(UpdateResult),
}

impl TemporalUpsertOutcome {
    /// Reads 201 or 204; the location never fails the outcome.
    pub(crate) fn decode(response: &Success) -> Result<Self, ResponseError> {
        match response.status() {
            StatusCode::CREATED => Ok(Self::Created { location: response.location() }),
            StatusCode::NO_CONTENT => Ok(Self::Updated),
            _ => Err(response.unexpected()),
        }
    }
}

impl BatchOutcome {
    /// Reads 204 or 207.
    pub(crate) fn decode(response: &Success) -> Result<Self, ResponseError> {
        Ok(done_or_partial(response)?.map_or(Self::Done, Self::Partial))
    }
}

impl BatchCreateOutcome {
    /// Reads 201 or 207.
    pub(crate) fn decode(response: &Success) -> Result<Self, ResponseError> {
        match response.status() {
            StatusCode::CREATED => response.deserialize().map(Self::Created),
            StatusCode::MULTI_STATUS => response.deserialize().map(Self::Partial),
            _ => Err(response.unexpected()),
        }
    }
}

impl UpsertOutcome {
    /// Reads 201, 204 or 207.
    pub(crate) fn decode(response: &Success) -> Result<Self, ResponseError> {
        match response.status() {
            StatusCode::CREATED => response.deserialize().map(Self::Created),
            StatusCode::NO_CONTENT => Ok(Self::Updated),
            StatusCode::MULTI_STATUS => response.deserialize().map(Self::Partial),
            _ => Err(response.unexpected()),
        }
    }
}

impl UpdateOutcome {
    /// Reads 204 or 207.
    pub(crate) fn decode(response: &Success) -> Result<Self, ResponseError> {
        Ok(done_or_partial(response)?.map_or(Self::Done, Self::Partial))
    }
}

/// Nothing for 204, the partial result for 207, an error for any other success status.
fn done_or_partial<T: DeserializeOwned>(response: &Success) -> Result<Option<T>, ResponseError> {
    match response.status() {
        StatusCode::NO_CONTENT => Ok(None),
        StatusCode::MULTI_STATUS => response.deserialize().map(Some),
        _ => Err(response.unexpected()),
    }
}
