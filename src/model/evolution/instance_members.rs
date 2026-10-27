//! What an instance of a temporal evolution a client sends carries besides its attribute (NGSI-LD clauses 4.5.7 and
//! 4.5.8): the `instanceId` it names and, for a recorded deletion, the `deletedAt` that makes the NGSI-LD Null it holds
//! the record of a deleted instance.

use crate::model::{DateTime, InstanceId};

/// The members an instance of a temporal evolution a client sends carries besides its attribute.
///
/// They are the `instanceId` the instance names (clauses 4.5.7 and 4.5.8) and the `deletedAt` of a recorded deletion:
/// the representation of a deleted Property or Relationship is an instance whose value is the NGSI-LD Null with the
/// `deletedAt` Temporal Property set (clauses 4.5.7 and 4.5.8). Neither is system generated in a temporal
/// representation (clause 5.2.5, Table 5.2.5-2), so Create or Update Temporal Evolution and Add Attributes take them.
///
/// A response holds both among the output-only members of the attribute, so `()` stands for them there (see
/// [`EntityDirection::TemporalMembers`](crate::model::EntityDirection::TemporalMembers)).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TemporalInstanceMembers {
    /// `instanceId`.
    instance_id: Option<InstanceId>,
    /// `deletedAt`, set only on an instance that records a deletion.
    deleted_at: Option<DateTime>,
}

impl TemporalInstanceMembers {
    /// The members of an instance that names `instance_id` or not, and records a deletion or not.
    pub(crate) const fn new(instance_id: Option<InstanceId>, deleted_at: Option<DateTime>) -> Self {
        Self { instance_id, deleted_at }
    }

    /// Returns the `instanceId` the instance names; `None` when the broker chooses it (clause 4.5.7).
    #[must_use]
    pub const fn instance_id(&self) -> Option<&InstanceId> {
        self.instance_id.as_ref()
    }

    /// Returns when the instance records the deletion at, or `None` when it records a value.
    #[must_use]
    pub const fn deleted_at(&self) -> Option<&DateTime> {
        self.deleted_at.as_ref()
    }
}
