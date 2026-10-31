//! Where the elements of a sequence that carry a key stand, by key, so an element about to be added is checked for
//! a key one of them already holds without reading them all: the dataset ids of an attribute's instances (clause
//! 4.5.5), and the `instanceId`s of a temporal evolution (clause 4.5.7).

use hashbrown::{HashTable, hash_table::Entry};
use std::hash::{BuildHasher, Hash, RandomState};

/// An element of the sequence already holds the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyHeld;

/// The positions of the keyed elements of one sequence.
///
/// The positions are built from the sequence the first time an element is checked against a non-empty one, so a
/// sequence of one element, the common case, holds no table. They follow from the sequence, so two sets of
/// positions are equal whatever they hold (see the `PartialEq` impl below).
#[derive(Debug, Clone, Default)]
pub(crate) struct KeyedPositions(Option<Table>);

/// The positions, once built.
#[derive(Debug, Clone)]
struct Table {
    /// The position of every keyed element, found by the hash of its key.
    positions: HashTable<usize>,
    /// Hashes keys; seeded per sequence, so keys a broker chose cannot be made to collide.
    hasher: RandomState,
}

impl KeyedPositions {
    /// Records `key` for the element about to be added after `existing`, unless an element of `existing` holds it.
    ///
    /// `key_of` reads an element's key, `None` for an element that carries none. Every keyed element of `existing`
    /// was recorded here as it was added, or `existing` has not been checked against before.
    pub(crate) fn record<E, K: Hash + Eq + ?Sized>(&mut self, existing: &[E], key_of: impl Fn(&E) -> Option<&K>, key: &K) -> Result<(), KeyHeld> {
        if existing.is_empty() {
            return Ok(());
        }
        let Table { positions, hasher } = self.0.get_or_insert_with(|| Table::of(existing, &key_of));
        let hash_at = |at: &usize| hasher.hash_one(existing.get(*at).and_then(&key_of));
        match positions.entry(hasher.hash_one(Some(key)), |at| existing.get(*at).and_then(&key_of) == Some(key), hash_at) {
            Entry::Occupied(_) => Err(KeyHeld),
            Entry::Vacant(vacant) => {
                vacant.insert(existing.len());
                Ok(())
            }
        }
    }
}

impl Table {
    /// The positions of the keyed elements of `existing`, whose keys are distinct.
    fn of<E, K: Hash + Eq + ?Sized>(existing: &[E], key_of: impl Fn(&E) -> Option<&K>) -> Self {
        let hasher = RandomState::new();
        let mut positions = HashTable::new();
        for (at, key) in existing.iter().enumerate().filter_map(|(at, element)| key_of(element).map(|key| (at, key))) {
            positions.insert_unique(hasher.hash_one(Some(key)), at, |held: &usize| {
                hasher.hash_one(existing.get(*held).and_then(&key_of))
            });
        }
        Self { positions, hasher }
    }
}

// No derive can say "equal whatever it holds": the positions follow from the sequence beside them.
impl PartialEq for KeyedPositions {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

// Reflexive and transitive, since every pair is equal.
impl Eq for KeyedPositions {}
