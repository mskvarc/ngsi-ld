//! Members the specification writes as one value or an array of values: a Relationship's `object`, an entity's
//! `type` and `scope`, a `VocabProperty`'s `vocab`, a relationship's `objectType`.

use crate::model::{BuildProblem, ProblemKind, ReservedMember, build_problem::Recorded};
#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping},
};
use jstrict::Value as Json;
use std::slice::{self, Iter};

/// A list that must hold at least one value is empty.
///
/// An empty array carries no value at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("the list must hold at least one value")]
pub struct EmptyList;

/// One value, written bare, or one or more values, written as a JSON array.
///
/// The two spellings are distinct values so a document is written back the way it was read: clause 4.5.1
/// writes one entity type as a string and several as an array. An array always holds at least one value, so an
/// empty list of types, scopes or objects cannot be represented.
///
/// ```
/// use ngsi_ld::model::{EmptyList, OneOrMany};
///
/// let types = OneOrMany::<&str>::try_from(vec!["Building", "House"])?;
/// assert_eq!(types.as_slice(), ["Building", "House"]);
/// assert!(OneOrMany::one("Building").iter().eq(["Building"].iter()));
/// assert_eq!(OneOrMany::<&str>::try_from(Vec::new()), Err(EmptyList));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OneOrMany<T>(Spelling<T>);

/// How the values are written; the array is never empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Spelling<T> {
    /// One value, written bare.
    One(T),
    /// One or more values, written as an array.
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    /// One value, written bare.
    #[must_use]
    pub const fn one(value: T) -> Self {
        Self(Spelling::One(value))
    }

    /// Whether the values are written as an array, even an array of one.
    #[must_use]
    pub const fn is_many(&self) -> bool {
        matches!(self.0, Spelling::Many(_))
    }

    /// Returns the value when it is written bare.
    #[must_use]
    pub const fn as_one(&self) -> Option<&T> {
        match &self.0 {
            Spelling::One(one) => Some(one),
            Spelling::Many(_) => None,
        }
    }

    /// Iterates over the values.
    pub fn iter(&self) -> Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns the values as a slice; never empty.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        match &self.0 {
            Spelling::One(one) => slice::from_ref(one),
            Spelling::Many(many) => many,
        }
    }

    /// Writes the value bare or as an array.
    pub(crate) fn to_json(&self, item: impl Fn(&T) -> Json) -> Json {
        match &self.0 {
            Spelling::One(one) => item(one),
            Spelling::Many(many) => Json::Array(many.iter().map(item).collect()),
        }
    }
}

// No derive builds the private spelling from the bare value.
impl<T> From<T> for OneOrMany<T> {
    fn from(one: T) -> Self {
        Self::one(one)
    }
}

/// An array of values; an empty one is refused.
impl<T> TryFrom<Vec<T>> for OneOrMany<T> {
    type Error = EmptyList;

    fn try_from(many: Vec<T>) -> Result<Self, Self::Error> {
        if many.is_empty() { Err(EmptyList) } else { Ok(Self(Spelling::Many(many))) }
    }
}

impl<'a, T> IntoIterator for &'a OneOrMany<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// One value or several given to a builder.
///
/// It is a value, a [`OneOrMany`], or a `Vec` whose emptiness is reported when the document is built, together with
/// every other problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneOrManyDraft<T>(Result<OneOrMany<T>, EmptyList>);

impl<T> OneOrManyDraft<T> {
    /// Returns the values of `member`, or records that there are none.
    pub(crate) fn resolve(self, member: ReservedMember, problems: &mut Vec<BuildProblem>) -> Option<OneOrMany<T>> {
        self.0
            .map_err(|EmptyList| problems.push(BuildProblem::top(ProblemKind::EmptyList { member })))
            .ok()
    }

    /// Returns the values of an optional `member`, `None` when it is absent, or records that it is empty.
    pub(crate) fn resolve_optional(draft: Option<Self>, member: ReservedMember, problems: &mut Vec<BuildProblem>) -> Result<Option<OneOrMany<T>>, Recorded> {
        match draft {
            Some(draft) => draft.resolve(member, problems).map(Some).ok_or(Recorded),
            None => Ok(None),
        }
    }
}

// No derive: the value is wrapped twice, into the spelling and into the draft's `Result`.
impl<T> From<T> for OneOrManyDraft<T> {
    fn from(one: T) -> Self {
        Self(Ok(OneOrMany::one(one)))
    }
}

// No derive: the draft keeps the conversion's outcome, so an empty list is reported by `build()`.
impl<T> From<Vec<T>> for OneOrManyDraft<T> {
    fn from(many: Vec<T>) -> Self {
        Self(OneOrMany::try_from(many))
    }
}

// No derive: the value is wrapped into the draft's `Result`.
impl<T> From<OneOrMany<T>> for OneOrManyDraft<T> {
    fn from(values: OneOrMany<T>) -> Self {
        Self(Ok(values))
    }
}

/// Reads one value or an array of them, refusing an empty array, which carries no value at all.
pub(crate) fn one_or_many_from_json<T, E>(json: Json, item: impl FnMut(Json) -> Result<T, E>, empty: E) -> Result<OneOrMany<T>, E> {
    let mut item = item;
    match json {
        Json::Array(items) if items.is_empty() => Err(empty),
        Json::Array(items) => items
            .into_iter()
            .map(item)
            .collect::<Result<Vec<_>, _>>()
            .map(|many| OneOrMany(Spelling::Many(many))),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_) => item(json).map(OneOrMany::one),
    }
}

/// The values of an array, each placed under its index; a bare value at the top of the member.
#[cfg(feature = "linked-data")]
impl<T: MapNames> MapNames for OneOrMany<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self.0 {
            Spelling::One(one) => one.map_names(mapping).map(Self::one),
            Spelling::Many(many) => many.map_names(mapping).map(|many| Self(Spelling::Many(many))),
        }
    }
}
