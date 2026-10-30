//! Relationships (NGSI-LD clauses 4.5.3 and 5.2.6).

use crate::{
    model::{
        AttributeDraft,
        AttributeMeta,
        AttributeNameDraft,
        BuildError,
        DatasetId,
        DateTime,
        Direction,
        Entity,
        EntityId,
        Input,
        MemberChange,
        OneOrMany,
        OneOrManyDraft,
        Output,
        Patch,
        Plain,
        Projected,
        Proof,
        ReservedMember,
        VocabTerm,
        attribute::pending::PendingAttributes,
        null,
    },
    seal::Seal,
};
use bon::bon;
use iri_rs::Iri;

/// A Relationship: a link to one or more entities.
///
/// NGSI-LD clauses 4.5.3 and 5.2.6.
///
/// Build one with [`Relationship::builder`] from an entity id or a `Vec` of them; read one from a document through
/// the representation parsers.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{EntityId, Relationship};
///
/// let owner = Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Person:Bob"))).build()?;
/// assert_eq!(owner.object().iter().count(), 1);
/// assert!(
///     Relationship::builder(Vec::<EntityId>::new())
///         .build()
///         .is_err()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship<D: Direction> {
    /// `object`: the target entity or entities.
    pub(crate) object: OneOrMany<EntityId>,
    /// `objectType`: the type or types of the targets (clause 4.5.23).
    pub(crate) object_type: D::Member<OneOrMany<VocabTerm>>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<RelationshipOutputMembers>,
}

#[bon]
impl Relationship<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, an empty
    /// `object` or `objectType`, and an `object` that is the NGSI-LD Null (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] object: OneOrManyDraft<EntityId>,
        #[builder(field)] attributes: PendingAttributes,
        #[builder(into)] object_type: Option<OneOrManyDraft<VocabTerm>>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let object = object.resolve(ReservedMember::Object, &mut problems);
        if let Some(object) = &object {
            null::report(null::check_object(object), &mut problems);
        }
        let object_type = OneOrManyDraft::resolve_optional(object_type, ReservedMember::ObjectType, &mut problems);
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        // An unresolved member has recorded its problem, so the error is never empty.
        if let (Some(object), Ok(object_type)) = (object, object_type) {
            BuildError::finish(
                problems,
                Self {
                    object,
                    object_type,
                    meta,
                    output: (),
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }
}

impl<D: Direction> Relationship<D> {
    /// Returns the target entity or entities (`object`).
    #[must_use]
    pub const fn object(&self) -> &OneOrMany<EntityId> {
        &self.object
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> Relationship<D> {
    /// Returns the type or types of the targets (`objectType`).
    #[must_use]
    pub fn object_type(&self) -> Option<&OneOrMany<VocabTerm>> {
        D::member(Seal::new(), &self.object_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> Relationship<E> {
        Relationship {
            object: self.object,
            object_type: E::set_member(Seal::new(), D::into_member(Seal::new(), self.object_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl Relationship<Patch> {
    /// Returns the change the fragment makes to the type or types of the targets (`objectType`): set, deleted with the
    /// NGSI-LD Null, or absent.
    #[must_use]
    pub fn object_type(&self) -> Option<MemberChange<&OneOrMany<VocabTerm>>> {
        self.object_type.as_ref().map(MemberChange::as_ref)
    }
}

impl Relationship<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &RelationshipOutputMembers {
        &self.output
    }
}

/// The members a Relationship has only in a response.
///
/// NGSI-LD clauses 4.5.3.2, 4.5.23.2 and Table 5.2.6-2.
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RelationshipOutputMembers {
    /// `previousObject`: the object before the change that triggered a notification, sent only when the
    /// subscription asks for `showChanges` (clause 4.5.3.2). It is written like `object`.
    pub previous_object: Option<OneOrMany<EntityId>>,
    /// `entity`: the linked entity, or entities for a Relationship with several objects, that inline linked
    /// entity retrieval adds (clause 4.5.23.2), in the representation of the response. The `pick` and `omit` of the
    /// request apply to linked entities too (clause 4.21, Table 6.5.3.1-1), and may remove their `id` and `type`, so
    /// a linked entity is read as a projected entity whatever the request asked for.
    pub entity: Option<OneOrMany<Entity<Output, Projected>>>,
}

attribute_methods!(RelationshipBuilder, relationship_builder::State, attributes);

impl AttributeDraft<Relationship<Input>> for Relationship<Input> {
    fn into_attribute(self) -> Result<Relationship<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: relationship_builder::IsComplete> AttributeDraft<Relationship<Input>> for RelationshipBuilder<S> {
    fn into_attribute(self) -> Result<Relationship<Input>, BuildError> {
        self.build()
    }
}

/// An entity id is a Relationship with nothing but that object.
impl AttributeDraft<Relationship<Input>> for EntityId {
    fn into_attribute(self) -> Result<Relationship<Input>, BuildError> {
        Relationship::builder(self).build()
    }
}

/// A compile-time IRI literal is a Relationship with nothing but that object.
impl AttributeDraft<Relationship<Input>> for Iri<&str> {
    fn into_attribute(self) -> Result<Relationship<Input>, BuildError> {
        Relationship::builder(EntityId::from(self)).build()
    }
}

/// Several entity ids are a Relationship with an array of objects.
impl AttributeDraft<Relationship<Input>> for Vec<EntityId> {
    fn into_attribute(self) -> Result<Relationship<Input>, BuildError> {
        Relationship::builder(self).build()
    }
}
