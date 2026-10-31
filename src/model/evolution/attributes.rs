//! The attributes of a temporal evolution (NGSI-LD clauses 4.5.7 and 4.5.8): per name, the sequence of its
//! instances over time.

#[cfg(feature = "linked-data")]
use crate::model::{
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
        DocumentError,
        EntityDirection,
        Input,
        InstanceId,
        Lossless,
        Output,
        RawJson,
        attribute::attributes::check_kind,
        evolution::{DeletedInstance, TemporalInstanceMembers, read::read_temporal_attributes, write::temporal_fragment_object},
        keyed_positions::KeyedPositions,
        reader::parse_json,
    },
    query::temporal_query::TemporalProperty,
    seal::Seal,
};
use indexmap::{IndexMap, map::Iter};
use jstrict::Value as Json;
use std::{cmp::Reverse, collections::HashSet, num::NonZeroU64, slice};

/// The temporal evolution of one attribute.
///
/// It holds the instances in document order, each recorded at a point in time (NGSI-LD clauses 4.5.7 and 4.5.8). A
/// request carries at least one; a response may carry none, when the attribute has no instance in the time range of a
/// page of a paginated answer (clause 6.3.10). All are of one kind, the instances of the same attribute; the same
/// dataset id may appear any number of times, unlike the instances of an entity's attribute (clause 4.5.5).
///
/// In a request each instance may name its `instanceId` and record a deletion (see
/// [`EntityDirection::TemporalMembers`]); in a response the instance holds them among its output-only members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evolution<D: EntityDirection> {
    /// The instances, in document order.
    instances: Vec<Attribute<D>>,
    /// The `instanceId` each instance of a request names and the deletion it records, at the instance's index.
    members: Vec<D::TemporalMembers>,
    /// The positions of the members that name an `instanceId`, by `instanceId`.
    instance_ids: KeyedPositions,
}

impl<D: EntityDirection> Evolution<D> {
    /// One instance.
    fn one(attribute: Attribute<D>, members: D::TemporalMembers) -> Self {
        Self {
            instances: vec![attribute],
            members: vec![members],
            instance_ids: KeyedPositions::default(),
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

    /// Iterates over the instances in order, each with the members a request gives it besides the attribute.
    pub(crate) fn with_members(&self) -> impl Iterator<Item = (&Attribute<D>, &D::TemporalMembers)> {
        self.instances.iter().zip(&self.members)
    }

    /// Returns the number of instances; zero only for the evolution of a response that holds no instance of the
    /// attribute in its time range.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether there is no instance, which only an evolution read from a response can be.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Adds an instance after the others, refusing one of another kind and one that names an `instanceId` another
    /// instance names: an `instanceId` identifies one instance (clauses 4.5.7 and 4.5.8). An instance that names
    /// none, and every instance of a response, whose identifier is `()`, is the default and never collides.
    fn push(&mut self, attribute: Attribute<D>, members: D::TemporalMembers) -> Result<(), AttributesError> {
        if let (Some(expected), Some(found)) = (self.kind(), attribute.written_kind(Seal::new()))
            && expected != found
        {
            return Err(AttributesError::KindChange { expected, found });
        }
        if let Some(instance_id) = D::instance_id(Seal::new(), &members)
            && self
                .instance_ids
                .record(&self.members, |held| D::instance_id(Seal::new(), held), instance_id)
                .is_err()
        {
            return Err(AttributesError::InstanceIdRepeated);
        }
        self.instances.push(attribute);
        self.members.push(members);
        Ok(())
    }
}

impl Evolution<Input> {
    /// Iterates over the `instanceId` each instance names, in the order of the instances; `None` for an instance
    /// that names none, whose identifier the broker chooses (clause 4.5.7).
    pub fn instance_ids(&self) -> impl Iterator<Item = Option<&InstanceId>> {
        self.members.iter().map(TemporalInstanceMembers::instance_id)
    }

    /// Iterates over the time each instance records a deletion at, in the order of the instances; `None` for an
    /// instance that records a value (clauses 4.5.7 and 4.5.8).
    pub fn deleted_ats(&self) -> impl Iterator<Item = Option<&DateTime>> {
        self.members.iter().map(TemporalInstanceMembers::deleted_at)
    }
}

impl<'a, D: EntityDirection> IntoIterator for &'a Evolution<D> {
    type IntoIter = slice::Iter<'a, Attribute<D>>;
    type Item = &'a Attribute<D>;

    fn into_iter(self) -> Self::IntoIter {
        self.instances.iter()
    }
}

/// The attributes of an entity's temporal evolution, by name, in document order.
///
/// Each name holds the [`Evolution`] of its instances (NGSI-LD clause 5.2.20).
///
/// Every way of adding an instance goes through [`TemporalAttributes::insert`], which keeps the instances of one
/// name of one kind and the names clause 5.2.4 reserves for geometries holding `GeoProperty` instances.
///
/// ```
/// use ngsi_ld::model::{AttributeName, Input, Lossless, Property, evolution::TemporalAttributes};
///
/// let speed: AttributeName = "speed".parse()?;
/// let mut attributes = TemporalAttributes::<Input>::new();
/// attributes.insert(
///     speed.clone(),
///     Property::builder(120)
///         .observed_at("2018-08-01T12:03:00Z".parse()?)
///         .build()?
///         .into(),
/// )?;
/// attributes.insert(
///     speed,
///     Property::builder(80)
///         .observed_at("2018-08-01T12:05:00Z".parse()?)
///         .build()?
///         .into(),
/// )?;
/// assert_eq!(
///     attributes.render(Lossless::Concise).to_string(),
///     concat!(
///         r#"{"speed":[{"value":120,"#,
///         r#""observedAt":"2018-08-01T12:03:00Z"},{"value":80,"#,
///         r#""observedAt":"2018-08-01T12:05:00Z"}]}"#
///     )
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
        self.insert_instance(name, attribute, D::TemporalMembers::default())
    }

    /// Adds an instance and the members a request gives it to the evolution of `name`.
    pub(crate) fn insert_instance(&mut self, name: AttributeName, attribute: Attribute<D>, members: D::TemporalMembers) -> Result<(), AttributesError> {
        check_kind(&name, &attribute)?;
        if let Some(evolution) = self.0.get_mut(&name) {
            return evolution.push(attribute, members);
        }
        self.0.insert(name, Evolution::one(attribute, members));
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
                    members: evolution.members,
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
    /// instances (see [`EntityDirection::TemporalMembers`]).
    ///
    /// # Errors
    ///
    /// Returns what [`insert`](Self::insert) returns, and [`AttributesError::InstanceIdRepeated`] when another
    /// instance of the evolution of `name` names `instance_id` (clause 4.5.7).
    pub fn insert_identified(&mut self, name: AttributeName, instance_id: InstanceId, attribute: Attribute<Input>) -> Result<(), AttributesError> {
        self.insert_instance(name, attribute, TemporalInstanceMembers::new(Some(instance_id), None))
    }

    /// Adds the record of a deleted instance to the evolution of `name`, after the instances already there (clauses
    /// 4.5.7 and 4.5.8).
    ///
    /// # Errors
    ///
    /// Returns what [`insert`](Self::insert) returns, and [`AttributesError::InstanceIdRepeated`] when another
    /// instance of the evolution of `name` names the `instanceId` the record names.
    pub fn insert_deleted(&mut self, name: AttributeName, deleted: DeletedInstance) -> Result<(), AttributesError> {
        let (attribute, members) = deleted.into_instance();
        self.insert_instance(name, attribute, members)
    }

    /// Writes the attributes as one object, each evolution an array of instances, in normalized or concise
    /// representation: the body Add Attributes and Modify Attribute Instance take (clauses 5.6.12.3 and 5.6.14.3).
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(temporal_fragment_object(None, None, self, form)))
    }

    /// Reads the attributes of a temporal evolution a client could send, in normalized or concise representation:
    /// the body of Add Attributes. Each member is the evolution of an attribute, an array of instances or the
    /// single instance clauses 4.5.7 and 4.5.8 allow when only one is present; an instance may name its `instanceId`
    /// and record a deletion as [`TemporalEntity::parse`](crate::model::evolution::TemporalEntity::parse) reads them.
    /// An `@context` is checked and not kept: the binding delivers it (clause 6.3.5).
    ///
    /// ```
    /// use ngsi_ld::model::{Input, Lossless, evolution::TemporalAttributes};
    ///
    /// let attributes = TemporalAttributes::<Input>::parse(
    ///     r#"{"speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"},{"value":80,"observedAt":"2018-08-01T12:05:00Z"}]}"#,
    ///     Lossless::Concise,
    /// )?;
    /// assert_eq!(attributes.len(), 1);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including an attribute given twice, an evolution with
    /// no instance, instances of one attribute of different kinds, any output-only member besides the `instanceId`
    /// and the `deletedAt` of a recorded deletion, and any instance holding the NGSI-LD Null without that
    /// `deletedAt` (clause 5.5.4).
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_temporal_attributes(parse_json(text)?, form)
    }
}

impl TemporalAttributes<Output> {
    /// Writes the attributes as one object, each evolution an array of instances, with their output-only members.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(temporal_fragment_object(None, None, self, form)))
    }

    /// Reads the attributes of a temporal evolution a broker returns, in normalized or concise representation, with
    /// the output-only members of their instances.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including an attribute given twice and instances of one
    /// attribute of different kinds. An attribute whose evolution is an empty array is read with no instance.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_temporal_attributes(parse_json(text)?, form)
    }

    /// Adds an attribute with no instance, the evolution of an attribute that has none in the time range of the
    /// answer (clause 6.3.10).
    pub(crate) fn insert_empty(&mut self, name: AttributeName) {
        self.0.insert(
            name,
            Evolution {
                instances: Vec::new(),
                members: Vec::new(),
                instance_ids: KeyedPositions::default(),
            },
        );
    }

    /// Adds the instances of `later`, the answer for the time range after this one (clause 6.3.10), dropping each one
    /// this evolution already holds: the same `instanceId` when both carry one, an equal instance otherwise.
    pub(crate) fn absorb(&mut self, later: TemporalAttributes<Output>) {
        for (name, evolution) in later.0 {
            match self.0.get_mut(&name) {
                Some(known) => {
                    // Built from what this evolution holds once a later instance names an `instanceId`.
                    let mut identified = KeyedPositions::default();
                    for instance in evolution.instances {
                        let held = match instance.meta().output.instance_id.as_ref() {
                            Some(instance_id) => identified.record(&known.instances, output_instance_id, instance_id).is_err(),
                            None => known.instances.iter().any(|held| held == &instance),
                        };
                        if !held {
                            known.instances.push(instance);
                            known.members.push(());
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

    /// Drops the output-only members of every instance, recording each NGSI-LD Null a request cannot carry under the
    /// name of the attribute that holds it. An instance that records a deletion with its `deletedAt` (clauses 4.5.7
    /// and 4.5.8) stays one, since a request can carry it. An attribute with no instance is left out: a request
    /// carries nothing by it.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> TemporalAttributes<Input> {
        let attributes = self
            .0
            .into_iter()
            .filter(|(_, evolution)| !evolution.instances.is_empty())
            .map(|(name, evolution)| {
                let (instances, members): (Vec<_>, Vec<_>) = evolution
                    .instances
                    .into_iter()
                    .map(|instance| {
                        let mut found = Vec::new();
                        let deleted_at = instance.meta().output.deleted_at.filter(|_| instance.is_deleted());
                        let input = match deleted_at {
                            Some(_) => instance.into_input_keeping_null(&mut found),
                            None => instance.into_input_reporting(&mut found),
                        };
                        problems.extend(BuildError::new(found).within(&name));
                        // The instances a broker identified are sent back without their `instanceId`, which it chose.
                        (input, TemporalInstanceMembers::new(None, deleted_at))
                    })
                    .unzip();
                (
                    name,
                    Evolution {
                        instances,
                        members,
                        instance_ids: KeyedPositions::default(),
                    },
                )
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
        self.members.truncate(self.instances.len());
    }
}

/// The `instanceId` a broker gave an instance of a response, by which two pages name the same instance (clause
/// 4.5.7).
const fn output_instance_id(instance: &Attribute<Output>) -> Option<&InstanceId> {
    instance.meta().output.instance_id.as_ref()
}

impl<'a, D: EntityDirection> IntoIterator for &'a TemporalAttributes<D> {
    type IntoIter = Iter<'a, AttributeName, Evolution<D>>;
    type Item = (&'a AttributeName, &'a Evolution<D>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
