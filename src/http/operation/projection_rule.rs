//! Which projections a request may send, given what the entities of its answer must hold of their `id` and `type`
//! (clause 4.21).

use crate::{
    http::operation::ArgumentError,
    model::{Complete, Completeness, Projected},
    query::{
        Join,
        projection::{EntityMember, Projection, ProjectionTerm},
    },
};
use std::iter;

/// The projection a request applies, as far as it decides whether the answer can be read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectionRequest<'a> {
    /// `pick`.
    pub(crate) pick: Option<&'a Projection>,
    /// `omit`.
    pub(crate) omit: Option<&'a Projection>,
    /// The linked entity retrieval, whose flattened answer lists the linked entities among the others.
    pub(crate) join: Option<Join>,
}

/// The linked projections a projection holds, at every depth (clause 4.21): the `{...}` of each linked term.
fn linked_projections(projection: &Projection) -> Vec<&Projection> {
    projection
        .terms()
        .iter()
        .filter_map(|term| match term {
            ProjectionTerm::Attribute(attribute) => attribute.linked(),
            ProjectionTerm::Member(_) => None,
        })
        .flat_map(|linked| iter::once(linked).chain(linked_projections(linked)))
        .collect()
}

impl ProjectionRequest<'_> {
    /// The first identifying member, `id` then `type`, the projection removes: `pick` keeps only the members it
    /// names, `omit` removes the ones it names (Tables 6.4.3.2-1 and 6.5.3.1-1).
    fn removed_member(self) -> Option<EntityMember> {
        [EntityMember::Id, EntityMember::Type]
            .into_iter()
            .find(|&member| self.pick.is_some_and(|pick| !pick.names_member(member)) || self.omit.is_some_and(|omit| omit.names_member(member)))
    }

    /// The first identifying member a linked projection removes from the linked entities a flattened join lists
    /// among the others (clause 4.5.23.3): a linked `pick` keeps only what it names of the linked entity, a linked
    /// `omit` removes what it names.
    fn linked_removed_member(self) -> Option<EntityMember> {
        match self.join {
            Some(Join::Flat) => [EntityMember::Id, EntityMember::Type].into_iter().find(|&member| {
                self.pick
                    .is_some_and(|pick| linked_projections(pick).iter().any(|linked| !linked.names_member(member)))
                    || self
                        .omit
                        .is_some_and(|omit| linked_projections(omit).iter().any(|linked| linked.names_member(member)))
            }),
            Some(Join::Inline | Join::None) | None => None,
        }
    }
}

/// What an entity of a completeness lets a projection remove.
pub(crate) trait ProjectionRule: Completeness {
    /// Refuses a projection whose answer an entity of this completeness cannot hold.
    fn check(projection: ProjectionRequest<'_>) -> Result<(), ArgumentError>;
}

impl ProjectionRule for Complete {
    /// A complete entity holds `id` and `type`, so no projection may remove them: neither from the entities the
    /// request names, nor from the linked entities a flattened join lists among them (clause 4.5.23.3), which a
    /// nested projection such as `isParked{name}` reduces (clause 4.21). The linked entities of an inline join are
    /// read as projected entities, so a nested projection is sent with it.
    fn check(projection: ProjectionRequest<'_>) -> Result<(), ArgumentError> {
        projection
            .removed_member()
            .or_else(|| projection.linked_removed_member())
            .map_or(Ok(()), |member| Err(ArgumentError::ProjectionRemovesMember { member }))
    }
}

impl ProjectionRule for Projected {
    /// A projected entity may lack `id` and `type`, and so may the linked entities of an inline join, which are
    /// read as projected entities whatever the request (clauses 4.21 and 4.5.23.2).
    fn check(_: ProjectionRequest<'_>) -> Result<(), ArgumentError> {
        Ok(())
    }
}
