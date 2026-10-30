//! The spelling a value was given in, when it differs from the canonical form.

use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

/// The text a value was built from, kept only when it differs from the canonical form the value writes by default.
///
/// It is a spelling, not part of the value: every `Written` compares equal to every other, hashes to nothing and
/// orders as equal, so a value holding one is equal to the same value built from its canonical text.
#[derive(Debug, Clone, Default)]
pub(crate) struct Written(Option<Box<Spelling>>);

/// The kept text. It is boxed behind a thin pointer so that a value holding a `Written` grows by one word, since
/// the values that hold one travel in the `Err` variant of every reader.
#[derive(Debug, Clone)]
struct Spelling(Box<str>);

impl Written {
    /// Keeps nothing: the value writes its canonical form.
    pub(crate) const fn none() -> Self {
        Self(None)
    }

    /// Keeps `text` when it is not the canonical form `canonical`.
    pub(crate) fn of(text: &str, canonical: &str) -> Self {
        if text == canonical {
            Self(None)
        } else {
            Self(Some(Box::new(Spelling(text.into()))))
        }
    }

    /// Returns the kept text, when there is one.
    pub(crate) fn as_deref(&self) -> Option<&str> {
        self.0.as_deref().map(|spelling| &*spelling.0)
    }
}

// No derive can say "equal whatever it holds"; `derive_where` would have to be repeated on every holder.
impl PartialEq for Written {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

// Reflexive and transitive, since every pair is equal.
impl Eq for Written {}

// Consistent with `PartialEq`: equal values must hash alike, so nothing is hashed.
impl Hash for Written {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

// No derive orders every pair as equal.
impl PartialOrd for Written {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// No derive orders every pair as equal.
impl Ord for Written {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::{Ordering, Written};
    use std::collections::HashSet;

    #[test]
    fn a_written_spelling_is_kept_only_when_it_differs_from_the_canonical_text() {
        assert_eq!(Written::of("A", "a").as_deref(), Some("A"));
        assert_eq!(Written::of("a", "a").as_deref(), None);
    }

    #[test]
    fn every_written_spelling_is_equal_and_hashes_alike() {
        let kept = Written::of("A", "a");
        let nothing = Written::default();
        assert_eq!(kept, nothing);
        assert_eq!(kept.cmp(&nothing), Ordering::Equal);
        assert_eq!(HashSet::from([kept, nothing]).len(), 1);
    }
}
