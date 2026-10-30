//! The attribute fragment of Partial Attribute Update (NGSI-LD clauses 5.4, 5.5.8 and 6.7.3.1).

use crate::model::{
    Attribute,
    AttributeKind,
    AttributeMeta,
    AttributeNameDraft,
    BuildError,
    BuildProblem,
    DatasetId,
    DateTime,
    Deletion,
    Input,
    MemberChange,
    MemberOfOtherKind,
    OneOrMany,
    OneOrManyDraft,
    Patch,
    PatchMember,
    ProblemKind,
    Proof,
    ReservedMember,
    VocabTerm,
    attribute::pending::PendingAttributes,
};
use bon::bon;
use cefact_units::UnitCode;

/// The changes Partial Attribute Update makes to one attribute instance.
///
/// NGSI-LD clauses 5.4, 5.5.8 and 6.7.3.1. The body of the operation is an Attribute Fragment: the members of the
/// attribute to change, each of which the fragment may leave out. A fragment that sets the payload of its kind
/// (`value`, `object`, `languageMap`, ...) is an [`Attribute<Patch>`]; one that names none, such as
/// `{"observedAt": "2022-03-14T12:51:02Z"}`, `{"accuracy": {"value": 3}}` or `{}`, changes only the other members and is
/// [`AttributeMembers`].
///
/// The enum is exhaustive: a fragment sets a payload or it does not.
///
/// ```
/// use ngsi_ld::model::{AttributeFragment, AttributeMembers, Lossless};
///
/// let observed = AttributeMembers::builder()
///     .observed_at("2022-03-14T12:51:02Z".parse()?)
///     .build()?;
/// assert_eq!(
///     AttributeFragment::from(observed)
///         .render(Lossless::Concise)
///         .to_string(),
///     r#"{"observedAt":"2022-03-14T12:51:02Z"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum AttributeFragment {
    /// A fragment that sets the payload of its kind.
    WithPayload(Attribute<Patch>),
    /// A fragment that sets no payload.
    WithoutPayload(AttributeMembers),
}

/// An Attribute Fragment that sets no payload: the members of the attribute it changes, besides its value.
///
/// NGSI-LD clauses 5.4 and 5.5.8. Any member the fragment names replaces the target's, the members it leaves out are
/// kept, and a member whose value is the NGSI-LD Null is deleted. The fragment may name its `type` (clause 5.6.4.4),
/// which has to be the one of the target; without it, the kind is the target's.
///
/// A member only some kinds have (`unitCode`, `valueType`, `objectType`) is checked against the kind when the fragment
/// names it. Without a kind the fragment leaves the check to the target.
///
/// ```
/// use ngsi_ld::model::{AttributeKind, AttributeMembers, Lossless};
///
/// let fragment = AttributeMembers::builder()
///     .kind(AttributeKind::Property)
///     .observed_at("2022-03-14T12:51:02Z".parse()?)
///     .property("accuracy", 3)
///     .build()?;
/// assert_eq!(fragment.kind(), Some(AttributeKind::Property));
/// assert!(
///     AttributeMembers::builder()
///         .kind(AttributeKind::Relationship)
///         .unit_code(cefact_units::UnitCode::Cel)
///         .build()
///         .is_err()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeMembers {
    /// `type`, when the fragment names it.
    pub(crate) kind: Option<AttributeKind>,
    /// The change to `unitCode`.
    pub(crate) unit_code: Option<MemberChange<UnitCode>>,
    /// The change to `valueType`.
    pub(crate) value_type: Option<MemberChange<VocabTerm>>,
    /// The change to `objectType`.
    pub(crate) object_type: Option<MemberChange<OneOrMany<VocabTerm>>>,
    /// The members every kind shares, and the sub-attributes.
    pub(crate) meta: AttributeMeta<Patch>,
}

#[bon]
impl AttributeMembers {
    /// Starts a builder of a fragment that sets no payload. The optional members have setters; sub-attributes are
    /// added with the per-kind methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, an empty
    /// `objectType`, and a member the given kind has not.
    #[builder]
    pub fn new(
        #[builder(field)] attributes: PendingAttributes,
        kind: Option<AttributeKind>,
        unit_code: Option<UnitCode>,
        value_type: Option<VocabTerm>,
        #[builder(into)] object_type: Option<OneOrManyDraft<VocabTerm>>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let object_type = OneOrManyDraft::resolve_optional(object_type, ReservedMember::ObjectType, &mut problems);
        if let Some(kind) = kind {
            let named = [
                (PatchMember::UnitCode, unit_code.is_some()),
                (PatchMember::ValueType, value_type.is_some()),
                (PatchMember::ObjectType, matches!(object_type, Ok(Some(_)))),
            ];
            for (member, _) in named.into_iter().filter(|&(_, present)| present) {
                if let Err(MemberOfOtherKind { member, kind }) = member.check(kind) {
                    problems.push(BuildProblem::top(ProblemKind::MemberOfOtherKind { member, kind }));
                }
            }
        }
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems).into_patch();
        // An unresolved member has recorded its problem, so the error is never empty.
        if let Ok(object_type) = object_type {
            BuildError::finish(
                problems,
                Self {
                    kind,
                    unit_code: unit_code.map(MemberChange::Set),
                    value_type: value_type.map(MemberChange::Set),
                    object_type: object_type.map(MemberChange::Set),
                    meta,
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }
}

impl AttributeMembers {
    /// Returns the kind the fragment names (`type`), if it does.
    #[must_use]
    pub const fn kind(&self) -> Option<AttributeKind> {
        self.kind
    }

    /// Returns the change to the unit of the value (`unitCode`): set, deleted with the NGSI-LD Null, or absent.
    #[must_use]
    pub fn unit_code(&self) -> Option<MemberChange<&UnitCode>> {
        self.unit_code.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the datatype of the value (`valueType`).
    #[must_use]
    pub fn value_type(&self) -> Option<MemberChange<&VocabTerm>> {
        self.value_type.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the types of the targets (`objectType`).
    #[must_use]
    pub fn object_type(&self) -> Option<MemberChange<&OneOrMany<VocabTerm>>> {
        self.object_type.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the members every kind shares, and the sub-attributes.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<Patch> {
        &self.meta
    }

    /// The same fragment, deleting `member` of the target attribute: the member is written as the NGSI-LD Null
    /// (clause 5.5.8), replacing any value the fragment set for it.
    ///
    /// ```
    /// use ngsi_ld::model::{AttributeFragment, AttributeMembers, Lossless, PatchMember};
    ///
    /// let fragment = AttributeMembers::builder()
    ///     .build()?
    ///     .delete_member(PatchMember::ObservedAt)?;
    /// assert_eq!(
    ///     AttributeFragment::from(fragment)
    ///         .render(Lossless::Concise)
    ///         .to_string(),
    ///     r#"{"observedAt":"urn:ngsi-ld:null"}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`MemberOfOtherKind`] when the fragment names a kind that has no such member.
    pub fn delete_member(mut self, member: PatchMember) -> Result<Self, MemberOfOtherKind> {
        if let Some(kind) = self.kind {
            member.check(kind)?;
        }
        match member {
            PatchMember::ObservedAt => self.meta.observed_at = Some(MemberChange::Delete),
            PatchMember::ExpiresAt => self.meta.expires_at = Some(MemberChange::Delete),
            PatchMember::UnitCode => self.unit_code = Some(MemberChange::Delete),
            PatchMember::ValueType => self.value_type = Some(MemberChange::Delete),
            PatchMember::ObjectType => self.object_type = Some(MemberChange::Delete),
        }
        Ok(self)
    }

    /// The same fragment, deleting the sub-attribute `name` of the target attribute with `deletion`, as
    /// [`Attribute::delete_sub_attribute`] does.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] when `name` is not an attribute name, or the fragment already holds a change of the
    /// sub-attribute the deletion cannot stand beside: the bare Null among other instances, or a second change of the
    /// same instance.
    pub fn delete_sub_attribute(mut self, name: impl Into<AttributeNameDraft>, deletion: impl Into<Deletion>) -> Result<Self, BuildError> {
        let problems = self.meta.delete_attribute(name.into(), deletion.into()).into_iter().collect();
        BuildError::finish(problems, self)
    }
}

attribute_methods!(AttributeMembersBuilder, attribute_members_builder::State, attributes);

#[cfg(test)]
mod tests;
