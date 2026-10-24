//! Checks, for the tests of the languages that nest, that a tree nested far deeper than any stack holds is copied,
//! compared, hashed, debug-printed, mapped and dropped without overflowing the stack.

#[cfg(feature = "linked-data")]
use crate::{
    model::{
        AttributeName,
        AttributesError,
        EntityTypeName,
        Term,
        VocabTerm,
        name::mapping::{MapNames, NameMapping},
    },
    query::type_selection::SelectableTypeError,
};
use std::{
    fmt::Debug,
    hash::{DefaultHasher, Hash, Hasher},
    io::{self, Write},
};

/// The hash of `value`.
fn hash_of(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Copies, compares, hashes and debug-prints `value`, then drops the copy.
pub(crate) fn assert_walks<T: Clone + Debug + Eq + Hash>(value: &T) {
    let copy = value.clone();
    assert_eq!(copy, *value);
    assert_eq!(hash_of(&copy), hash_of(value));
    write!(io::sink(), "{copy:?}").unwrap();
}

/// Why the identity mapping fails; it never does, but a mapping's error must hold these two.
#[cfg(feature = "linked-data")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub(crate) enum IdentityError {
    /// Never produced.
    #[error(transparent)]
    Attributes(#[from] AttributesError),
    /// Never produced.
    #[error(transparent)]
    Types(#[from] SelectableTypeError),
}

/// A mapping that maps every name to itself.
#[cfg(feature = "linked-data")]
pub(crate) struct IdentityMapping;

#[cfg(feature = "linked-data")]
impl NameMapping for IdentityMapping {
    type Error = IdentityError;

    fn entity_type(&self, name: &EntityTypeName) -> Result<EntityTypeName, Self::Error> {
        Ok(name.clone())
    }

    fn attribute(&self, name: &AttributeName) -> Result<AttributeName, Self::Error> {
        Ok(name.clone())
    }

    fn term(&self, name: &Term) -> Result<Term, Self::Error> {
        Ok(name.clone())
    }

    fn vocab(&self, name: &VocabTerm) -> Result<VocabTerm, Self::Error> {
        Ok(name.clone())
    }
}

/// Maps the names of `value` to themselves, and checks that nothing changed.
#[cfg(feature = "linked-data")]
pub(crate) fn assert_maps_to_itself<T: MapNames + Clone + Debug + Eq>(value: &T) {
    assert_eq!(value.clone().map_names(&IdentityMapping).unwrap(), *value);
}
