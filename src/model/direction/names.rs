//! How each direction maps the names its attribute slots and output-only members hold.

use crate::model::{
    Change,
    Direction,
    DocumentError,
    Input,
    MemberChange,
    Output,
    Patch,
    name::mapping::{MapNames, NameMapping, member},
};
use std::fmt::Debug;

/// A direction whose documents a [`NameMapping`] rewrites.
pub(crate) trait MapDirection: Direction {
    /// Maps the names an optional member of an attribute holds, such as its `valueType` or `objectType`; a
    /// fragment's deletion of the member holds none.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error under `name`.
    fn map_member<T, M>(value: Self::Member<T>, name: &str, mapping: &M) -> Result<Self::Member<T>, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq,
        M: NameMapping;

    /// Maps the names an attribute slot holds.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error at the member that holds the name that failed.
    fn map_slot<M: NameMapping>(slot: Self::Slot, mapping: &M) -> Result<Self::Slot, DocumentError<M::Error>>;

    /// Maps the names the output-only members of one attribute kind hold.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error at the member that holds the name that failed.
    fn map_kind_output<K, M>(output: Self::KindOutput<K>, mapping: &M) -> Result<Self::KindOutput<K>, DocumentError<M::Error>>
    where
        K: MapNames + Debug + Clone + PartialEq + Eq + Default,
        M: NameMapping;
}

impl MapDirection for Input {
    fn map_member<T, M>(value: Option<T>, name: &str, mapping: &M) -> Result<Option<T>, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq,
        M: NameMapping,
    {
        member(value, name, mapping)
    }

    fn map_slot<M: NameMapping>(slot: Self::Slot, mapping: &M) -> Result<Self::Slot, DocumentError<M::Error>> {
        slot.map_names(mapping)
    }

    fn map_kind_output<K, M>(output: Self::KindOutput<K>, _mapping: &M) -> Result<Self::KindOutput<K>, DocumentError<M::Error>>
    where
        K: MapNames + Debug + Clone + PartialEq + Eq + Default,
        M: NameMapping,
    {
        Ok(output)
    }
}

impl MapDirection for Patch {
    fn map_member<T, M>(value: Option<MemberChange<T>>, name: &str, mapping: &M) -> Result<Option<MemberChange<T>>, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq,
        M: NameMapping,
    {
        member(value, name, mapping)
    }

    /// A deletion names a kind and a dataset id, never a name.
    fn map_slot<M: NameMapping>(slot: Self::Slot, mapping: &M) -> Result<Self::Slot, DocumentError<M::Error>> {
        match slot {
            Change::Set(attribute) => attribute.map_names(mapping).map(Change::Set),
            Change::Delete(deletion) => Ok(Change::Delete(deletion)),
        }
    }

    fn map_kind_output<K, M>(output: Self::KindOutput<K>, _mapping: &M) -> Result<Self::KindOutput<K>, DocumentError<M::Error>>
    where
        K: MapNames + Debug + Clone + PartialEq + Eq + Default,
        M: NameMapping,
    {
        Ok(output)
    }
}

impl MapDirection for Output {
    fn map_member<T, M>(value: Option<T>, name: &str, mapping: &M) -> Result<Option<T>, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq,
        M: NameMapping,
    {
        member(value, name, mapping)
    }

    fn map_slot<M: NameMapping>(slot: Self::Slot, mapping: &M) -> Result<Self::Slot, DocumentError<M::Error>> {
        slot.map_names(mapping)
    }

    fn map_kind_output<K, M>(output: Self::KindOutput<K>, mapping: &M) -> Result<Self::KindOutput<K>, DocumentError<M::Error>>
    where
        K: MapNames + Debug + Clone + PartialEq + Eq + Default,
        M: NameMapping,
    {
        output.map_names(mapping)
    }
}
