//! The members every attribute kind shares (NGSI-LD clauses 4.5.2.2 and 4.5.3.2).

use crate::{
    model::{
        AttributeOutputMembers,
        Attributes,
        BuildProblem,
        Change,
        DatasetId,
        DateTime,
        Direction,
        Input,
        Output,
        Patch,
        Proof,
        attribute::pending::PendingAttributes,
    },
    query::temporal_query::TemporalProperty,
};

/// The members every attribute kind has: the dataset id that tells instances apart (clause 4.5.5), the
/// `observedAt` and `expiresAt` timestamps (clauses 4.8 and 4.22), the digital signature (`ngsildproof`, clause
/// 4.5.2.2), the attribute's own sub-attributes, and, in a response, the output-only members every kind shares.
///
/// In a fragment ([`Patch`]) the timestamps are changes: a fragment may delete them with the NGSI-LD Null (clause
/// 5.5.8), while its `datasetId` names the instance it changes and cannot be deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttributeMeta<D: Direction> {
    /// `datasetId`; `None` for the default instance.
    pub dataset_id: Option<DatasetId>,
    /// `observedAt`; in a fragment, the change to it.
    pub observed_at: D::Member<DateTime>,
    /// `expiresAt`; in a fragment, the change to it.
    pub expires_at: D::Member<DateTime>,
    /// `ngsildproof`: the digital signature of the attribute (clause 4.5.2.2). Boxed, because the proof is itself a
    /// Property with these members.
    pub proof: Option<Box<Proof<D>>>,
    /// The sub-attributes: Properties and Relationships of this attribute.
    pub attributes: Attributes<D>,
    /// The output-only members; `()` outside responses.
    pub output: D::AttributeOutput,
}

impl<D: Direction> AttributeMeta<D> {
    /// No members set.
    pub(crate) fn empty() -> Self {
        Self {
            dataset_id: None,
            observed_at: Default::default(),
            expires_at: Default::default(),
            proof: None,
            attributes: Attributes::new(),
            output: D::AttributeOutput::default(),
        }
    }
}

impl AttributeMeta<Input> {
    /// Assembles the members a builder collected, recording every problem of the sub-attributes.
    pub(crate) fn build(
        dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
        pending: PendingAttributes,
        problems: &mut Vec<BuildProblem>,
    ) -> Self {
        Self {
            dataset_id,
            observed_at,
            expires_at,
            proof: proof.map(Box::new),
            attributes: pending.resolve(problems),
            output: (),
        }
    }

    /// Every sub-attribute becomes a [`Change::Set`].
    pub(crate) fn into_patch(self) -> AttributeMeta<Patch> {
        AttributeMeta {
            dataset_id: self.dataset_id,
            observed_at: Patch::set_member(self.observed_at),
            expires_at: Patch::set_member(self.expires_at),
            proof: self.proof.map(|proof| Box::new(proof.into_patch())),
            attributes: self.attributes.convert::<Patch>(|_, attribute| Change::Set(attribute.into_patch())),
            output: (),
        }
    }

    /// Every sub-attribute becomes the attribute a response carries, with no output-only member yet.
    pub(crate) fn into_output(self) -> AttributeMeta<Output> {
        AttributeMeta {
            dataset_id: self.dataset_id,
            observed_at: self.observed_at,
            expires_at: self.expires_at,
            proof: self.proof.map(|proof| Box::new(proof.into_output())),
            attributes: self.attributes.convert::<Output>(|_, attribute| attribute.into_output()),
            output: AttributeOutputMembers::default(),
        }
    }
}

impl AttributeMeta<Output> {
    /// The time the Temporal Property `property` records for the attribute (clause 4.11), when the attribute
    /// carries that member.
    pub(crate) const fn recorded_at(&self, property: TemporalProperty) -> Option<DateTime> {
        match property {
            TemporalProperty::ObservedAt => self.observed_at,
            TemporalProperty::CreatedAt => self.output.created_at,
            TemporalProperty::ModifiedAt => self.output.modified_at,
            TemporalProperty::DeletedAt => self.output.deleted_at,
        }
    }

    /// Drops the output-only members, at every level, recording every NGSI-LD Null a sub-attribute holds.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> AttributeMeta<Input> {
        AttributeMeta {
            dataset_id: self.dataset_id,
            observed_at: self.observed_at,
            expires_at: self.expires_at,
            proof: self.proof.map(|proof| Box::new(proof.into_input(problems))),
            attributes: self.attributes.into_input(problems),
            output: (),
        }
    }
}
