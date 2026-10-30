//! List Relationships (NGSI-LD clauses 4.5.22 and 5.2.37).

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
        build_problem::Recorded,
        null,
    },
    seal::Seal,
};
use bon::bon;
use iri_rs::Iri;

/// A `ListRelationship`: an ordered list of linked entities.
///
/// NGSI-LD clauses 4.5.22 and 5.2.37.
///
/// Build one with [`ListRelationship::builder`]; read one from a document through the representation parsers.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{EntityId, ListRelationship};
///
/// let route = ListRelationship::builder(vec![
///     EntityId::from(iri!("urn:ngsi-ld:City:Antwerp")),
///     EntityId::from(iri!("urn:ngsi-ld:City:Rotterdam")),
/// ])
/// .build()?;
/// assert_eq!(route.object_list().len(), 2);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRelationship<D: Direction> {
    /// `objectList`: the ordered target entities.
    pub(crate) object_list: Vec<EntityId>,
    /// `objectType`: the type or types of the targets (clause 4.5.23).
    pub(crate) object_type: D::Member<OneOrMany<VocabTerm>>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<ListRelationshipOutputMembers>,
}

#[bon]
impl ListRelationship<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, an empty
    /// `objectType`, and an `objectList` that is the NGSI-LD Null alone (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn)] object_list: Vec<EntityId>,
        #[builder(field)] attributes: PendingAttributes,
        #[builder(into)] object_type: Option<OneOrManyDraft<VocabTerm>>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        null::report(null::check_object_list(&object_list), &mut problems);
        let object_type = OneOrManyDraft::resolve_optional(object_type, ReservedMember::ObjectType, &mut problems);
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        // An unresolved member has recorded its problem, so the error is never empty.
        match object_type {
            Ok(object_type) => BuildError::finish(
                problems,
                Self {
                    object_list,
                    object_type,
                    meta,
                    output: (),
                },
            ),
            Err(Recorded) => Err(BuildError::new(problems)),
        }
    }
}

impl<D: Direction> ListRelationship<D> {
    /// Returns the ordered target entities (`objectList`).
    #[must_use]
    pub fn object_list(&self) -> &[EntityId] {
        &self.object_list
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> ListRelationship<D> {
    /// Returns the type or types of the targets (`objectType`).
    #[must_use]
    pub fn object_type(&self) -> Option<&OneOrMany<VocabTerm>> {
        D::member(Seal::new(), &self.object_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> ListRelationship<E> {
        ListRelationship {
            object_list: self.object_list,
            object_type: E::set_member(Seal::new(), D::into_member(Seal::new(), self.object_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl ListRelationship<Patch> {
    /// Returns the change the fragment makes to the type or types of the targets (`objectType`): set, deleted with the
    /// NGSI-LD Null, or absent.
    #[must_use]
    pub fn object_type(&self) -> Option<MemberChange<&OneOrMany<VocabTerm>>> {
        self.object_type.as_ref().map(MemberChange::as_ref)
    }
}

impl ListRelationship<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &ListRelationshipOutputMembers {
        &self.output
    }
}

/// The members a `ListRelationship` has only in a response.
///
/// NGSI-LD clauses 4.5.22.2, 4.5.23.2 and Table 5.2.37-2.
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ListRelationshipOutputMembers {
    /// `previousObjectList`: the object list before the change that triggered a notification, sent only when the
    /// subscription asks for `showChanges` (clause 4.5.22.2). It is written like `objectList`.
    pub previous_object_list: Option<Vec<EntityId>>,
    /// `entityList`: the linked entities, in the order of `objectList`, that inline linked entity retrieval adds
    /// (clause 4.5.23.2), in the representation of the response. The `pick` and `omit` of the request apply to
    /// linked entities too (clause 4.21, Table 6.5.3.1-1), and may remove their `id` and `type`, so a linked entity
    /// is read as a projected entity whatever the request asked for.
    pub entity_list: Option<Vec<Entity<Output, Projected>>>,
}

attribute_methods!(ListRelationshipBuilder, list_relationship_builder::State, attributes);

impl AttributeDraft<ListRelationship<Input>> for ListRelationship<Input> {
    fn into_attribute(self) -> Result<ListRelationship<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: list_relationship_builder::IsComplete> AttributeDraft<ListRelationship<Input>> for ListRelationshipBuilder<S> {
    fn into_attribute(self) -> Result<ListRelationship<Input>, BuildError> {
        self.build()
    }
}

/// A list of entity ids is a `ListRelationship` with nothing but that list.
impl AttributeDraft<ListRelationship<Input>> for Vec<EntityId> {
    fn into_attribute(self) -> Result<ListRelationship<Input>, BuildError> {
        ListRelationship::builder(self).build()
    }
}

/// An array of compile-time IRI literals is a `ListRelationship` with nothing but those targets.
impl<const N: usize> AttributeDraft<ListRelationship<Input>> for [Iri<&str>; N] {
    fn into_attribute(self) -> Result<ListRelationship<Input>, BuildError> {
        ListRelationship::builder(self.into_iter().map(EntityId::from).collect()).build()
    }
}
