//! The NGSI-LD Attribute Projection Language (NGSI-LD clause 4.21), written in the `pick` and `omit`
//! parameters.
//!
//! A projection is a list of attribute names, each optionally followed by a projection of the entity its
//! relationship targets (Linked Entity Retrieval, clause 4.5.23): `observation{temperature,humidity}`.
//! Clause 6.4.3.2 also lets `pick` and `omit` name the entity members `id`, `type` and `scope`, which are
//! modelled as [`EntityMember`]s rather than attributes. Clause 4.21 accepts `,` and `|` as the separator; the
//! parser reads both and the printer writes `,`.
//!
//! Clause 4.21 does not bound how deep linked projections nest, and a broker returns the `pick` and `omit` of its
//! subscriptions, so a projection may be thousands of levels deep. Every level passes through a [`Projection`],
//! whose parser, printer, mapping of names and `Clone`, `PartialEq`, `Hash`, `Debug` and `Drop` run each level on
//! a stack that grows with the depth.
//!
//! ```
//! use ngsi_ld::query::projection;
//!
//! let pick = projection::linked(
//!     "observation",
//!     projection::attributes(["temperature", "humidity"]),
//! )
//! .build()?;
//! assert_eq!(pick.to_string(), "observation{temperature,humidity}");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Projections
//!
//! [`Projection`], [`ProjectionTerm`], [`AttributeProjection`], [`EntityMember`], [`EntityMemberError`].
//!
//! # Start functions
//!
//! [`attributes`], [`linked`], [`member`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, each_in_text},
};
use crate::{
    model::{Term, nesting::grow},
    query::{
        Expected,
        ParseError,
        Problem,
        build_error::Deferred,
        grammar::{self, write_separated},
        projection::builders::ProjectionBuilder,
    },
};
#[cfg(feature = "linked-data")]
use std::mem;
use std::{fmt, str::FromStr};

pub mod builders;
mod parser;
mod walk;

/// A member of an entity that `pick` and `omit` can name besides its attributes (NGSI-LD clause 6.4.3.2).
///
/// The enum is exhaustive: clause 6.4.3.2 closes the list.
///
/// ```
/// use ngsi_ld::query::projection::EntityMember;
///
/// assert_eq!(EntityMember::Scope.to_string(), "scope");
/// assert_eq!("id".parse::<EntityMember>(), Ok(EntityMember::Id));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum EntityMember {
    /// The entity id.
    Id,
    /// The entity type.
    Type,
    /// The scopes of the entity.
    Scope,
}

/// A name is an entity member, which a projection holds as an [`EntityMember`], not as an attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("`{member}` is an entity member, not an attribute")]
pub struct EntityMemberError {
    /// The entity member the name spells.
    pub member: EntityMember,
}

/// An attribute in a projection, optionally with a projection of the entity its relationship targets.
///
/// ```
/// use ngsi_ld::{
///     model::Term,
///     query::projection::{AttributeProjection, EntityMember, EntityMemberError},
/// };
///
/// let observation: Term = "observation".parse()?;
/// let printed = AttributeProjection::new(observation).map(|attribute| attribute.to_string());
/// assert_eq!(printed, Ok("observation".to_owned()));
/// let id: Term = "id".parse()?;
/// assert_eq!(
///     AttributeProjection::new(id).err(),
///     Some(EntityMemberError {
///         member: EntityMember::Id
///     })
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeProjection {
    /// The attribute; never `id`, `type` or `scope`.
    name: Term,
    /// The projection of the linked entity, if any.
    linked: Option<Projection>,
}

impl AttributeProjection {
    /// The attribute `name`, without a linked projection.
    ///
    /// # Errors
    ///
    /// Returns an [`EntityMemberError`] when `name` is `id`, `type` or `scope`.
    pub fn new(name: Term) -> Result<AttributeProjection, EntityMemberError> {
        match name.as_ref().parse::<EntityMember>() {
            Ok(member) => Err(EntityMemberError { member }),
            Err(_) => Ok(Self { name, linked: None }),
        }
    }

    /// Projects the entity the relationship targets with `projection` (the `LinkedEntityTerm` production).
    #[must_use]
    pub fn with_linked(self, projection: Projection) -> Self {
        Self {
            linked: Some(projection),
            ..self
        }
    }

    /// Returns the attribute name.
    #[must_use]
    pub const fn name(&self) -> &Term {
        &self.name
    }

    /// Returns the projection of the linked entity, if any.
    #[must_use]
    pub const fn linked(&self) -> Option<&Projection> {
        self.linked.as_ref()
    }
}

// The linked projection is optional and braced, which a derive cannot express.
impl fmt::Display for AttributeProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)?;
        self.linked.as_ref().map_or(Ok(()), |linked| write!(f, "{{{linked}}}"))
    }
}

/// One term of a projection (the `ProjectionTerm` production of NGSI-LD clause 4.21, without its list).
///
/// The enum is exhaustive: a projection names entity members or attributes.
///
/// ```
/// use ngsi_ld::{
///     model::Term,
///     query::projection::{EntityMember, ProjectionTerm},
/// };
///
/// let name: Term = "type".parse()?;
/// assert_eq!(
///     ProjectionTerm::from(name),
///     ProjectionTerm::Member(EntityMember::Type)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum ProjectionTerm {
    /// `id`, `type` or `scope`.
    #[display("{_0}")]
    Member(EntityMember),
    /// An attribute.
    #[display("{_0}")]
    Attribute(AttributeProjection),
}

/// A name is the entity member it spells, or else an attribute.
impl From<Term> for ProjectionTerm {
    fn from(name: Term) -> Self {
        match name.as_ref().parse::<EntityMember>() {
            Ok(member) => Self::Member(member),
            Err(_) => Self::Attribute(AttributeProjection { name, linked: None }),
        }
    }
}

/// A projection (the `ProjectionTerm` production of NGSI-LD clause 4.21): a non-empty list of terms.
///
/// Every level of linked projections passes through a projection, so its `Clone`, `PartialEq`, `Hash`, `Debug` and
/// `Drop` are written to run each level on a stack that grows with the depth.
///
/// ```
/// use ngsi_ld::query::projection::Projection;
///
/// let projection: Projection = "id|temperature,humidity".parse()?;
/// assert_eq!(projection.to_string(), "id,temperature,humidity");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Eq)]
pub struct Projection(Vec<ProjectionTerm>);

impl Projection {
    /// Returns the terms, in order; there is at least one.
    #[must_use]
    pub fn terms(&self) -> &[ProjectionTerm] {
        &self.0
    }

    /// Whether the projection names the entity member `member`.
    #[must_use]
    pub fn names_member(&self, member: EntityMember) -> bool {
        self.0.contains(&ProjectionTerm::Member(member))
    }
}

// The terms are joined with `,`, which a derive cannot express; each level of linked projections is written on a
// stack that grows with the depth.
impl fmt::Display for Projection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        grow(|| write_separated(f, &self.0, ','))
    }
}

// No derive parses the projection grammar.
impl FromStr for Projection {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::projection, Expected::ProjectionTerm, Expected::Separator)
    }
}

/// Validates a name and reads it as a projection term.
fn parse_term(name: &str) -> Deferred<ProjectionTerm> {
    Deferred::from_result(name.parse::<Term>().map(ProjectionTerm::from).map_err(|source| Problem::InvalidName { source }))
}

/// Starts a projection with the attributes (or entity members) `names`.
///
/// ```
/// use ngsi_ld::query::projection;
///
/// let pick = projection::attributes(["temperature", "humidity"]).build()?;
/// assert_eq!(pick.to_string(), "temperature,humidity");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn attributes(names: impl IntoIterator<Item = impl AsRef<str>>) -> ProjectionBuilder {
    ProjectionBuilder(names.into_iter().map(|name| parse_term(name.as_ref())).collect())
}

/// Starts a projection with the relationship `name` and the projection `linked` of the entity it targets.
#[must_use]
pub fn linked(name: impl AsRef<str>, linked: ProjectionBuilder) -> ProjectionBuilder {
    ProjectionBuilder(Deferred::valid(Vec::new())).linked(name, linked)
}

/// Starts a projection with the entity member `member`.
#[must_use]
pub fn member(member: EntityMember) -> ProjectionBuilder {
    ProjectionBuilder(Deferred::valid(vec![ProjectionTerm::Member(member)]))
}

#[cfg(feature = "linked-data")]
impl MapNames for Projection {
    fn map_names<M: NameMapping>(mut self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        grow(|| each_in_text(mem::take(&mut self.0), mapping)).map(Self)
    }
}

/// `id`, `type` and `scope` are members of every entity, never terms of a context.
#[cfg(feature = "linked-data")]
impl MapNames for ProjectionTerm {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Member(member) => Ok(Self::Member(member)),
            Self::Attribute(attribute) => attribute.map_names(mapping).map(Self::Attribute),
        }
    }
}

#[cfg(feature = "linked-data")]
impl MapNames for AttributeProjection {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            name: self.name.map_names(mapping)?,
            linked: self.linked.map_names(mapping)?,
        })
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// A projection carries no `@context`: it is written under the context of the request that carries it.
    impl for Projection
);

#[cfg(test)]
mod tests;
