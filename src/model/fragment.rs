//! Fragments: documents that change an existing entity (NGSI-LD clause 5.4), for Merge Entity (clause 5.6.17),
//! Update and Append Attributes (clauses 5.6.2 and 5.6.3), and Batch Entity Update and Merge (clauses 5.6.9 and
//! 5.6.20).

use crate::model::{
    Attribute,
    AttributeKind,
    AttributeName,
    AttributeNameDraft,
    Attributes,
    BuildError,
    BuildProblem,
    Change,
    Complete,
    Completeness,
    Context,
    DatasetId,
    DateTime,
    Deletion,
    Entity,
    EntityId,
    EntityTypeName,
    IdDraft,
    Input,
    InstanceDeletion,
    InstanceKey,
    OneOrMany,
    Patch,
    ProblemKind,
    Projected,
    Scope,
    TypeDraft,
    attribute::pending::{insert, resolve_name},
};
use bon::bon;

#[cfg(feature = "linked-data")]
mod names;

/// What a fragment does to the `scope` of an entity.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum ScopeChange {
    /// Sets the scopes.
    Set(OneOrMany<Scope>),
    /// Deletes every scope (the NGSI-LD Null of clause 4.18).
    Delete,
}

/// An entity fragment: the members to add, change or delete in an existing entity.
///
/// NGSI-LD clause 5.4.
///
/// `C` says whether the fragment names its entity. A fragment sent to one entity ([`Projected`], the default)
/// has an optional `id` and `type`, because the HTTP binding names the entity in the path. A fragment in the body
/// of Batch Entity Update or Merge ([`Complete`], built with [`for_entity`](EntityFragment::for_entity)) holds
/// both, because each element of the batch represents an entity "as mandated by clause 5.2.4" (clauses 5.6.9.3
/// and 5.6.20.3). A fragment that names its entity is also a valid fragment of one entity, so the single-entity
/// operations accept either.
///
/// Every attribute is a [`Change`]: [`Change::Set`] writes it, [`Change::Delete`] writes the NGSI-LD Null in the
/// shape its kind needs. A deletion written as the bare Null ([`delete`](EntityFragmentBuilder::delete)) is the
/// whole member, so it cannot be combined with other instances of the same attribute; [`delete_instance`] names
/// one instance instead.
///
/// [`delete_instance`]: EntityFragmentBuilder::delete_instance
///
/// ```
/// use ngsi_ld::model::{AttributeKind, EntityFragment};
///
/// let fragment = EntityFragment::builder()
///     .property("speed", 0)
///     .delete("isParked")
///     .delete_kind("street", AttributeKind::LanguageProperty)
///     .build()?;
/// assert_eq!(fragment.attributes.len(), 3);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityFragment<C: Completeness = Projected> {
    /// `id`: the target; optional in a fragment of one entity, whose path names it.
    pub id: C::Member<EntityId>,
    /// `type`: entity types to add to the target (clause 5.5.8); optional in a fragment of one entity.
    pub entity_type: C::Member<OneOrMany<EntityTypeName>>,
    /// `scope`: what to do with the scopes.
    pub scope: Option<ScopeChange>,
    /// `expiresAt`.
    pub expires_at: Option<DateTime>,
    /// The attribute changes, in document order.
    pub attributes: Attributes<Patch>,
    /// The `@context` the document carried (clause 4.5.1), as a fragment forwarded in JSON-LD holds it.
    /// [`render`](EntityFragment::render) writes it; a request body never carries it, because the request pipeline
    /// delivers the endpoint's (clause 6.3.5).
    pub context: Option<Context>,
}

/// A change waiting for the builder to finish: an attribute or a deletion, under a name as written.
#[derive(Debug)]
enum PendingChange {
    /// Writes an attribute; boxed, being much larger than the other variants.
    Set(Box<Result<Attribute<Input>, BuildError>>),
    /// Writes an attribute fragment, which may delete members of the attribute; boxed for the same reason.
    Patch(Box<Attribute<Patch>>),
    /// Deletes an attribute or instance.
    Delete(Deletion),
    /// Deletes a sub-attribute of an attribute set earlier in the same fragment.
    DeleteSubAttribute {
        /// The name of the sub-attribute as the caller gave it, validated when the builder finishes.
        sub_attribute: AttributeNameDraft,
    },
}

/// The changes a fragment builder collects; validated together when it finishes.
#[derive(Debug, Default)]
pub struct PendingChanges(Vec<(AttributeNameDraft, PendingChange)>);

impl PendingChanges {
    /// Adds an attribute to write.
    pub(crate) fn push_attribute(&mut self, name: impl Into<AttributeNameDraft>, attribute: Result<Attribute<Input>, BuildError>) {
        self.0.push((name.into(), PendingChange::Set(Box::new(attribute))));
    }

    /// Adds an attribute fragment to write.
    fn push_patch(&mut self, name: AttributeNameDraft, attribute: Attribute<Patch>) {
        self.0.push((name, PendingChange::Patch(Box::new(attribute))));
    }

    /// Adds the deletion of an attribute or of one of its instances.
    fn push_deletion(&mut self, name: AttributeNameDraft, deletion: Deletion) {
        self.0.push((name, PendingChange::Delete(deletion)));
    }

    /// Adds the deletion of a sub-attribute of an attribute set earlier in the same fragment.
    fn push_sub_attribute_deletion(&mut self, name: AttributeNameDraft, sub_attribute: AttributeNameDraft) {
        self.0.push((name, PendingChange::DeleteSubAttribute { sub_attribute }));
    }

    /// Validates every change, collecting every problem.
    fn resolve(self, problems: &mut Vec<BuildProblem>) -> Attributes<Patch> {
        let mut attributes = Attributes::<Patch>::new();
        for (index, (name, change)) in self.0.into_iter().enumerate() {
            let name = match resolve_name(index, name) {
                Ok(name) => name,
                Err(problem) => {
                    problems.push(BuildProblem::top(problem));
                    continue;
                }
            };
            let slot = match change {
                PendingChange::Set(attribute) => match *attribute {
                    Ok(attribute) => Change::Set(attribute.into_patch()),
                    Err(error) => {
                        problems.extend(error.within(&name));
                        continue;
                    }
                },
                PendingChange::Patch(attribute) => Change::Set(*attribute),
                PendingChange::Delete(deletion) => Change::Delete(deletion),
                PendingChange::DeleteSubAttribute { sub_attribute } => {
                    delete_sub_attribute(&mut attributes, &name, (index, sub_attribute), problems);
                    continue;
                }
            };
            if let Err(problem) = insert(&mut attributes, name, slot) {
                problems.push(BuildProblem::top(problem));
            }
        }
        attributes
    }
}

/// Adds the deletion of `sub_attribute` to the default instance of `name`, which an earlier call set.
fn delete_sub_attribute(
    attributes: &mut Attributes<Patch>,
    name: &AttributeName,
    (index, sub_attribute): (usize, AttributeNameDraft),
    problems: &mut Vec<BuildProblem>,
) {
    let sub_name = match resolve_name(index, sub_attribute) {
        Ok(sub_name) => sub_name,
        Err(problem) => {
            problems.push(BuildProblem::of(name, problem));
            return;
        }
    };
    let target = attributes.get_mut(name).and_then(|instances| {
        instances.instances_mut().find_map(|slot| match slot {
            Change::Set(attribute) if attribute.is_default_instance() => Some(attribute),
            Change::Set(_) | Change::Delete(_) => None,
        })
    });
    match target {
        Some(attribute) => {
            if let Err(problem) = insert(&mut attribute.meta_mut().attributes, sub_name, Change::Delete(Deletion::Member)) {
                problems.push(BuildProblem::of(name, problem));
            }
        }
        // Clone: the problem owns the name, while the caller keeps it.
        None => problems.push(BuildProblem::top(ProblemKind::SubAttributeOfUnsetAttribute { name: name.clone() })),
    }
}

/// Generates, on the fragment builder type `$builder<S>` whose typestate trait is `$state`, the methods that delete:
/// `delete`, `delete_kind`, `delete_instance` and `delete_sub_attribute`, and `patch`, which writes an attribute
/// fragment that may delete members of the attribute; each adds a change to the builder's `changes` field. Both
/// fragment builders take them, so a fragment that names its entity deletes the same way.
macro_rules! deletion_methods {
    ($builder:ident, $state:path) => {
        impl<S: $state> $builder<S> {
            /// Writes the attribute `name` as the fragment `attribute`, which may delete some of the attribute's
            /// optional members, such as its `unitCode`, with the NGSI-LD Null (clauses 5.5.8 and 5.5.12); one
            /// made with [`Attribute::delete_member`].
            pub fn patch(mut self, name: impl Into<AttributeNameDraft>, attribute: Attribute<Patch>) -> Self {
                self.changes.push_patch(name.into(), attribute);
                self
            }

            /// Deletes the attribute `name` (its default instance), writing the bare NGSI-LD Null clauses 4.5.3.3
            /// and 4.5.18.3 give for fragments, in either representation. The bare Null is the whole member, so
            /// `name` takes no other instance in the same fragment.
            pub fn delete(mut self, name: impl Into<AttributeNameDraft>) -> Self {
                self.changes.push_deletion(name.into(), Deletion::Member);
                self
            }

            /// Deletes the attribute `name` of a known kind, writing the NGSI-LD Null in the shape the kind needs
            /// (clause 4.5.0): `{"type": "LanguageProperty", "languageMap": {"@none": "urn:ngsi-ld:null"}}`, ...
            /// This is the deletion a normalized fragment writes with its `type` (clause 5.4, example 2).
            pub fn delete_kind(mut self, name: impl Into<AttributeNameDraft>, kind: AttributeKind) -> Self {
                self.changes.push_deletion(name.into(), Deletion::Instance(InstanceDeletion::new(kind)));
                self
            }

            /// Deletes one instance of the attribute `name` (clause 5.5.8).
            pub fn delete_instance(mut self, name: impl Into<AttributeNameDraft>, kind: AttributeKind, dataset_id: impl Into<DatasetId>) -> Self {
                self.changes
                    .push_deletion(name.into(), Deletion::Instance(InstanceDeletion::new(kind).with_dataset_id(dataset_id)));
                self
            }

            /// Deletes the sub-attribute `sub_attribute` of the attribute `name`, which an earlier call in the
            /// same builder sets (clause 5.4, example 2).
            pub fn delete_sub_attribute(mut self, name: impl Into<AttributeNameDraft>, sub_attribute: impl Into<AttributeNameDraft>) -> Self {
                self.changes.push_sub_attribute_deletion(name.into(), sub_attribute.into());
                self
            }
        }
    };
}

#[bon]
impl EntityFragment<Projected> {
    /// Starts the builder of a fragment of one entity, whose `id` and `type` are optional. Attributes to write are
    /// added with the per-kind methods (`property`, ...); deletions with [`delete`](EntityFragmentBuilder::delete)
    /// and its variants.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every problem, including a bare deletion
    /// ([`delete`](EntityFragmentBuilder::delete)) of a name that has other instances in the fragment, and a
    /// deletion of `location`, `observationSpace` or `operationSpace` that names a kind other than `GeoProperty`.
    #[builder]
    pub fn new(
        #[builder(field)] changes: PendingChanges,
        #[builder(into)] id: Option<EntityId>,
        #[builder(into)] entity_type: Option<TypeDraft>,
        scope: Option<ScopeChange>,
        expires_at: Option<DateTime>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let entity_type = entity_type.map(|entity_type| entity_type.resolve(&mut problems));
        let attributes = changes.resolve(&mut problems);
        // An unresolved type has recorded its problem, so the error is never empty.
        match entity_type {
            Some(None) => Err(BuildError::new(problems)),
            Some(Some(entity_type)) => BuildError::finish(
                problems,
                Self {
                    id,
                    entity_type: Some(entity_type),
                    scope,
                    expires_at,
                    attributes,
                    context: None,
                },
            ),
            None => BuildError::finish(
                problems,
                Self {
                    id,
                    entity_type: None,
                    scope,
                    expires_at,
                    attributes,
                    context: None,
                },
            ),
        }
    }
}

#[bon]
impl EntityFragment<Complete> {
    /// Starts the builder of a fragment that names its entity: the element of Batch Entity Update or Merge
    /// (clauses 5.6.9.3 and 5.6.20.3), whose `id` and `type` are mandatory (clause 5.2.4). It takes the same
    /// attribute and deletion methods as [`builder`](EntityFragment::builder).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::EntityFragment;
    ///
    /// let fragment = EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
    ///     .property("speed", 0)
    ///     .delete("isParked")
    ///     .build()?;
    /// assert_eq!(fragment.id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every problem: an invalid id or type, and every problem
    /// [`builder`](EntityFragment::builder) reports.
    #[builder(finish_fn = build)]
    pub fn for_entity(
        #[builder(start_fn, into)] id: IdDraft,
        #[builder(start_fn, into)] entity_type: TypeDraft,
        #[builder(field)] changes: PendingChanges,
        scope: Option<ScopeChange>,
        expires_at: Option<DateTime>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let id = id.resolve(&mut problems);
        let entity_type = entity_type.resolve(&mut problems);
        let attributes = changes.resolve(&mut problems);
        // An unresolved member has recorded its problem, so the error is never empty.
        if let (Some(id), Some(entity_type)) = (id, entity_type) {
            BuildError::finish(
                problems,
                Self {
                    id,
                    entity_type,
                    scope,
                    expires_at,
                    attributes,
                    context: None,
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }
}

attribute_methods!(EntityFragmentBuilder, entity_fragment_builder::State, changes);
attribute_methods!(EntityFragmentForEntityBuilder, entity_fragment_for_entity_builder::State, changes);
deletion_methods!(EntityFragmentBuilder, entity_fragment_builder::State);
deletion_methods!(EntityFragmentForEntityBuilder, entity_fragment_for_entity_builder::State);

impl Entity<Input> {
    /// Turns the entity into a fragment that writes every attribute: `id`, `type` and `scope` are kept and every
    /// attribute becomes a [`Change::Set`]. The fragment names its entity, so it is an element of a batch as well
    /// as a fragment of one entity. The `@context` is not part of a fragment, whose request carries the
    /// endpoint's (clause 6.3.5).
    #[must_use]
    pub fn into_fragment(self) -> EntityFragment<Complete> {
        EntityFragment {
            id: self.id,
            entity_type: self.entity_type,
            scope: self.scope.map(ScopeChange::Set),
            expires_at: self.expires_at,
            attributes: self.attributes.convert::<Patch>(|_, attribute| Change::Set(attribute.into_patch())),
            context: None,
        }
    }
}
