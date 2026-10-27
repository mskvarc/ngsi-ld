//! Attributes a builder has been given, kept with their names as written until the builder finishes, and the
//! two steps every builder takes to place one: reading the name, and adding the attribute under it.

use crate::{
    model::{
        Attribute,
        AttributeName,
        AttributeNameDraft,
        Attributes,
        AttributesError,
        BuildError,
        BuildProblem,
        Direction,
        EntityDirection,
        Input,
        InstanceId,
        Output,
        ProblemKind,
        draft::Draft,
        evolution::{DeletedInstance, TemporalAttributes, TemporalInstanceMembers},
    },
    seal::Seal,
};

/// Reads the name given to a builder for the attribute at `index`, counting from 0 in the order the builder was
/// given them.
///
/// # Errors
///
/// Returns [`ProblemKind::AttributeName`] when the name is not an attribute name.
pub(crate) fn resolve_name(index: usize, name: AttributeNameDraft) -> Result<AttributeName, ProblemKind> {
    name.into_result(Seal::new()).map_err(|source| ProblemKind::AttributeName { index, source })
}

/// Adds a slot under `name`.
///
/// # Errors
///
/// Returns [`ProblemKind::Attributes`] when the map refuses the slot.
pub(crate) fn insert<D: Direction>(attributes: &mut Attributes<D>, name: AttributeName, slot: D::Slot) -> Result<(), ProblemKind> {
    // The name is reported if the slot is refused, so the map gets a copy.
    attributes.insert(name.clone(), slot).map_err(|source| ProblemKind::Attributes { name, source })
}

/// One pending attribute: the name as given and the attribute, or the problems found building it.
#[derive(Debug)]
struct PendingEntry<D: EntityDirection> {
    /// The attribute name as the caller gave it, validated when the builder finishes.
    name: AttributeNameDraft,
    /// The attribute, or the problems found building it.
    attribute: Result<Attribute<D>, BuildError>,
    /// The `instanceId` a temporal request names for the instance and the deletion it records; only a temporal entity
    /// builder sets them.
    members: D::TemporalMembers,
}

/// The attributes a builder collects; validated together when the builder finishes. `D` is the direction of the
/// document built: a request ([`Input`], the default) or a response ([`Output`]).
#[derive(Debug, Default)]
pub(crate) struct PendingAttributes<D: EntityDirection = Input>(Vec<PendingEntry<D>>);

impl PendingAttributes<Input> {
    /// Adds an attribute built (or failed to build) by a draft.
    pub(crate) fn push_attribute(&mut self, name: impl Into<AttributeNameDraft>, attribute: Result<Attribute<Input>, BuildError>) {
        self.0.push(PendingEntry {
            name: name.into(),
            attribute,
            members: TemporalInstanceMembers::default(),
        });
    }

    /// Adds an instance of a temporal evolution that names its `instanceId`.
    pub(crate) fn push_identified(&mut self, name: impl Into<AttributeNameDraft>, instance_id: InstanceId, attribute: Result<Attribute<Input>, BuildError>) {
        self.0.push(PendingEntry {
            name: name.into(),
            attribute,
            members: TemporalInstanceMembers::new(Some(instance_id), None),
        });
    }

    /// Adds the record of a deleted instance of a temporal evolution.
    pub(crate) fn push_deleted(&mut self, name: impl Into<AttributeNameDraft>, deleted: DeletedInstance) {
        let (attribute, members) = deleted.into_instance();
        self.0.push(PendingEntry {
            name: name.into(),
            attribute: Ok(attribute),
            members,
        });
    }
}

impl PendingAttributes<Output> {
    /// Adds an attribute a request draft built (or failed to build), as a response carries it with no output-only
    /// member.
    pub(crate) fn push_attribute(&mut self, name: impl Into<AttributeNameDraft>, attribute: Result<Attribute<Input>, BuildError>) {
        self.push_output(name, attribute.map(Attribute::into_output));
    }

    /// Adds a response attribute built (or failed to build) by a draft, with its output-only members.
    pub(crate) fn push_output(&mut self, name: impl Into<AttributeNameDraft>, attribute: Result<Attribute<Output>, BuildError>) {
        self.0.push(PendingEntry {
            name: name.into(),
            attribute,
            members: (),
        });
    }
}

impl<D: EntityDirection> PendingAttributes<D> {
    /// Validates every name and instance set, collecting every problem.
    pub(crate) fn resolve(self, problems: &mut Vec<BuildProblem>) -> Attributes<D> {
        // Only a temporal entity builder names instance ids or records deletions, so an entity's pending attributes hold
        // none.
        self.place(Attributes::new(), problems, |attributes, name, attribute, _| attributes.insert(name, attribute))
    }

    /// Validates every name, collecting every problem, and adds each attribute to the evolution of its name, in the
    /// order given (clause 4.5.7).
    pub(crate) fn resolve_evolutions(self, problems: &mut Vec<BuildProblem>) -> TemporalAttributes<D> {
        self.place(TemporalAttributes::new(), problems, TemporalAttributes::insert_instance)
    }

    /// Validates every name and adds each attribute to `target` with `add`, collecting every problem.
    fn place<T>(
        self,
        mut target: T,
        problems: &mut Vec<BuildProblem>,
        mut add: impl FnMut(&mut T, AttributeName, Attribute<D>, D::TemporalMembers) -> Result<(), AttributesError>,
    ) -> T {
        for (index, PendingEntry { name, attribute, members }) in self.0.into_iter().enumerate() {
            let name = match resolve_name(index, name) {
                Ok(name) => name,
                Err(problem) => {
                    problems.push(BuildProblem::top(problem));
                    continue;
                }
            };
            match attribute {
                // The name is reported if the attribute is refused, so the target gets a copy.
                Ok(attribute) => {
                    if let Err(source) = add(&mut target, name.clone(), attribute, members) {
                        problems.push(BuildProblem::top(ProblemKind::Attributes { name, source }));
                    }
                }
                Err(error) => problems.extend(error.within(&name)),
            }
        }
        target
    }
}
