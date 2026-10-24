//! The operations a context source accepts (NGSI-LD clause 4.20): the names of Table 4.20-1, the groups of Table
//! 4.20-2, and an entry of a registration's `operations`, which is either.

use std::{fmt, str::FromStr};

/// An operation a context source may accept.
///
/// It is named as NGSI-LD Table 4.20-1 names it.
///
/// The enum is `#[non_exhaustive]`: each version of the specification adds operations to the table.
///
/// ```
/// use ngsi_ld::registration::DistributedOperation;
///
/// assert_eq!(
///     DistributedOperation::UpdateAttrInstanceTemporal.to_string(),
///     "updateAttrInstanceTemporal"
/// );
/// assert_eq!(
///     "queryBatch".parse::<DistributedOperation>()?,
///     DistributedOperation::QueryBatch
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum DistributedOperation {
    /// Create Entity (clause 5.6.1).
    CreateEntity,
    /// Update Attributes (clause 5.6.2).
    UpdateEntity,
    /// Append Attributes (clause 5.6.3).
    AppendAttrs,
    /// Partial Attribute Update (clause 5.6.4).
    UpdateAttrs,
    /// Delete Attribute (clause 5.6.5).
    DeleteAttrs,
    /// Delete Entity (clause 5.6.6).
    DeleteEntity,
    /// Batch Entity Creation (clause 5.6.7).
    CreateBatch,
    /// Batch Entity Upsert (clause 5.6.8).
    UpsertBatch,
    /// Batch Entity Update (clause 5.6.9).
    UpdateBatch,
    /// Batch Entity Delete (clause 5.6.10).
    DeleteBatch,
    /// Create or Update Temporal Evolution of an Entity (clause 5.6.11).
    UpsertTemporal,
    /// Add Attributes to Temporal Evolution of an Entity (clause 5.6.12).
    AppendAttrsTemporal,
    /// Delete Attributes from Temporal Evolution of an Entity (clause 5.6.13).
    DeleteAttrsTemporal,
    /// Partial Update Attribute Instance in Temporal Evolution of an Entity (clause 5.6.14).
    UpdateAttrInstanceTemporal,
    /// Delete Attribute Instance from Temporal Evolution of an Entity (clause 5.6.15).
    DeleteAttrInstanceTemporal,
    /// Delete Temporal Evolution of an Entity (clause 5.6.16).
    DeleteTemporal,
    /// Merge Entity (clause 5.6.17).
    MergeEntity,
    /// Replace Entity (clause 5.6.18).
    ReplaceEntity,
    /// Replace Attribute (clause 5.6.19).
    ReplaceAttrs,
    /// Batch Entity Merge (clause 5.6.20).
    MergeBatch,
    /// Purge Entities (clause 5.6.21).
    PurgeEntity,
    /// Retrieve Entity (clause 5.7.1).
    RetrieveEntity,
    /// Query Entities, excluding batch entity queries (clause 5.7.2).
    QueryEntity,
    /// Query Entities, batch entity queries only (clause 5.7.2).
    QueryBatch,
    /// Retrieve Temporal Evolution of an Entity (clause 5.7.3).
    RetrieveTemporal,
    /// Query Temporal Evolution of Entities (clause 5.7.4).
    QueryTemporal,
    /// Retrieve Available Entity Types (clause 5.7.5).
    RetrieveEntityTypes,
    /// Retrieve Details of Available Entity Types (clause 5.7.6).
    RetrieveEntityTypeDetails,
    /// Retrieve Available Entity Type Information (clause 5.7.7).
    RetrieveEntityTypeInfo,
    /// Retrieve Available Attributes (clause 5.7.8).
    RetrieveAttrTypes,
    /// Retrieve Details of Available Attributes (clause 5.7.9).
    RetrieveAttrTypeDetails,
    /// Retrieve Available Attribute Information (clause 5.7.10).
    RetrieveAttrTypeInfo,
    /// Create Subscription (clause 5.8.1).
    CreateSubscription,
    /// Update Subscription (clause 5.8.2).
    UpdateSubscription,
    /// Retrieve Subscription (clause 5.8.3).
    RetrieveSubscription,
    /// Query Subscriptions (clause 5.8.4).
    QuerySubscription,
    /// Delete Subscription (clause 5.8.5).
    DeleteSubscription,
    /// Retrieve Entity Map (clause 5.14.1).
    RetrieveEntityMap,
    /// Update Entity Map (clause 5.14.2).
    UpdateEntityMap,
    /// Delete Entity Map (clause 5.14.3).
    DeleteEntityMap,
    /// Create Entity Map for Query Entities (clause 5.14.4).
    CreateEntityMapQueryEntity,
    /// Create Entity Map for Query Temporal Evolution of Entities (clause 5.14.5).
    CreateEntityMapQueryTemporal,
    /// Retrieve Context Source Identity Information (clause 5.15.1).
    RetrieveContextSourceIdentity,
}

/// A named group of operations.
///
/// NGSI-LD Table 4.20-2.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification may name more groups.
///
/// ```
/// use ngsi_ld::registration::{DistributedOperation, OperationGroup};
///
/// assert_eq!(OperationGroup::RetrieveOps.to_string(), "retrieveOps");
/// assert_eq!(
///     OperationGroup::RetrieveOps.members(),
///     [
///         DistributedOperation::RetrieveEntity,
///         DistributedOperation::QueryEntity
///     ]
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum OperationGroup {
    /// `federationOps`: the operations a federated broker needs; the default set of a registration that names
    /// none (clause 4.20).
    FederationOps,
    /// `associationOps`: the consumption and subscription operations without entity maps.
    AssociationOps,
    /// `updateOps`: the operations that change attributes or replace an entity.
    UpdateOps,
    /// `retrieveOps`: Retrieve Entity and Query Entities.
    RetrieveOps,
    /// `redirectionOps`: the operations a broker forwards to a redirect registration.
    RedirectionOps,
}

impl OperationGroup {
    /// Returns the operations the group stands for, as Table 4.20-2 lists them.
    #[must_use]
    pub const fn members(self) -> &'static [DistributedOperation] {
        match self {
            Self::FederationOps => &[
                DistributedOperation::RetrieveEntity,
                DistributedOperation::QueryEntity,
                DistributedOperation::QueryBatch,
                DistributedOperation::RetrieveEntityTypes,
                DistributedOperation::RetrieveEntityTypeDetails,
                DistributedOperation::RetrieveEntityTypeInfo,
                DistributedOperation::RetrieveAttrTypes,
                DistributedOperation::RetrieveAttrTypeDetails,
                DistributedOperation::RetrieveAttrTypeInfo,
                DistributedOperation::CreateSubscription,
                DistributedOperation::UpdateSubscription,
                DistributedOperation::RetrieveSubscription,
                DistributedOperation::QuerySubscription,
                DistributedOperation::DeleteSubscription,
                DistributedOperation::RetrieveEntityMap,
                DistributedOperation::UpdateEntityMap,
                DistributedOperation::DeleteEntityMap,
                DistributedOperation::CreateEntityMapQueryEntity,
                DistributedOperation::RetrieveContextSourceIdentity,
            ],
            Self::AssociationOps => &[
                DistributedOperation::RetrieveEntity,
                DistributedOperation::QueryEntity,
                DistributedOperation::QueryBatch,
                DistributedOperation::RetrieveEntityTypes,
                DistributedOperation::RetrieveEntityTypeDetails,
                DistributedOperation::RetrieveEntityTypeInfo,
                DistributedOperation::RetrieveAttrTypes,
                DistributedOperation::RetrieveAttrTypeDetails,
                DistributedOperation::RetrieveAttrTypeInfo,
                DistributedOperation::CreateSubscription,
                DistributedOperation::UpdateSubscription,
                DistributedOperation::RetrieveSubscription,
                DistributedOperation::QuerySubscription,
                DistributedOperation::DeleteSubscription,
                DistributedOperation::RetrieveContextSourceIdentity,
            ],
            Self::UpdateOps => &[
                DistributedOperation::UpdateEntity,
                DistributedOperation::UpdateAttrs,
                DistributedOperation::ReplaceEntity,
                DistributedOperation::ReplaceAttrs,
            ],
            Self::RetrieveOps => &[DistributedOperation::RetrieveEntity, DistributedOperation::QueryEntity],
            Self::RedirectionOps => &[
                DistributedOperation::CreateEntity,
                DistributedOperation::UpdateEntity,
                DistributedOperation::AppendAttrs,
                DistributedOperation::UpdateAttrs,
                DistributedOperation::DeleteAttrs,
                DistributedOperation::DeleteEntity,
                DistributedOperation::MergeEntity,
                DistributedOperation::ReplaceEntity,
                DistributedOperation::ReplaceAttrs,
                DistributedOperation::RetrieveEntity,
                DistributedOperation::QueryEntity,
                DistributedOperation::PurgeEntity,
                DistributedOperation::RetrieveEntityTypes,
                DistributedOperation::RetrieveEntityTypeDetails,
                DistributedOperation::RetrieveEntityTypeInfo,
                DistributedOperation::RetrieveAttrTypes,
                DistributedOperation::RetrieveAttrTypeDetails,
                DistributedOperation::RetrieveAttrTypeInfo,
                DistributedOperation::RetrieveEntityMap,
                DistributedOperation::UpdateEntityMap,
                DistributedOperation::DeleteEntityMap,
                DistributedOperation::CreateEntityMapQueryEntity,
                DistributedOperation::RetrieveContextSourceIdentity,
            ],
        }
    }
}

/// An entry of a registration's `operations`: an operation or a group of them.
///
/// It is an operation of NGSI-LD Table 4.20-1 or a group of Table 4.20-2, whose names never collide (Table 5.2.9-1).
///
/// ```
/// use ngsi_ld::registration::{DistributedOperation, OperationGroup, SupportedOperation};
///
/// assert_eq!(
///     "retrieveOps".parse::<SupportedOperation>()?,
///     SupportedOperation::from(OperationGroup::RetrieveOps)
/// );
/// assert_eq!(
///     SupportedOperation::from(DistributedOperation::QueryEntity).to_string(),
///     "queryEntity"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// The enum is exhaustive: an entry is an operation or a group, and a later version of the specification adds
/// variants to [`DistributedOperation`] and [`OperationGroup`], which are `#[non_exhaustive]`, instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::From, derive_more::IsVariant)]
pub enum SupportedOperation {
    /// One operation.
    Operation(DistributedOperation),
    /// A named group of operations.
    Group(OperationGroup),
}

// Each variant prints its own name, which a derive cannot express for an enum of two enums.
impl fmt::Display for SupportedOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operation(operation) => operation.fmt(f),
            Self::Group(group) => group.fmt(f),
        }
    }
}

// A name is tried as a group, then as an operation, which a derive cannot express.
impl FromStr for SupportedOperation {
    type Err = strum::ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self::Group).or_else(|_| text.parse().map(Self::Operation))
    }
}
