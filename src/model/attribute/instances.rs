//! The instances of one attribute, told apart by `datasetId` (NGSI-LD clause 4.5.5).

use crate::{model::DatasetId, seal::Seal};
use std::slice::{Iter, IterMut};

/// The private supertrait that keeps `InstanceKey` implemented by this crate alone.
pub(crate) mod sealed {
    /// Seals [`InstanceKey`](super::InstanceKey): an attribute and a change to one are the only instances the
    /// specification has.
    pub trait Sealed {}
}

/// Something that is one instance of an attribute, and so has a dataset id or is the default instance.
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

/// Why a set of instances breaks clause 4.5.5.
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

/// The instances of one attribute: at least one, at most one default instance, no dataset id twice
/// (NGSI-LD clause 4.5.5), and a deletion of the whole attribute only on its own. One instance is written as an
/// object, several as an array of objects (clauses 4.5.2.2 and 4.5.3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instances<T>(Vec<T>);

impl<T: InstanceKey> Instances<T> {
    /// One instance.
    #[must_use]
    pub fn one(instance: T) -> Self {
        Self(vec![instance])
    }

    /// Several instances, in order.
    ///
    /// # Errors
    ///
    /// Returns [`InstancesError`] when the list is empty or breaks the dataset-id rules.
    pub fn try_from_vec(instances: Vec<T>) -> Result<Self, InstancesError> {
        let mut checked = Self(Vec::with_capacity(instances.len()));
        for instance in instances {
            checked.push(instance)?;
        }
        if checked.0.is_empty() { Err(InstancesError::Empty) } else { Ok(checked) }
    }

    /// Adds an instance.
    ///
    /// # Errors
    ///
    /// Returns [`InstancesError`] when the instance is a second default instance, repeats a dataset id, or would
    /// stand beside a deletion of the whole attribute.
    pub fn push(&mut self, instance: T) -> Result<(), InstancesError> {
        if !self.0.is_empty()
            && (instance.excludes_other_instances(Seal::new()) || self.0.iter().any(|existing| existing.excludes_other_instances(Seal::new())))
        {
            return Err(InstancesError::BareDeletionAmongInstances);
        }
        if instance.is_default_instance() {
            if self.0.iter().any(InstanceKey::is_default_instance) {
                return Err(InstancesError::DefaultTwice);
            }
        } else if let Some(dataset_id) = instance.dataset_id()
            && self.0.iter().any(|existing| existing.dataset_id() == Some(dataset_id))
        {
            return Err(InstancesError::DuplicateDatasetId {
                // Clone: the error owns the dataset id, which the refused instance only lends; the instance is dropped.
                dataset_id: dataset_id.clone(),
            });
        }
        self.0.push(instance);
        Ok(())
    }

    /// Iterates over the instances in order.
    pub fn iter(&self) -> Iter<'_, T> {
        self.0.iter()
    }

    /// Iterates over the instances for modification; crate code never changes a dataset id through it. It is not
    /// named `iter_mut`, since no `&mut Instances` iterates outside the crate.
    pub(crate) fn instances_mut(&mut self) -> IterMut<'_, T> {
        self.0.iter_mut()
    }

    /// Returns the number of instances; never zero.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: an attribute has at least one instance.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the first instance.
    #[must_use]
    pub fn first(&self) -> &T {
        &self.0[0]
    }

    /// Returns the default instance, if any.
    #[must_use]
    pub fn default_instance(&self) -> Option<&T> {
        self.0.iter().find(|instance| instance.is_default_instance())
    }

    /// Converts every instance, keeping the order.
    pub(crate) fn map<U>(self, map: impl FnMut(T) -> U) -> Instances<U> {
        Instances(self.0.into_iter().map(map).collect())
    }

    /// Converts every instance with `map`, keeping the order, until one fails. The conversion keeps each
    /// instance's dataset id, so the rules of clause 4.5.5 still hold.
    #[cfg(feature = "linked-data")]
    pub(crate) fn try_map<U, E>(self, map: impl FnMut(T) -> Result<U, E>) -> Result<Instances<U>, E> {
        self.0.into_iter().map(map).collect::<Result<_, _>>().map(Instances)
    }

    /// Returns the instances.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<'a, T> IntoIterator for &'a Instances<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
