//! The instances of one attribute, told apart by `datasetId` (NGSI-LD clause 4.5.5).

use crate::{
    model::{DatasetId, keyed_positions::KeyedPositions},
    seal::Seal,
};
use std::slice::{Iter, IterMut};

#[cfg(test)]
mod tests;

/// The private supertrait that keeps `InstanceKey` implemented by this crate alone.
pub(crate) mod sealed {
    /// Seals [`InstanceKey`](super::InstanceKey): an attribute and a change to one are the only instances the
    /// specification has.
    pub trait Sealed {}
}

/// Something that is one instance of an attribute.
///
/// As such, it has a dataset id or is the default instance.
///
/// The trait is sealed: an attribute and, in a fragment, a change to one are its only implementors.
pub trait InstanceKey: sealed::Sealed {
    /// The dataset id; `None` for an instance written without one.
    fn dataset_id(&self) -> Option<&DatasetId>;

    /// Whether this is the default instance: no dataset id, or the explicit `@none` (clause 4.5.5).
    fn is_default_instance(&self) -> bool {
        self.dataset_id().is_none_or(DatasetId::is_default_instance)
    }

    /// Whether this instance is the whole member and so cannot stand among other instances: a deletion written as
    /// the bare NGSI-LD Null (clauses 4.5.3.3 and 5.4), which no instance array can hold (clause 4.5.0).
    #[doc(hidden)]
    fn excludes_other_instances(&self, _: Seal) -> bool {
        false
    }
}

/// Why a set of instances of one attribute is invalid.
///
/// It breaks NGSI-LD clause 4.5.5.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum InstancesError {
    /// More than one instance is the default instance.
    #[error("an attribute can have only one default instance")]
    DefaultTwice,
    /// Two instances share a dataset id.
    #[error("two instances share the dataset id `{dataset_id}`")]
    DuplicateDatasetId {
        /// The shared dataset id.
        dataset_id: DatasetId,
    },
    /// There is no instance.
    #[error("an attribute needs at least one instance")]
    Empty,
    /// A deletion written as the bare NGSI-LD Null, which is the whole member, is given with other instances.
    #[error("a deletion of the whole attribute cannot stand among other instances")]
    BareDeletionAmongInstances,
}

/// The instances of one attribute.
///
/// There is at least one, at most one default instance, no dataset id twice (NGSI-LD clause 4.5.5), and a deletion of
/// the whole attribute only on its own. One instance is written as an object, several as an array of objects (clauses
/// 4.5.2.2 and 4.5.3.2).
///
/// Adding an instance takes constant time however many there are, so an attribute of many thousand instances is
/// built and read in time proportional to its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instances<T> {
    /// The instances, in order.
    items: Vec<T>,
    /// The position of the default instance, if there is one.
    default: Option<usize>,
    /// The positions of the named instances, by dataset id.
    named: KeyedPositions,
}

impl<T: InstanceKey> Instances<T> {
    /// One instance.
    #[must_use]
    pub fn one(instance: T) -> Self {
        Self {
            default: instance.is_default_instance().then_some(0),
            items: vec![instance],
            named: KeyedPositions::default(),
        }
    }

    /// Several instances, in order.
    ///
    /// # Errors
    ///
    /// Returns [`InstancesError`] when the list is empty or breaks the dataset-id rules.
    pub fn try_from_vec(instances: Vec<T>) -> Result<Self, InstancesError> {
        let mut checked = Self {
            items: Vec::with_capacity(instances.len()),
            default: None,
            named: KeyedPositions::default(),
        };
        for instance in instances {
            checked.push(instance)?;
        }
        if checked.items.is_empty() { Err(InstancesError::Empty) } else { Ok(checked) }
    }

    /// Adds an instance.
    ///
    /// # Errors
    ///
    /// Returns [`InstancesError`] when the instance is a second default instance, repeats a dataset id, or would
    /// stand beside a deletion of the whole attribute.
    pub fn push(&mut self, instance: T) -> Result<(), InstancesError> {
        // A deletion of the whole attribute is only ever added alone, so when there is one it is the first instance.
        if let Some(first) = self.items.first()
            && (instance.excludes_other_instances(Seal::new()) || first.excludes_other_instances(Seal::new()))
        {
            return Err(InstancesError::BareDeletionAmongInstances);
        }
        if instance.is_default_instance() {
            if self.default.is_some() {
                return Err(InstancesError::DefaultTwice);
            }
            self.default = Some(self.items.len());
        } else if let Some(dataset_id) = instance.dataset_id()
            && self.named.record(&self.items, named_dataset_id, dataset_id).is_err()
        {
            return Err(InstancesError::DuplicateDatasetId {
                // Clone: the error owns the dataset id, which the refused instance only lends; the instance is dropped.
                dataset_id: dataset_id.clone(),
            });
        }
        self.items.push(instance);
        Ok(())
    }

    /// Iterates over the instances in order.
    pub fn iter(&self) -> Iter<'_, T> {
        self.items.iter()
    }

    /// Iterates over the instances for modification; crate code never changes a dataset id through it. It is not
    /// named `iter_mut`, since no `&mut Instances` iterates outside the crate.
    pub(crate) fn instances_mut(&mut self) -> IterMut<'_, T> {
        self.items.iter_mut()
    }

    /// Returns the number of instances; never zero.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    /// Always `false`: an attribute has at least one instance.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the first instance.
    #[must_use]
    pub fn first(&self) -> &T {
        &self.items[0]
    }

    /// Returns the default instance, if any.
    #[must_use]
    pub fn default_instance(&self) -> Option<&T> {
        self.default.and_then(|at| self.items.get(at))
    }

    /// Converts every instance, keeping the order. The conversion keeps each instance's dataset id, so the rules of
    /// clause 4.5.5 still hold.
    pub(crate) fn map<U>(self, map: impl FnMut(T) -> U) -> Instances<U> {
        Instances {
            items: self.items.into_iter().map(map).collect(),
            default: self.default,
            named: self.named,
        }
    }

    /// Converts every instance with `map`, keeping the order, until one fails. The conversion keeps each
    /// instance's dataset id, so the rules of clause 4.5.5 still hold.
    #[cfg(feature = "linked-data")]
    pub(crate) fn try_map<U, E>(self, map: impl FnMut(T) -> Result<U, E>) -> Result<Instances<U>, E> {
        let Self { items, default, named } = self;
        items
            .into_iter()
            .map(map)
            .collect::<Result<_, _>>()
            .map(|items| Instances { items, default, named })
    }

    /// Returns the instances.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

/// The dataset id of a named instance; `None` for the default instance, which the named positions leave out.
fn named_dataset_id<T: InstanceKey>(instance: &T) -> Option<&DatasetId> {
    instance.dataset_id().filter(|_| !instance.is_default_instance())
}

impl<'a, T> IntoIterator for &'a Instances<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}
