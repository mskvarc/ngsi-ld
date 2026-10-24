//! What a context source can provide (NGSI-LD clauses 5.2.8 and 5.2.10): the entities, by type and optionally by
//! identifier or identifier pattern, and the names of the Properties and Relationships.

use crate::{
    model::{
        AttributeName,
        AttributeNameDraft,
        BuildError,
        EntityId,
        EntityTypeName,
        MemberProblem,
        NameError,
        OneOrMany,
        draft::resolve_required_drafts,
        name::draft::resolve_required_names,
    },
    query::IdPattern,
    registration::{Problem, RegistrationField, RegistrationInfoMember},
};
use bon::bon;

/// Entities a context source can provide (NGSI-LD clause 5.2.8): of one or more entity types and, optionally, of
/// one identifier or of the identifiers a pattern matches; `id` takes precedence over `idPattern`.
///
/// Unlike the `EntitySelector` of a query (clause 5.2.33), `type` is a list of entity type names, not a type
/// selection, and `id` is one identifier.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     model::{EntityId, EntityTypeName},
///     registration::EntityInfo,
/// };
///
/// let vehicle = EntityInfo::new("Vehicle".parse::<EntityTypeName>()?)
///     .with_id(EntityId::from(iri!("urn:ngsi-ld:Vehicle:A456")));
/// assert_eq!(
///     vehicle.id().map(EntityId::as_str),
///     Some("urn:ngsi-ld:Vehicle:A456")
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityInfo {
    /// `type`: one entity type, written as a string, or several, written as an array.
    pub(crate) entity_type: OneOrMany<EntityTypeName>,
    /// `id`.
    pub(crate) id: Option<EntityId>,
    /// `idPattern`.
    pub(crate) id_pattern: Option<IdPattern>,
}

impl EntityInfo {
    /// The entities of `entity_type`: one type, or several in a [`OneOrMany`].
    #[must_use]
    pub fn new(entity_type: impl Into<OneOrMany<EntityTypeName>>) -> EntityInfo {
        EntityInfo {
            entity_type: entity_type.into(),
            id: None,
            id_pattern: None,
        }
    }

    /// Restricts the entities to the one identified by `id`.
    #[must_use]
    pub fn with_id(self, id: EntityId) -> Self {
        Self { id: Some(id), ..self }
    }

    /// Restricts the entities to those whose identifier `pattern` matches.
    #[must_use]
    pub fn with_id_pattern(self, pattern: IdPattern) -> Self {
        Self {
            id_pattern: Some(pattern),
            ..self
        }
    }

    /// Returns the entity types (`type`).
    #[must_use]
    pub const fn entity_type(&self) -> &OneOrMany<EntityTypeName> {
        &self.entity_type
    }

    /// Returns the entity identifier (`id`).
    #[must_use]
    pub const fn id(&self) -> Option<&EntityId> {
        self.id.as_ref()
    }

    /// Returns the identifier pattern (`idPattern`).
    #[must_use]
    pub const fn id_pattern(&self) -> Option<&IdPattern> {
        self.id_pattern.as_ref()
    }
}

draft!(
    /// The entities given to a registration's builder: an [`EntityInfo`], or text naming one entity type, validated
    /// when the builder finishes, so every invalid name is reported together. A list may mix both, each converted
    /// with `.into()`; entities by identifier, by pattern or of several types are an [`EntityInfo`].
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     model::{EntityId, EntityTypeName},
    ///     registration::{EntityInfo, EntityInfoDraft},
    /// };
    ///
    /// let car: EntityTypeName = "Car".parse()?;
    /// let one_car = EntityInfo::new(car).with_id(EntityId::from(iri!("urn:ngsi-ld:Car:1")));
    /// let entities: [EntityInfoDraft; 2] = [one_car.into(), "Vehicle".into()];
    /// # let _ = entities;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    EntityInfoDraft(EntityInfo, NameError),
    parse = |text: &str| text.parse::<EntityTypeName>().map(EntityInfo::new)
);

/// One element of a registration's `information` (NGSI-LD clause 5.2.10): the entities, Properties and
/// Relationships a context source may provide information about. At least one of the three is present, and none is
/// an empty list.
///
/// ```
/// use ngsi_ld::registration::RegistrationInfo;
///
/// let information = RegistrationInfo::builder()
///     .entities(["Vehicle"])
///     .property_names(["brandName", "speed"])
///     .relationship_names(["isParked"])
///     .build()?;
/// assert_eq!(information.property_names().len(), 2);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationInfo {
    /// `entities`; empty when absent.
    pub(crate) entities: Vec<EntityInfo>,
    /// `propertyNames`; empty when absent.
    pub(crate) property_names: Vec<AttributeName>,
    /// `relationshipNames`; empty when absent.
    pub(crate) relationship_names: Vec<AttributeName>,
}

#[bon]
impl RegistrationInfo {
    /// Starts a builder; each member is optional, but at least one is needed.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid attribute name, every member given as an empty list,
    /// and `EmptyRegistrationInfo` when no member is given (Table 5.2.10-1).
    #[builder]
    pub fn new(
        #[builder(with = |entities: impl IntoIterator<Item = impl Into<EntityInfoDraft>>| entities.into_iter().map(Into::into).collect())] entities: Option<
            Vec<EntityInfoDraft>,
        >,
        #[builder(with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())] property_names: Option<
            Vec<AttributeNameDraft>,
        >,
        #[builder(with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())]
        relationship_names: Option<Vec<AttributeNameDraft>>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let field = RegistrationField::InformationElement;
        let member = field(RegistrationInfoMember::Entities);
        let entities = resolve_required_drafts(member, entities, &mut problems, |index, source| {
            Problem::from(MemberProblem::InvalidEntityTypeName { member, index, source })
        });
        let information = Self {
            entities,
            property_names: resolve_required_names(field(RegistrationInfoMember::PropertyNames), property_names, &mut problems),
            relationship_names: resolve_required_names(field(RegistrationInfoMember::RelationshipNames), relationship_names, &mut problems),
        };
        if information.is_empty() && problems.is_empty() {
            problems.push(Problem::EmptyRegistrationInfo);
        }
        BuildError::finish(problems, information)
    }
}

impl RegistrationInfo {
    /// Returns the entities (`entities`); empty when the member is absent.
    #[must_use]
    pub fn entities(&self) -> &[EntityInfo] {
        &self.entities
    }

    /// Returns the names of the Properties (`propertyNames`); empty when the member is absent.
    #[must_use]
    pub fn property_names(&self) -> &[AttributeName] {
        &self.property_names
    }

    /// Returns the names of the Relationships (`relationshipNames`); empty when the member is absent.
    #[must_use]
    pub fn relationship_names(&self) -> &[AttributeName] {
        &self.relationship_names
    }

    /// Whether the element names neither entities nor attributes, which Table 5.2.10-1 forbids.
    pub(crate) const fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.property_names.is_empty() && self.relationship_names.is_empty()
    }

    /// Whether every entity is named by its `id` and attributes are named, as an exclusive registration requires
    /// (clause 4.3.6.3).
    pub(crate) fn names_entity_ids_and_attributes(&self) -> bool {
        !self.entities.is_empty()
            && self.entities.iter().all(|entity| entity.id.is_some())
            && !(self.property_names.is_empty() && self.relationship_names.is_empty())
    }
}
