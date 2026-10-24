//! The builder of a [`Projection`].
//!
//! [`ProjectionBuilder`].

use crate::{
    model::BuildError,
    query::{
        Problem,
        build_error::Deferred,
        projection::{EntityMember, Projection, ProjectionTerm, parse_term},
    },
};
use std::mem;

/// A projection under construction, started by [`attributes`](crate::query::projection::attributes), [`linked`](crate::query::projection::linked) or
/// [`member`](crate::query::projection::member).
///
/// Names are validated on the spot, without panicking; [`build`](Self::build) reports every invalid one.
///
/// ```
/// use ngsi_ld::query::projection::{self, EntityMember};
///
/// let pick = projection::member(EntityMember::Id)
///     .attribute("temperature")
///     .build()?;
/// assert_eq!(pick.to_string(), "id,temperature");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionBuilder(pub(super) Deferred<Vec<ProjectionTerm>>);

impl ProjectionBuilder {
    /// Adds the attribute (or entity member) `name`.
    #[must_use]
    pub fn attribute(self, name: impl AsRef<str>) -> Self {
        self.push(parse_term(name.as_ref()))
    }

    /// Adds the entity member `member`.
    #[must_use]
    pub fn member(self, member: EntityMember) -> Self {
        self.push(Deferred::valid(ProjectionTerm::Member(member)))
    }

    /// Adds the relationship `name` with the projection `linked` of the entity it targets.
    #[must_use]
    pub fn linked(self, name: impl AsRef<str>, linked: ProjectionBuilder) -> Self {
        let relationship = parse_term(name.as_ref()).and_then(|term| match term {
            ProjectionTerm::Attribute(attribute) => Ok(attribute),
            ProjectionTerm::Member(member) => Err(Problem::LinkedEntityMember { member }),
        });
        self.push(
            relationship
                .zip(linked.into_projection())
                .map(|(attribute, linked)| ProjectionTerm::Attribute(attribute.with_linked(linked))),
        )
    }

    /// Appends a term.
    fn push(self, term: Deferred<ProjectionTerm>) -> Self {
        Self(self.0.zip(term).map(|(mut terms, term)| {
            terms.push(term);
            terms
        }))
    }

    /// The projection, or the problems preventing it.
    fn into_projection(self) -> Deferred<Projection> {
        self.0.and_then(|terms| {
            if terms.is_empty() {
                Err(Problem::EmptyProjection)
            } else {
                Ok(Projection(terms))
            }
        })
    }

    /// Returns the projection.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid name, every entity member given a linked projection, and
    /// every empty projection.
    pub fn build(self) -> Result<Projection, BuildError<Problem>> {
        self.into_projection().finish()
    }
}

/// A projection already built.
impl From<Projection> for ProjectionBuilder {
    fn from(mut projection: Projection) -> Self {
        Self(Deferred::valid(mem::take(&mut projection.0)))
    }
}

impl TryFrom<ProjectionBuilder> for Projection {
    type Error = BuildError<Problem>;

    fn try_from(builder: ProjectionBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
