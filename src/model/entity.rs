//! Entities (NGSI-LD clauses 4.5.1 and 5.2.4).

use crate::{
    model::{
        Attribute,
        AttributeName,
        AttributeNameDraft,
        Attributes,
        BuildError,
        Complete,
        Completeness,
        Context,
        DateTime,
        Direction,
        EntityId,
        EntityTypeName,
        IdDraft,
        Input,
        InstanceId,
        Instances,
        OneOrMany,
        Output,
        Projected,
        Scope,
        ScopeDraft,
        TypeDraft,
        attribute::pending::PendingAttributes,
        build_problem::Recorded,
    },
    seal::Seal,
};
use bon::bon;
use std::fmt::Debug;

pub(super) mod completeness;
pub(super) mod draft;
pub(crate) mod member;
#[cfg(feature = "linked-data")]
mod names;
pub(super) mod output;

/// The private supertrait that keeps `EntityDirection` implemented by this crate alone.
mod sealed {
    /// Seals [`EntityDirection`](super::EntityDirection).
    pub trait Sealed {}
}

/// The directions a whole entity travels in: a request or a response.
///
/// They are [`Input`] for a request and [`Output`] for a response. A fragment that changes an entity is an
/// [`EntityFragment`](crate::model::EntityFragment) instead, whose `id` and `type` are optional (NGSI-LD clause 5.4).
pub trait EntityDirection: Direction<Slot = Attribute<Self>> + sealed::Sealed {
    /// The `instanceId` an instance of a temporal evolution carries besides its attribute (clauses 4.5.7 and 4.5.8):
    /// `Option<InstanceId>` in a request, `()` in a response, whose attributes hold it among their output-only
    /// members.
    ///
    /// Tables 5.2.5-2 and 5.2.6-2 list `instanceId` among the output-only members but restrict it to "the temporal
    /// representation" of an attribute, without making it system generated as they make `createdAt`, `modifiedAt`
    /// and `deletedAt` (clause 5.2.2); Create or Update Temporal Evolution and Add Attributes take that temporal
    /// representation (clauses 5.2.20, 5.6.11.3 and 5.6.12.3). The temporal evolution a client sends may therefore
    /// name its instances, while an entity of the Core API cannot.
    type TemporalInstanceId: Debug + Clone + PartialEq + Eq + Default + Send + Sync;
}

impl sealed::Sealed for Input {}
impl sealed::Sealed for Output {}

impl EntityDirection for Input {
    type TemporalInstanceId = Option<InstanceId>;
}

impl EntityDirection for Output {
    type TemporalInstanceId = ();
}

/// An entity and its attributes.
///
/// NGSI-LD clause 5.2.4 defines it. It holds an identifier, one or more types, optional scopes and expiry, its
/// attributes in document order, and the `@context` the document carried, if any (clause 4.5.1).
///
/// `C` says whether the entity holds its `id` and `type`: every entity does ([`Complete`], the default), except
/// one in the answer to a projection that removed them ([`Projected`], clause 4.21), which a client reads and a mock
/// broker builds.
///
/// The `location`, `observationSpace` and `operationSpace` attributes are always `GeoProperty` attributes
/// (clause 5.2.4): [`Attributes`] refuses any other kind under those names, whether the entity is built or read.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{Entity, Property};
///
/// let vehicle = Entity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///     .entity_type("Vehicle")
///     .property("brandName", "Mercedes")
///     .property("speed", Property::builder(80))
///     .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
///     .build()?;
/// assert_eq!(vehicle.attributes.len(), 3);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entity<D: EntityDirection, C: Completeness = Complete> {
    /// `id`; optional only in a projected entity.
    pub id: C::Member<EntityId>,
    /// `type`: one entity type, or several (clause 4.16); optional only in a projected entity.
    pub entity_type: C::Member<OneOrMany<EntityTypeName>>,
    /// `scope`: one scope, or several (clause 4.18).
    pub scope: Option<OneOrMany<Scope>>,
    /// `expiresAt` (clause 4.22).
    pub expires_at: Option<DateTime>,
    /// The attributes, in document order.
    pub attributes: Attributes<D>,
    /// The output-only members; `()` for a request.
    pub output: D::EntityOutput,
    /// The `@context` the document carried (clause 4.5.1). [`render`](Entity::render) writes it; a request
    /// body never carries it, because the request pipeline delivers the endpoint's context instead
    /// (clause 6.3.5).
    pub context: Option<Context>,
}

#[bon]
impl Entity<Input> {
    /// Starts a builder. `id` and `entity_type` are required, which the compiler checks; attributes are added
    /// with the per-kind methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid identifier, type, scope, attribute name, value and
    /// instance set, every empty list of types or scopes, and every `location`, `observationSpace` or
    /// `operationSpace` that is not a `GeoProperty`.
    #[builder]
    pub fn new(
        #[builder(field)] attributes: PendingAttributes,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        #[builder(into)] scope: Option<ScopeDraft>,
        expires_at: Option<DateTime>,
        #[builder(into)] context: Option<Context>,
    ) -> Result<Self, BuildError> {
        assemble(
            EntityDrafts {
                id: Some(id),
                entity_type: Some(entity_type),
                scope,
                expires_at,
                context,
            },
            attributes,
            (),
        )
    }
}

/// The members every entity builder takes besides the attributes and the output-only members. `id` and `type` are
/// `None` only when a builder of a projected entity was not given them (clause 4.21).
pub(crate) struct EntityDrafts {
    /// `id`.
    pub(crate) id: Option<IdDraft>,
    /// `type`.
    pub(crate) entity_type: Option<TypeDraft>,
    /// `scope`.
    pub(crate) scope: Option<ScopeDraft>,
    /// `expiresAt`.
    pub(crate) expires_at: Option<DateTime>,
    /// `@context`.
    pub(crate) context: Option<Context>,
}

/// An identifying member as a builder resolved it (`Ok(None)` when it was not given, [`Recorded`] when it was invalid
/// and recorded its problem), held as completeness `C` holds it. A builder of a complete entity requires the member,
/// so only a projected entity lacks one that is valid.
fn identifying<C: Completeness, T: Debug + Clone + PartialEq + Eq + Send + Sync>(member: Result<Option<T>, Recorded>) -> Option<C::Member<T>> {
    member.ok().and_then(|member| C::from_read(Seal::new(), member))
}

/// Resolves an optional draft with `resolve`, which records why the draft is invalid.
fn resolve_given<D, T>(draft: Option<D>, resolve: impl FnOnce(D) -> Option<T>) -> Result<Option<T>, Recorded> {
    draft.map_or(Ok(None), |draft| resolve(draft).map(Some).ok_or(Recorded))
}

/// Validates what an entity builder collected and assembles the entity, recording every problem.
pub(crate) fn assemble<D: EntityDirection, C: Completeness>(
    drafts: EntityDrafts,
    attributes: PendingAttributes<D>,
    output: D::EntityOutput,
) -> Result<Entity<D, C>, BuildError> {
    let mut problems = Vec::new();
    let id = resolve_given(drafts.id, |id| id.resolve(&mut problems));
    let entity_type = resolve_given(drafts.entity_type, |entity_type| entity_type.resolve(&mut problems));
    let scope = ScopeDraft::resolve_optional(drafts.scope, &mut problems);
    let attributes = attributes.resolve(&mut problems);
    // An unresolved member has recorded its problem, so the error is never empty.
    if let (Some(id), Some(entity_type), Ok(scope)) = (identifying::<C, _>(id), identifying::<C, _>(entity_type), scope) {
        BuildError::finish(
            problems,
            Entity {
                id,
                entity_type,
                scope,
                expires_at: drafts.expires_at,
                attributes,
                output,
                context: drafts.context,
            },
        )
    } else {
        Err(BuildError::new(problems))
    }
}

attribute_methods!(EntityBuilder, entity_builder::State, attributes);

impl Entity<Output> {
    /// Drops the output-only members at every level (clause 5.2.2), giving an entity a client can send back,
    /// e.g. to Replace Entity after changing it. The `@context` is kept.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] naming every attribute that holds the NGSI-LD Null, which a broker writes for a
    /// deleted attribute and a request cannot carry (clause 5.5.4).
    pub fn into_input(self) -> Result<Entity<Input>, BuildError> {
        let mut problems = Vec::new();
        let attributes = self.attributes.into_input(&mut problems);
        BuildError::finish(
            problems,
            Entity {
                id: self.id,
                entity_type: self.entity_type,
                scope: self.scope,
                expires_at: self.expires_at,
                attributes,
                output: (),
                context: self.context,
            },
        )
    }
}

/// A complete entity is a projected one that kept both its `id` and its `type` (clause 4.21): the inline linked
/// entities of a response (clause 4.5.23.2) are projected entities, and a mock broker builds them complete. Only a
/// response is projected: a request entity always holds its `id` and `type` (clause 5.6.1), so no `Entity<Input,
/// Projected>` is made.
impl From<Entity<Output, Complete>> for Entity<Output, Projected> {
    fn from(entity: Entity<Output, Complete>) -> Self {
        Entity {
            id: Some(entity.id),
            entity_type: Some(entity.entity_type),
            scope: entity.scope,
            expires_at: entity.expires_at,
            attributes: entity.attributes,
            output: entity.output,
            context: entity.context,
        }
    }
}

/// One complete entity is the one projected entity the `entity` member of a Relationship holds (clause 4.5.23.2).
impl From<Entity<Output, Complete>> for OneOrMany<Entity<Output, Projected>> {
    fn from(entity: Entity<Output, Complete>) -> Self {
        OneOrMany::one(entity.into())
    }
}

impl<D: EntityDirection, C: Completeness> Entity<D, C> {
    /// Returns the instances of the attribute `name`.
    #[must_use]
    pub fn attribute(&self, name: &AttributeName) -> Option<&Instances<D::Slot>> {
        self.attributes.get(name)
    }
}

#[cfg(test)]
mod tests;
