//! Fragments in simplified representation (NGSI-LD clauses 4.5.4 and 5.6.17): the body of Merge Entity with
//! `format=simplified` (clause 6.5.3.4).

use crate::model::{
    AttributeName,
    AttributeNameDraft,
    AttributesError,
    BuildError,
    BuildProblem,
    DatasetId,
    EntityId,
    EntityTypeName,
    InstancesError,
    OneOrMany,
    ProblemKind,
    RawJson,
    ScopeChange,
    SimplifiedValue,
    TypeDraft,
    Value,
    attribute::{attributes::holds_geometries, pending::resolve_name},
    null::{NGSI_LD_NULL, check_simplified},
    representation::write::write_fragment_head,
    simplified::dataset_json,
};
use bon::bon;
use indexmap::{IndexMap, map::Entry};
use jstrict::{Object, Value as Json};

#[cfg(feature = "linked-data")]
mod names;

/// What a simplified fragment does to one instance of an attribute.
///
/// NGSI-LD clause 5.6.17.4.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum SimplifiedChange {
    /// Merges the value into the instance, keeping the kind the attribute has, or appends a new instance.
    Set(SimplifiedValue),
    /// Deletes the instance, by writing the NGSI-LD Null (clause 4.5.0).
    Delete,
}

impl SimplifiedChange {
    /// Writes the change: the simplified value, or the NGSI-LD Null.
    fn to_json(&self) -> Json {
        match self {
            Self::Set(value) => value.to_json(),
            Self::Delete => Json::from(NGSI_LD_NULL.as_str()),
        }
    }
}

/// What a simplified fragment does to one attribute.
///
/// It is a change to its default instance, written as the bare member value, or changes to instances named by dataset
/// id, written `{"dataset": {...}}` with `@none` for the default instance (NGSI-LD clauses 4.5.4 and 4.5.5).
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum SimplifiedAttributeChange {
    /// The default instance.
    Default(SimplifiedChange),
    /// Instances by dataset id; never empty, and never holding a `dataset` value.
    Dataset(IndexMap<DatasetId, SimplifiedChange>),
}

impl SimplifiedAttributeChange {
    /// Writes the change.
    fn to_json(&self) -> Json {
        match self {
            Self::Default(change) => change.to_json(),
            Self::Dataset(changes) => dataset_json(changes.iter().map(|(dataset_id, change)| (dataset_id, change.to_json()))),
        }
    }
}

/// An entity fragment in simplified representation.
///
/// It is the body of Merge Entity with `format=simplified` (NGSI-LD clauses 4.5.4 and 5.6.17).
///
/// Each attribute is a bare value the broker merges into the attribute the entity holds, keeping its kind
/// (clause 5.6.17.4); a `LanguageProperty` given as a string is merged under the request's `lang`. `id`, `type` and
/// `scope` are optional, because the HTTP binding names the entity in the path. Built only with
/// [`SimplifiedFragment::builder`], which refuses what the representation cannot carry: a value holding the
/// NGSI-LD Null (a deletion is [`delete`](SimplifiedFragmentBuilder::delete)), a value other than a geometry under
/// `location`, `observationSpace` or `operationSpace` (clause 5.2.4), and an instance named twice (clause 4.5.5).
///
/// It deletes with the NGSI-LD Null of clause 4.5.0, which the crate writes and the caller never spells.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{SimplifiedFragment, Value};
///
/// let fragment = SimplifiedFragment::builder()
///     .value("speed", 55)
///     .value("street", "Grand Place")
///     .set_instance(
///         "temperature",
///         iri!("urn:ngsi-ld:dataset:gps"),
///         Value::from(21),
///     )
///     .delete("isParked")
///     .build()?;
/// assert_eq!(fragment.attributes().len(), 4);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimplifiedFragment {
    /// The `id` of the entity; `None` when only the request path names it.
    id: Option<EntityId>,
    /// The `type` of the entity; `None` when only the request path names the entity.
    entity_type: Option<OneOrMany<EntityTypeName>>,
    /// The change to `scope`; `None` leaves the scope as it is.
    scope: Option<ScopeChange>,
    /// The change to each attribute, in the order given.
    attributes: IndexMap<AttributeName, SimplifiedAttributeChange>,
}

/// One change a simplified fragment builder was given, under the name as written.
#[derive(Debug)]
struct PendingSimplifiedChange {
    /// The attribute name as given.
    name: AttributeNameDraft,
    /// The instance named; `None` for the bare member.
    dataset_id: Option<DatasetId>,
    /// The change.
    change: SimplifiedChange,
}

/// The changes a simplified fragment builder collects.
///
/// They are validated together when the builder finishes.
#[derive(Debug, Default)]
pub struct PendingSimplifiedChanges(Vec<PendingSimplifiedChange>);

impl PendingSimplifiedChanges {
    /// Adds a change.
    fn push(&mut self, name: AttributeNameDraft, dataset_id: Option<DatasetId>, change: SimplifiedChange) {
        self.0.push(PendingSimplifiedChange { name, dataset_id, change });
    }

    /// Validates every change and groups the changes by attribute, in the order the names first appear, recording
    /// every problem.
    fn resolve(self, problems: &mut Vec<BuildProblem>) -> IndexMap<AttributeName, SimplifiedAttributeChange> {
        let mut grouped: IndexMap<AttributeName, Vec<(Option<DatasetId>, SimplifiedChange)>> = IndexMap::new();
        for (index, PendingSimplifiedChange { name, dataset_id, change }) in self.0.into_iter().enumerate() {
            let name = match resolve_name(index, name) {
                Ok(name) => name,
                Err(problem) => {
                    problems.push(BuildProblem::top(problem));
                    continue;
                }
            };
            match check_change(&name, &change) {
                Ok(()) => grouped.entry(name).or_default().push((dataset_id, change)),
                Err(problem) => problems.push(BuildProblem::of(&name, problem)),
            }
        }
        grouped
            .into_iter()
            .filter_map(|(name, changes)| match group(changes) {
                Ok(change) => Some((name, change)),
                Err(source) => {
                    problems.push(BuildProblem::top(ProblemKind::Attributes {
                        name,
                        source: AttributesError::Instances { source },
                    }));
                    None
                }
            })
            .collect()
    }
}

/// Checks one change against what a simplified fragment can carry.
fn check_change(name: &AttributeName, change: &SimplifiedChange) -> Result<(), ProblemKind> {
    let SimplifiedChange::Set(value) = change else {
        return Ok(());
    };
    match value {
        SimplifiedValue::Dataset(_) => Err(ProblemKind::DatasetValue),
        SimplifiedValue::Geometry(_) => Ok(()),
        SimplifiedValue::Value(_) | SimplifiedValue::LanguageMap(_) | SimplifiedValue::Vocab(_) | SimplifiedValue::Json(_) if holds_geometries(name) => {
            Err(ProblemKind::Attributes {
                // The name is reported with the problem, so the problem owns a copy.
                name: name.clone(),
                source: AttributesError::GeoPropertyRequired,
            })
        }
        SimplifiedValue::Value(_) | SimplifiedValue::LanguageMap(_) | SimplifiedValue::Vocab(_) | SimplifiedValue::Json(_) => {
            check_simplified(value).map_err(|member| ProblemKind::NgsiLdNull { member })
        }
    }
}

/// Groups the changes of one attribute: a bare change on its own is the default instance; anything else is a
/// `dataset` object, in which the bare change is the `@none` instance. A bare deletion is the whole member, so it
/// stands alone (clause 4.5.3.3).
fn group(changes: Vec<(Option<DatasetId>, SimplifiedChange)>) -> Result<SimplifiedAttributeChange, InstancesError> {
    if let [(None, _)] = changes.as_slice() {
        return changes
            .into_iter()
            .next()
            .map(|(_, change)| SimplifiedAttributeChange::Default(change))
            .ok_or(InstancesError::Empty);
    }
    if changes.iter().any(|(dataset_id, change)| dataset_id.is_none() && change.is_delete()) {
        return Err(InstancesError::BareDeletionAmongInstances);
    }
    let mut instances = IndexMap::new();
    for (dataset_id, change) in changes {
        match instances.entry(dataset_id.unwrap_or(DatasetId::DefaultInstance)) {
            Entry::Vacant(vacant) => {
                vacant.insert(change);
            }
            Entry::Occupied(occupied) => {
                return Err(match occupied.key() {
                    DatasetId::DefaultInstance => InstancesError::DefaultTwice,
                    // The error names the shared dataset id, so it owns a copy.
                    dataset_id @ DatasetId::Named(_) => InstancesError::DuplicateDatasetId {
                        dataset_id: dataset_id.clone(),
                    },
                });
            }
        }
    }
    Ok(SimplifiedAttributeChange::Dataset(instances))
}

#[bon]
impl SimplifiedFragment {
    /// Starts a builder. Values are added with [`value`](SimplifiedFragmentBuilder::value) and
    /// [`set`](SimplifiedFragmentBuilder::set), instances with
    /// [`set_instance`](SimplifiedFragmentBuilder::set_instance), deletions with
    /// [`delete`](SimplifiedFragmentBuilder::delete) and
    /// [`delete_instance`](SimplifiedFragmentBuilder::delete_instance).
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every problem: an invalid attribute name or entity type, a value
    /// holding the NGSI-LD Null, a `dataset` value, a value other than a geometry under `location`,
    /// `observationSpace` or `operationSpace`, an instance named twice, and a bare deletion of a name that has other
    /// instances in the fragment.
    #[builder]
    pub fn new(
        #[builder(field)] changes: PendingSimplifiedChanges,
        #[builder(into)] id: Option<EntityId>,
        #[builder(into)] entity_type: Option<TypeDraft>,
        scope: Option<ScopeChange>,
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
                    attributes,
                },
            ),
            None => BuildError::finish(
                problems,
                Self {
                    id,
                    entity_type: None,
                    scope,
                    attributes,
                },
            ),
        }
    }
}

impl<S: simplified_fragment_builder::State> SimplifiedFragmentBuilder<S> {
    /// Merges `value` into the default instance of the attribute `name`: a number, a string, a boolean, an array,
    /// an object, or a Relationship's object written as its URI string (clause 4.5.4).
    pub fn value(self, name: impl Into<AttributeNameDraft>, value: impl Into<Value>) -> Self {
        self.set(name, SimplifiedValue::Value(value.into()))
    }

    /// Merges a simplified value of any kind into the default instance of the attribute `name`: a value, a
    /// geometry, a language map, a vocabulary or raw JSON (clause 4.5.4).
    pub fn set(mut self, name: impl Into<AttributeNameDraft>, value: impl Into<SimplifiedValue>) -> Self {
        self.changes.push(name.into(), None, SimplifiedChange::Set(value.into()));
        self
    }

    /// Merges a simplified value into the instance `dataset_id` of the attribute `name` (clause 4.5.5);
    /// [`DatasetId::DefaultInstance`] names the default instance.
    pub fn set_instance(mut self, name: impl Into<AttributeNameDraft>, dataset_id: impl Into<DatasetId>, value: impl Into<SimplifiedValue>) -> Self {
        self.changes.push(name.into(), Some(dataset_id.into()), SimplifiedChange::Set(value.into()));
        self
    }

    /// Deletes the attribute `name`, writing the bare NGSI-LD Null as the member value (clause 4.5.0). The bare Null
    /// is the whole member, so `name` takes no other instance in the same fragment.
    pub fn delete(mut self, name: impl Into<AttributeNameDraft>) -> Self {
        self.changes.push(name.into(), None, SimplifiedChange::Delete);
        self
    }

    /// Deletes the instance `dataset_id` of the attribute `name` (clauses 4.5.5 and 5.6.17.4).
    pub fn delete_instance(mut self, name: impl Into<AttributeNameDraft>, dataset_id: impl Into<DatasetId>) -> Self {
        self.changes.push(name.into(), Some(dataset_id.into()), SimplifiedChange::Delete);
        self
    }
}

impl SimplifiedFragment {
    /// `id`, when the fragment names its target.
    #[must_use]
    pub const fn id(&self) -> Option<&EntityId> {
        self.id.as_ref()
    }

    /// `type`: entity types to add to the target (clause 5.6.17.4).
    #[must_use]
    pub const fn entity_type(&self) -> Option<&OneOrMany<EntityTypeName>> {
        self.entity_type.as_ref()
    }

    /// `scope`: what to do with the scopes (clause 5.6.17.4).
    #[must_use]
    pub const fn scope(&self) -> Option<&ScopeChange> {
        self.scope.as_ref()
    }

    /// The attribute changes, in document order.
    #[must_use]
    pub const fn attributes(&self) -> &IndexMap<AttributeName, SimplifiedAttributeChange> {
        &self.attributes
    }

    /// Writes the fragment in simplified representation (clause 4.5.4): `id`, `type` and `scope` when set, then the
    /// attributes in their order. A fragment carries no `@context`; its request delivers the endpoint's (clause
    /// 6.3.5).
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// The fragment as a JSON object, in the member order of [`render`](Self::render).
    pub(crate) fn to_object(&self) -> Object {
        let mut object = Object::new();
        write_fragment_head(&mut object, self.id.as_ref(), self.entity_type.as_ref(), self.scope.as_ref());
        for (name, change) in &self.attributes {
            object.push(name.as_ref().into(), change.to_json());
        }
        object
    }
}

#[cfg(test)]
mod tests;
