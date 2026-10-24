//! The attributes of an entity or of an attribute: an ordered map from name to instances.
//!
//! Every way of adding an attribute goes through [`Attributes::insert`] or [`Attributes::set`], so the rules that
//! tie a name to what it may hold are kept here: the dataset-id rules of clause 4.5.5 and the names clause 5.2.4
//! reserves for a `GeoProperty`.

use crate::model::{AttributeKind, AttributeName, AttributeSlot, BuildError, BuildProblem, Direction, Input, Instances, InstancesError, Output, nesting::grow};
#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    direction::names::MapDirection,
    name::mapping::{MapNames, NameMapping, map_attribute_names},
};
use indexmap::{
    IndexMap,
    map::{IntoIter, Iter},
};
use std::{
    fmt::{self, Debug, Formatter},
    mem,
};

/// The attribute names the core `@context` defines as `GeoProperty` terms (NGSI-LD clause 5.2.4 and Table
/// 5.2.4-1): every attribute so named holds geometries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString)]
enum GeoAttributeName {
    /// `location`.
    #[strum(serialize = "location")]
    Location,
    /// `observationSpace`.
    #[strum(serialize = "observationSpace")]
    ObservationSpace,
    /// `operationSpace`.
    #[strum(serialize = "operationSpace")]
    OperationSpace,
}

/// Why an attribute cannot be added under a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum AttributesError {
    /// The instances break the dataset-id rules of clause 4.5.5.
    #[error("invalid instances")]
    Instances {
        /// Which rule is broken.
        #[from]
        source: InstancesError,
    },
    /// `location`, `observationSpace` or `operationSpace` holds something other than a `GeoProperty`
    /// (clause 5.2.4).
    #[error("the attribute must be a GeoProperty")]
    GeoPropertyRequired,
    /// An instance of a temporal evolution has another kind than the instances before it; the evolution of an
    /// attribute is the sequence of its own instances (clauses 4.5.7 and 4.5.8).
    #[error("an instance of a {expected} cannot be a {found}")]
    KindChange {
        /// The kind of the first instance.
        expected: AttributeKind,
        /// The kind of the instance refused.
        found: AttributeKind,
    },
    /// Two instances of a temporal evolution a request sends name the same `instanceId`, which identifies one
    /// instance (clauses 4.5.7 and 4.5.8).
    #[error("two instances of an evolution name the same instanceId")]
    InstanceIdRepeated,
    /// The temporal values of an attribute in simplified temporal representation are given twice; clause 4.5.9 holds
    /// all of them in one member.
    #[error("the temporal values of an attribute are given once")]
    ValuesNamedTwice,
    /// Two attributes take the same name: moving a document to another `@context` mapped two names that expand to
    /// the same Fully Qualified Name onto one term (clause 5.5.7).
    #[error("the name `{name}` is taken by another attribute")]
    NameTaken {
        /// The name both attributes would take.
        name: AttributeName,
    },
}

/// The attributes (or sub-attributes) of an element, by name, in document order (NGSI-LD clauses 4.5.1 and
/// 5.2.4). Each name holds one or more [`Instances`] of the direction's attribute slot.
///
/// [`Attributes::insert`] and [`Attributes::set`] keep the rules that tie a name to what it may hold: the dataset-id
/// rules of clause 4.5.5 and the names clause 5.2.4 reserves for a `GeoProperty`.
///
/// ```
/// use ngsi_ld::model::{Attribute, AttributeName, Attributes, Input, Property};
///
/// let speed: AttributeName = "speed".parse()?;
/// let mut attributes = Attributes::<Input>::new();
/// attributes.insert(
///     speed.clone(),
///     Attribute::from(Property::builder(80).build()?),
/// )?;
/// attributes.insert(
///     "brandName".parse()?,
///     Attribute::from(Property::builder("Mercedes").build()?),
/// )?;
/// assert_eq!(attributes.len(), 2);
/// assert!(attributes.get(&speed).is_some());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Eq)]
pub struct Attributes<D: Direction>(IndexMap<AttributeName, Instances<D::Slot>>);

// No derive: every level of nesting, sub-attributes and inline linked entities alike (clause 4.5.23.2), passes
// through this map, so the four walks a derive would generate run here on a stack that grows with the depth.
impl<D: Direction> Debug for Attributes<D> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        grow(|| formatter.debug_tuple("Attributes").field(&self.0).finish())
    }
}

// See the `Debug` implementation for why this is not derived.
impl<D: Direction> Clone for Attributes<D> {
    fn clone(&self) -> Self {
        grow(|| Self(self.0.clone()))
    }
}

// See the `Debug` implementation for why this is not derived.
impl<D: Direction> PartialEq for Attributes<D> {
    fn eq(&self, other: &Self) -> bool {
        grow(|| self.0 == other.0)
    }
}

// The derived drop of a deep document would recurse once per level on the current stack; see the `Debug`
// implementation.
impl<D: Direction> Drop for Attributes<D> {
    fn drop(&mut self) {
        let attributes = mem::take(&mut self.0);
        grow(move || drop(attributes));
    }
}

// Derived `Default` would require `D::Slot: Default`, which an attribute has no meaning for.
impl<D: Direction> Default for Attributes<D> {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

/// Whether `name` is reserved for geometries: every attribute so named is a `GeoProperty` (clause 5.2.4).
pub(crate) fn holds_geometries(name: &AttributeName) -> bool {
    name.as_ref().parse::<GeoAttributeName>().is_ok()
}

/// Checks clause 5.2.4: a slot under a name reserved for geometries writes a `GeoProperty`, or names no kind.
pub(crate) fn check_kind<S: AttributeSlot>(name: &AttributeName, slot: &S) -> Result<(), AttributesError> {
    if holds_geometries(name) && slot.written_kind().is_some_and(|kind| kind != AttributeKind::GeoProperty) {
        Err(AttributesError::GeoPropertyRequired)
    } else {
        Ok(())
    }
}

impl<D: Direction> Attributes<D> {
    /// No attributes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an instance under `name`, after any instances already there.
    ///
    /// # Errors
    ///
    /// Returns [`AttributesError`] when the instance breaks the dataset-id rules of clause 4.5.5, or when `name` is
    /// `location`, `observationSpace` or `operationSpace` and the instance is not a `GeoProperty` (clause 5.2.4).
    pub fn insert(&mut self, name: AttributeName, slot: D::Slot) -> Result<(), AttributesError> {
        check_kind(&name, &slot)?;
        if let Some(instances) = self.0.get_mut(&name) {
            return Ok(instances.push(slot)?);
        }
        self.0.insert(name, Instances::one(slot));
        Ok(())
    }

    /// Sets all instances of `name`, replacing any already there and keeping the position of the name.
    ///
    /// # Errors
    ///
    /// Returns [`AttributesError::GeoPropertyRequired`] when `name` is `location`, `observationSpace` or
    /// `operationSpace` and an instance is not a `GeoProperty` (clause 5.2.4).
    pub fn set(&mut self, name: AttributeName, instances: Instances<D::Slot>) -> Result<(), AttributesError> {
        instances.iter().try_for_each(|slot| check_kind(&name, slot))?;
        self.0.insert(name, instances);
        Ok(())
    }

    /// Returns the instances of `name`.
    #[must_use]
    pub fn get(&self, name: &AttributeName) -> Option<&Instances<D::Slot>> {
        self.0.get(name)
    }

    /// Returns the instances of `name` for modification.
    pub(crate) fn get_mut(&mut self, name: &AttributeName) -> Option<&mut Instances<D::Slot>> {
        self.0.get_mut(name)
    }

    /// Removes `name`, keeping the order of the others.
    pub fn remove(&mut self, name: &AttributeName) -> Option<Instances<D::Slot>> {
        self.0.shift_remove(name)
    }

    /// Iterates over the attributes in document order.
    #[must_use]
    pub fn iter(&self) -> Iter<'_, AttributeName, Instances<D::Slot>> {
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

    /// Converts every slot into another direction's, keeping names and order. The conversion keeps each slot's
    /// kind and dataset id, so the rules the map enforces still hold.
    pub(crate) fn convert<E: Direction>(mut self, mut convert: impl FnMut(&AttributeName, D::Slot) -> E::Slot) -> Attributes<E> {
        Attributes(
            mem::take(&mut self.0)
                .into_iter()
                .map(|(name, instances)| {
                    let converted = instances.map(|slot| convert(&name, slot));
                    (name, converted)
                })
                .collect(),
        )
    }
}

impl Attributes<Output> {
    /// Drops the output-only members of every attribute, recording each NGSI-LD Null a request cannot carry under
    /// the name of the attribute that holds it.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> Attributes<Input> {
        self.convert::<Input>(|name, attribute| {
            let mut found = Vec::new();
            let input = attribute.into_input_reporting(&mut found);
            problems.extend(BuildError::new(found).within(name));
            input
        })
    }
}

/// Maps every attribute name and every attribute; an error is placed under the name as written.
#[cfg(feature = "linked-data")]
impl<D: MapDirection> MapNames for Attributes<D> {
    fn map_names<M: NameMapping>(mut self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        let attributes = mem::take(&mut self.0);
        grow(|| {
            map_attribute_names(
                attributes,
                mapping,
                |instances| instances.try_map(|slot| D::map_slot(slot, mapping)),
                |target, instances| instances.iter().try_for_each(|slot| check_kind(target, slot)),
            )
            .map(Self)
        })
    }
}

impl<'a, D: Direction> IntoIterator for &'a Attributes<D> {
    type IntoIter = Iter<'a, AttributeName, Instances<D::Slot>>;
    type Item = (&'a AttributeName, &'a Instances<D::Slot>);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// The map is taken out first: a type with a `Drop` implementation cannot be moved out of.
impl<D: Direction> IntoIterator for Attributes<D> {
    type IntoIter = IntoIter<AttributeName, Instances<D::Slot>>;
    type Item = (AttributeName, Instances<D::Slot>);

    fn into_iter(mut self) -> Self::IntoIter {
        mem::take(&mut self.0).into_iter()
    }
}
