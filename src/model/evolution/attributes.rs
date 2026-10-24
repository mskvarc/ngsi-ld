//! The attributes of a temporal evolution (NGSI-LD clauses 4.5.7 and 4.5.8): per name, the sequence of its
//! instances over time.

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    direction::names::MapDirection,
    name::mapping::{MapNames, NameMapping, map_attribute_names},
};
use crate::{
    model::{
        Attribute,
        AttributeKind,
        AttributeName,
        AttributeSlot,
        AttributesError,
        BuildError,
        BuildProblem,
        DateTime,
        EntityDirection,
        Input,
        InstanceId,
        Lossless,
        Output,
        RawJson,
        attribute::attributes::check_kind,
        evolution::write::temporal_attributes_object,
    },
    query::temporal_query::TemporalProperty,
    seal::Seal,
};
use indexmap::{IndexMap, map::Iter};
use jstrict::Value as Json;
use std::{cmp::Reverse, collections::HashSet, num::NonZeroU64, slice};

/// The temporal evolution of one attribute (NGSI-LD clauses 4.5.7 and 4.5.8): its instances in document order,
/// each recorded at a point in time. At least one; all of one kind, the instances of the same attribute; the same
/// dataset id any number of times, unlike the instances of an entity's attribute (clause 4.5.5).
///
/// In a request each instance may name its `instanceId` (see [`EntityDirection::TemporalInstanceId`]); in a response
/// the instance holds it among its output-only members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evolution<D: EntityDirection> {
    /// The instances, in document order.
    instances: Vec<Attribute<D>>,
    /// The `instanceId` each instance of a request names, at the instance's index.
    instance_ids: Vec<D::TemporalInstanceId>,
}

impl<D: EntityDirection> Evolution<D> {
    /// One instance.
    fn one(attribute: Attribute<D>, instance_id: D::TemporalInstanceId) -> Self {
        Self {
            instances: vec![attribute],
            instance_ids: vec![instance_id],
        }
    }

    /// The kind of the instances: the kind of the first one that names one. `None` when every instance is a
    /// deleted attribute reported as a bare NGSI-LD Null, which names no kind (clause 5.8.6).
    #[must_use]
    pub fn kind(&self) -> Option<AttributeKind> {
        self.instances.iter().find_map(|instance| instance.written_kind(Seal::new()))
    }

    /// Iterates over the instances in order.
    pub fn iter(&self) -> slice::Iter<'_, Attribute<D>> {
        self.instances.iter()
    }

    /// Returns the instances in order.
    #[must_use]
    pub fn as_slice(&self) -> &[Attribute<D>] {
        &self.instances
    }

    /// Iterates over the instances in order, each with the `instanceId` a request names for it.
    pub(crate) fn identified(&self) -> impl Iterator<Item = (&Attribute<D>, &D::TemporalInstanceId)> {
        self.instances.iter().zip(&self.instance_ids)
    }

    /// Returns the number of instances; never zero.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.instances.len()
    }

    /// Always `false`: an evolution has at least one instance.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Adds an instance after the others, refusing one of another kind and one that names an `instanceId` another
    /// instance names: an `instanceId` identifies one instance (clauses 4.5.7 and 4.5.8). An instance that names
    /// none, and every instance of a response, whose identifier is `()`, is the default and never collides.
    fn push(&mut self, attribute: Attribute<D>, instance_id: D::TemporalInstanceId) -> Result<(), AttributesError> {
        if let (Some(expected), Some(found)) = (self.kind(), attribute.written_kind(Seal::new()))
            && expected != found
        {
            return Err(AttributesError::KindChange { expected, found });
        }
        if instance_id != D::TemporalInstanceId::default() && self.instance_ids.contains(&instance_id) {
            return Err(AttributesError::InstanceIdRepeated);
        }
        self.instances.push(attribute);
        self.instance_ids.push(instance_id);
        Ok(())
    }
}

impl Evolution<Input> {
    /// Iterates over the `instanceId` each instance names, in the order of the instances; `None` for an instance
    /// that names none, whose identifier the broker chooses (clause 4.5.7).
    pub fn instance_ids(&self) -> impl Iterator<Item = Option<&InstanceId>> {
        self.instance_ids.iter().map(Option::as_ref)
    }
}

impl<'a, D: EntityDirection> IntoIterator for &'a Evolution<D> {
    type IntoIter = slice::Iter<'a, Attribute<D>>;
    type Item = &'a Attribute<D>;

    fn into_iter(self) -> Self::IntoIter {
        self.instances.iter()
    }
}

/// The attributes of a temporal evolution of an entity (NGSI-LD clause 5.2.20), by name, in document order: each
/// name holds the [`Evolution`] of its instances.
///
/// Every way of adding an instance goes through [`TemporalAttributes::insert`], which keeps the instances of one
/// name of one kind and the names clause 5.2.4 reserves for geometries holding `GeoProperty` instances.
///
/// ```
/// use ngsi_ld::model::{Property, Input, evolution::TemporalAttributes, AttributeName, Lossless};
///
/// let speed: AttributeName = "speed".parse()?;
/// let mut attributes = TemporalAttributes::<Input>::new();
/// attributes.insert(speed.clone(), Property::builder(120).observed_at("2018-08-01T12:03:00Z".parse()?).build()?.into())?;
/// attributes.insert(speed, Property::builder(80).observed_at("2018-08-01T12:05:00Z".parse()?).build()?.into())?;
/// assert_eq!(
///     attributes.render(Lossless::Concise).to_string(),
///     r#"{"speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"},{"value":80,"observedAt":"2018-08-01T12:05:00Z"}]}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TemporalAttributes<D: EntityDirection>(IndexMap<AttributeName, Evolution<D>>);

impl<D: EntityDirection> TemporalAttributes<D> {
    /// No attributes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an instance to the evolution of `name`, after the instances already there.
    ///
    /// # Errors
    ///
    /// Returns [`AttributesError::KindChange`] when the instance has another kind than the ones before it, and
    /// [`AttributesError::GeoPropertyRequired`] when `name` is `location`, `observationSpace` or `operationSpace`
    /// and the instance is not a `GeoProperty` (clause 5.2.4).
    pub fn insert(&mut self, name: AttributeName, attribute: Attribute<D>) -> Result<(), AttributesError> {
        self.insert_instance(name, attribute, D::TemporalInstanceId::default())
    }

    /// Adds an instance and the `instanceId` a request names for it to the evolution of `name`.
    pub(crate) fn insert_instance(&mut self, name: AttributeName, attribute: Attribute<D>, instance_id: D::TemporalInstanceId) -> Result<(), AttributesError> {
        check_kind(&name, &attribute)?;
        if let Some(evolution) = self.0.get_mut(&name) {
            return evolution.push(attribute, instance_id);
        }
        self.0.insert(name, Evolution::one(attribute, instance_id));
        Ok(())
    }

    /// Returns the evolution of `name`.
    #[must_use]
    pub fn get(&self, name: &AttributeName) -> Option<&Evolution<D>> {
        self.0.get(name)
    }

    /// Iterates over the attributes in document order.
    #[must_use]
    pub fn iter(&self) -> Iter<'_, AttributeName, Evolution<D>> {
        self.0.iter()
    }

    /// Returns the attribute names in document order.
    pub fn names(&self) -> impl Iterator<Item = &AttributeName> {
        self.0.keys()
    }

    /// Returns the number of attribute names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no attributes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Maps every attribute name and every instance; an error is placed under the name as written.
#[cfg(feature = "linked-data")]
impl<D: MapDirection + EntityDirection> MapNames for TemporalAttributes<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        map_attribute_names(
            self.0,
            mapping,
            |evolution| {
                let instances = evolution.instances.map_names(mapping)?;
                Ok(Evolution {
                    instances,
                    instance_ids: evolution.instance_ids,
                })
            },
            |target, evolution| evolution.instances.iter().try_for_each(|instance| check_kind(target, instance)),
        )
        .map(Self)
    }
}

impl TemporalAttributes<Input> {
    /// Adds an instance that names its `instanceId` to the evolution of `name`, after the instances already there: the
    /// temporal representation a client sends to Create or Update Temporal Evolution or Add Attributes may identify its
    /// instances (see [`EntityDirection::TemporalInstanceId`]).
    ///
    /// # Errors
    ///
    /// Returns what [`insert`](Self::insert) returns, and [`AttributesError::InstanceIdRepeated`] when another
    /// instance of the evolution of `name` names `instance_id` (clause 4.5.7).
    pub fn insert_identified(&mut self, name: AttributeName, instance_id: InstanceId, attribute: Attribute<Input>) -> Result<(), AttributesError> {
        self.insert_instance(name, attribute, Some(instance_id))
    }

    /// Writes the attributes as one object, each evolution an array of instances, in normalized or concise
    /// representation: the body Add Attributes and Modify Attribute Instance take (clauses 5.6.12.3 and 5.6.14.3).
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(temporal_attributes_object(self, form)))
    }
}

impl TemporalAttributes<Output> {
    /// Writes the attributes as one object, each evolution an array of instances, with their output-only members.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(temporal_attributes_object(self, form)))
    }

    /// Adds the instances of `later`, the answer for the time range after this one (clause 6.3.10), dropping each one
    /// this evolution already holds: the same `instanceId` when both carry one, an equal instance otherwise.
    pub(crate) fn absorb(&mut self, later: TemporalAttributes<Output>) {
        for (name, evolution) in later.0 {
            match self.0.get_mut(&name) {
                Some(known) => {
                    for instance in evolution.instances {
                        if !known.instances.iter().any(|held| same_instance(held, &instance)) {
                            known.instances.push(instance);
                            known.instance_ids.push(());
                        }
                    }
                }
                None => {
                    self.0.insert(name, evolution);
                }
            }
        }
    }

    /// Keeps the `count` most recent instances of each attribute by the Temporal Property `property`, the ones
    /// `lastN` selects (clause 4.11), in document order.
    pub(crate) fn keep_last(&mut self, count: NonZeroU64, property: TemporalProperty) {
        for evolution in self.0.values_mut() {
            evolution.keep_last(count, property);
        }
    }

    /// Drops the output-only members of every instance, recording each NGSI-LD Null a request cannot carry, such as
    /// a deleted instance (clause 4.5.0), under the name of the attribute that holds it.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> TemporalAttributes<Input> {
        let attributes = self
            .0
            .into_iter()
            .map(|(name, evolution)| {
                let instances: Vec<_> = evolution
                    .instances
                    .into_iter()
                    .map(|instance| {
                        let mut found = Vec::new();
                        let input = instance.into_input_reporting(&mut found);
                        problems.extend(BuildError::new(found).within(&name));
                        input
                    })
                    .collect();
                // The instances a broker identified are sent back without their `instanceId`, which it chose.
                let instance_ids = vec![None; instances.len()];
                (name, Evolution { instances, instance_ids })
            })
            .collect();
        TemporalAttributes(attributes)
    }
}

impl Evolution<Output> {
    /// Keeps the `count` most recent instances by the Temporal Property `property`, in document order. Of instances
    /// recorded at the same time the earlier ones in document order are kept, and an instance without the property
    /// is less recent than any with it. At least one instance is kept, so the evolution stays non-empty.
    fn keep_last(&mut self, count: NonZeroU64, property: TemporalProperty) {
        let count = usize::try_from(count.get()).unwrap_or(usize::MAX);
        if self.instances.len() <= count {
            return;
        }
        let mut ranked: Vec<(usize, Option<DateTime>)> = self
            .instances
            .iter()
            .map(|instance| instance.meta().recorded_at(property))
            .enumerate()
            .collect();
        // Stable, so instances recorded at the same time keep their document order.
        ranked.sort_by_key(|&(_, recorded)| Reverse(recorded));
        let kept: HashSet<usize> = ranked.into_iter().take(count).map(|(index, _)| index).collect();
        let mut index = 0..;
        self.instances.retain(|_| index.next().is_some_and(|position| kept.contains(&position)));
        self.instance_ids.truncate(self.instances.len());
    }
}

/// Whether two instances of an evolution are the same one: their `instanceId`s when both carry one (clause 4.5.7),
/// their whole content otherwise.
fn same_instance(held: &Attribute<Output>, instance: &Attribute<Output>) -> bool {
    match (&held.meta().output.instance_id, &instance.meta().output.instance_id) {
        (Some(held_id), Some(instance_id)) => held_id == instance_id,
        (Some(_) | None, None) | (None, Some(_)) => held == instance,
    }
}

impl<'a, D: EntityDirection> IntoIterator for &'a TemporalAttributes<D> {
    type IntoIter = Iter<'a, AttributeName, Evolution<D>>;
    type Item = (&'a AttributeName, &'a Evolution<D>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
