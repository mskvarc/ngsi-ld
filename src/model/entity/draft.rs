//! The inputs the entity builder takes for `id`, `type` and `scope`: a validated value, or a string that is
//! validated when the entity is built, so every problem is reported together.

use crate::model::{
    BuildProblem,
    EntityId,
    EntityTypeName,
    NameError,
    OneOrMany,
    ProblemKind,
    ReservedMember,
    Scope,
    ScopeError,
    UriError,
    build_problem::Recorded,
    draft::Draft,
};
use iri_rs::Iri;
use std::str::FromStr;

/// Why a list given to a builder is unusable: an item is invalid, or the list is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListProblem<E> {
    /// An item is invalid.
    Invalid(E),
    /// The list is empty.
    Empty,
}

/// Parses every string of a list into a [`OneOrMany`] array.
fn parse_list<T: FromStr, const N: usize>(items: [&str; N]) -> Result<OneOrMany<T>, ListProblem<T::Err>> {
    let parsed = items.into_iter().map(str::parse).collect::<Result<Vec<T>, _>>().map_err(ListProblem::Invalid)?;
    OneOrMany::try_from(parsed).map_err(|_| ListProblem::Empty)
}

/// Returns the list, or records why it is unusable.
fn resolve_list<T, E>(
    list: Result<OneOrMany<T>, ListProblem<E>>,
    member: ReservedMember,
    invalid: impl FnOnce(E) -> ProblemKind,
    problems: &mut Vec<BuildProblem>,
) -> Option<OneOrMany<T>> {
    list.map_err(|problem| {
        let kind = match problem {
            ListProblem::Invalid(source) => invalid(source),
            ListProblem::Empty => ProblemKind::EmptyList { member },
        };
        problems.push(BuildProblem::top(kind));
    })
    .ok()
}

draft!(
    /// An entity id given to a builder: an [`EntityId`], a compile-time IRI literal, or a string.
    IdDraft(EntityId, UriError)
);

impl IdDraft {
    /// Returns the id, or records why it is invalid.
    pub(crate) fn resolve(self, problems: &mut Vec<BuildProblem>) -> Option<EntityId> {
        self.into_result()
            .map_err(|source| problems.push(BuildProblem::top(ProblemKind::Identifier { source })))
            .ok()
    }
}

// No derive: the conversion wraps a compile-time IRI, valid by construction, into the draft's `Result`.
impl From<Iri<&str>> for IdDraft {
    fn from(iri: Iri<&str>) -> Self {
        Self::from(EntityId::from(iri))
    }
}

/// The entity type or types given to a builder: names, or strings validated when the entity is built.
#[derive(Debug, Clone)]
pub struct TypeDraft(Result<OneOrMany<EntityTypeName>, ListProblem<NameError>>);

impl TypeDraft {
    /// Returns the types, or records why they are unusable.
    pub(crate) fn resolve(self, problems: &mut Vec<BuildProblem>) -> Option<OneOrMany<EntityTypeName>> {
        resolve_list(self.0, ReservedMember::Type, |source| ProblemKind::Term { source }, problems)
    }
}

// No derive: each conversion below wraps a value, or the outcome of validating one, into the draft's `Result`.
impl From<EntityTypeName> for TypeDraft {
    fn from(name: EntityTypeName) -> Self {
        Self(Ok(OneOrMany::one(name)))
    }
}

// See the conversion above for why this is not derived.
impl From<OneOrMany<EntityTypeName>> for TypeDraft {
    fn from(names: OneOrMany<EntityTypeName>) -> Self {
        Self(Ok(names))
    }
}

/// Several types; an empty list is reported when the entity is built.
impl From<Vec<EntityTypeName>> for TypeDraft {
    fn from(names: Vec<EntityTypeName>) -> Self {
        Self(OneOrMany::try_from(names).map_err(|_| ListProblem::Empty))
    }
}

// See the conversions above for why this is not derived.
impl From<&str> for TypeDraft {
    fn from(name: &str) -> Self {
        Self(name.parse().map(OneOrMany::one).map_err(ListProblem::Invalid))
    }
}

/// Several type names as strings: the entity has every one of these types (clause 4.16).
impl<const N: usize> From<[&str; N]> for TypeDraft {
    fn from(names: [&str; N]) -> Self {
        Self(parse_list(names))
    }
}

/// The scope or scopes given to a builder: scopes, or strings validated when the entity is built.
#[derive(Debug, Clone)]
pub struct ScopeDraft(Result<OneOrMany<Scope>, ListProblem<ScopeError>>);

impl ScopeDraft {
    /// Returns the scopes of an optional `scope`, `None` when it is absent, or records why they are unusable.
    pub(crate) fn resolve_optional(draft: Option<Self>, problems: &mut Vec<BuildProblem>) -> Result<Option<OneOrMany<Scope>>, Recorded> {
        match draft {
            Some(Self(scopes)) => resolve_list(scopes, ReservedMember::Scope, |source| ProblemKind::Scope { source }, problems)
                .map(Some)
                .ok_or(Recorded),
            None => Ok(None),
        }
    }
}

// No derive: each conversion below wraps a value, or the outcome of validating one, into the draft's `Result`.
impl From<Scope> for ScopeDraft {
    fn from(scope: Scope) -> Self {
        Self(Ok(OneOrMany::one(scope)))
    }
}

// See the conversion above for why this is not derived.
impl From<OneOrMany<Scope>> for ScopeDraft {
    fn from(scopes: OneOrMany<Scope>) -> Self {
        Self(Ok(scopes))
    }
}

// See the conversion above for why this is not derived.
impl From<&str> for ScopeDraft {
    fn from(scope: &str) -> Self {
        Self(scope.parse().map(OneOrMany::one).map_err(ListProblem::Invalid))
    }
}

/// Several scopes as strings.
impl<const N: usize> From<[&str; N]> for ScopeDraft {
    fn from(scopes: [&str; N]) -> Self {
        Self(parse_list(scopes))
    }
}
