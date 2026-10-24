//! The query-language arguments of the client's calls, which a caller passes as builders or values: each is built
//! before the request is, and its problems are reported as a refused request.

use crate::{
    client::ClientError,
    http::operation::ArgumentError,
    model::{BuildError, draft::resolve_drafts},
    query::{
        EntitySelector,
        EntitySelectorDraft,
        Problem,
        SelectorType,
        builders::SelectorTypeBuilder,
        geoquery::{GeoQuery, builders::GeoQueryBuilder},
        language_filter::{LanguageFilter, builders::LanguageFilterBuilder},
        projection::{Projection, builders::ProjectionBuilder},
        q::{Query, builders::QueryBuilder},
        scope_query::{ScopeQuery, builders::ScopeQueryBuilder},
    },
};

/// Builds a query-language argument, reporting its problems as a refused request.
pub(super) fn build<B: TryInto<V, Error = BuildError<Problem>>, V, E>(builder: B) -> Result<V, ClientError<E>> {
    builder
        .try_into()
        .map_err(|source| ClientError::Request(ArgumentError::InvalidArgument { source }.into()))
}

/// Resolves the entity selectors given, reporting every invalid entity type together as a refused request.
pub(super) fn selectors_built<E>(drafts: Vec<EntitySelectorDraft>) -> Result<Vec<EntitySelector>, ClientError<E>> {
    let mut problems = Vec::new();
    let selectors = resolve_drafts(drafts, &mut problems, |_, source| Problem::InvalidEntityType { source });
    BuildError::finish(problems, selectors).map_err(|source| ClientError::Request(ArgumentError::InvalidArgument { source }.into()))
}

/// Builds an optional query-language argument, reporting its problems as a refused request.
pub(super) fn built<B: TryInto<V, Error = BuildError<Problem>>, V, E>(builder: Option<B>) -> Result<Option<V>, ClientError<E>> {
    builder.map(build).transpose()
}

/// The query-language arguments of a call that selects entities, as the caller gave them: the entity types, `q`,
/// the geoquery, `scopeQ`, `csf`, `pick`, `omit` and `lang`.
pub(super) struct SelectionBuilders {
    /// The entity types (clause 4.17), or every type.
    pub(super) entity_type: Option<SelectorTypeBuilder>,
    /// `q` (clause 4.9).
    pub(super) q: Option<QueryBuilder>,
    /// The geoquery (clause 4.10).
    pub(super) geo_q: Option<GeoQueryBuilder>,
    /// `scopeQ` (clause 4.19).
    pub(super) scope_q: Option<ScopeQueryBuilder>,
    /// `csf` (clause 4.9).
    pub(super) csf: Option<QueryBuilder>,
    /// `pick` (clause 4.21).
    pub(super) pick: Option<ProjectionBuilder>,
    /// `omit` (clause 4.21).
    pub(super) omit: Option<ProjectionBuilder>,
    /// `lang` (clause 4.15).
    pub(super) lang: Option<LanguageFilterBuilder>,
}

/// The query-language arguments of a call that selects entities, built.
pub(super) struct SelectionArguments {
    /// The entity types (clause 4.17), or every type.
    pub(super) entity_type: Option<SelectorType>,
    /// `q` (clause 4.9).
    pub(super) q: Option<Query>,
    /// The geoquery (clause 4.10).
    pub(super) geo_q: Option<GeoQuery>,
    /// `scopeQ` (clause 4.19).
    pub(super) scope_q: Option<ScopeQuery>,
    /// `csf` (clause 4.9).
    pub(super) csf: Option<Query>,
    /// `pick` (clause 4.21).
    pub(super) pick: Option<Projection>,
    /// `omit` (clause 4.21).
    pub(super) omit: Option<Projection>,
    /// `lang` (clause 4.15).
    pub(super) lang: Option<LanguageFilter>,
}

impl SelectionBuilders {
    /// Builds every argument, reporting the first that has problems as a refused request.
    pub(super) fn build<E>(self) -> Result<SelectionArguments, ClientError<E>> {
        Ok(SelectionArguments {
            entity_type: built(self.entity_type)?,
            q: built(self.q)?,
            geo_q: built(self.geo_q)?,
            scope_q: built(self.scope_q)?,
            csf: built(self.csf)?,
            pick: built(self.pick)?,
            omit: built(self.omit)?,
            lang: built(self.lang)?,
        })
    }
}
