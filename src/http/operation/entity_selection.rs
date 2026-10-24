//! What Query Entities and Query Temporal Evolution of Entities share (NGSI-LD clauses 5.7.2.3 and 5.7.4.3): the
//! entities a query matches and what of them the answer holds, and how the answer is assembled, written in either
//! binding, the query string of `GET` (Tables 6.4.3.2-1 and 6.18.3.2-1) or the `Query` body of `POST` (clause
//! 5.2.23).

use crate::{
    http::{
        QueryParameter,
        operation::{ArgumentError, EntityMapLifetime, EntityMapRequest, entity_ordering, entity_selection::targets::EntityTargets, geoquery},
        query_string::QueryString,
    },
    model::{AttributeName, DatasetId},
    query::{
        EntityMapReturn,
        EntitySplitting,
        Locality,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        ordering::EntityOrdering,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
    },
    snapshot::query_document::writer::{QueryMembers, attrs},
};
use std::fmt::Display;

pub(crate) mod targets;

/// Builds the [`EntitySelection`] of an operation whose fields carry the selection under the names the selection
/// gives them: `entity_selection!(operation)` reads `entity_type`, `ids` and `id_pattern` into the targets, and
/// `attrs`, `q`, `geo_q`, `scope_q`, `csf`, `pick`, `omit`, `expand_values`, `json_keys`, `dataset_ids` and
/// `lang` as they are. The operation is a place expression, read once per field.
macro_rules! entity_selection {
    ($operation:expr) => {
        $crate::http::operation::entity_selection::EntitySelection {
            targets: $crate::http::operation::entity_selection::targets::EntityTargets::parameters(
                $operation.entity_type,
                $operation.ids,
                $operation.id_pattern,
            ),
            attrs: $operation.attrs,
            q: $operation.q,
            geo_q: $operation.geo_q,
            scope_q: $operation.scope_q,
            csf: $operation.csf,
            pick: $operation.pick,
            omit: $operation.omit,
            expand_values: $operation.expand_values,
            json_keys: $operation.json_keys,
            dataset_ids: $operation.dataset_ids,
            lang: $operation.lang,
        }
    };
}

/// The entities a query matches and what of them the answer holds. `N` is what `attrs` names: attributes, or, in a
/// temporal query, attributes and the entity's `scope` (clause 4.18).
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntitySelection<'a, N = AttributeName> {
    /// `type`, `id` and `idPattern`, or the entity selectors of a `Query` body (clause 5.2.33).
    pub(crate) targets: EntityTargets<'a>,
    /// `attrs`, deprecated (Tables 6.4.3.2-1 and 6.18.3.2-1, Table 5.2.23-1).
    pub(crate) attrs: &'a [N],
    /// `q` (clause 4.9).
    pub(crate) q: Option<&'a Query>,
    /// The geoquery (clause 4.10).
    pub(crate) geo_q: Option<&'a GeoQuery>,
    /// `scopeQ` (clause 4.19).
    pub(crate) scope_q: Option<&'a ScopeQuery>,
    /// `csf` (clause 4.9).
    pub(crate) csf: Option<&'a Query>,
    /// `pick` (clause 4.21).
    pub(crate) pick: Option<&'a Projection>,
    /// `omit` (clause 4.21).
    pub(crate) omit: Option<&'a Projection>,
    /// `expandValues`.
    pub(crate) expand_values: &'a [AttributeName],
    /// `jsonKeys`.
    pub(crate) json_keys: &'a [AttributeName],
    /// `datasetId` (clause 4.5.5).
    pub(crate) dataset_ids: &'a [DatasetId],
    /// `lang` (clause 4.15).
    pub(crate) lang: Option<&'a LanguageFilter>,
}

impl<'a, N> EntitySelection<'a, N> {
    /// The selection of `targets` alone: no `attrs`, query, projection or other parameter, for an operation that
    /// takes some of them and fills them in.
    pub(crate) const fn of(targets: EntityTargets<'a>) -> EntitySelection<'a, N> {
        EntitySelection {
            targets,
            attrs: &[],
            q: None,
            geo_q: None,
            scope_q: None,
            csf: None,
            pick: None,
            omit: None,
            expand_values: &[],
            json_keys: &[],
            dataset_ids: &[],
            lang: None,
        }
    }
}

impl<'a, N: Display> EntitySelection<'a, N> {
    /// Refuses a query that names no entity type, `attrs`, `q` or geoquery and is not local: Table 6.4.3.2-1
    /// requires one of type, `attrs`, `q` or `georel` unless the query is local, Table 6.18.3.2-1 a type unless
    /// `attrs` is present or the query is local, and clauses 5.7.2.4 and 5.7.3.4 accept a restriction made
    /// implicitly by a query or a geoquery; a scope query alone does not count. A query on a snapshot is local by
    /// itself (clause 5.5.15), whatever its `locality`, and so is a query of every type (`*`, Tables 6.4.3.2-1 and
    /// 5.2.33-1), which refuses `local=false`.
    pub(crate) fn check_bounded(&self, locality: Locality, reach: Reach) -> Result<(), ArgumentError> {
        self.targets.check_locality(locality)?;
        let local = locality == Locality::Local || reach == Reach::Snapshot || self.targets.selects_every_type();
        if !self.targets.names_type() && self.attrs.is_empty() && self.q.is_none() && self.geo_q.is_none() && !local {
            return Err(ArgumentError::UnboundedQuery);
        }
        Ok(())
    }

    /// Adds the parameters to a query string, from `type` to `lang`. A selection the query string cannot carry
    /// is refused by [`EntityTargets::check_query_string`] first.
    pub(crate) fn push_query(&self, query: &mut QueryString) {
        self.targets.push_query(query);
        query.push_list(QueryParameter::Attrs, self.attrs);
        query.push_some(QueryParameter::Q, self.q);
        if let Some(geo_q) = self.geo_q {
            geoquery::push_query(query, geo_q);
        }
        query.push_scope_query(QueryParameter::ScopeQ, self.scope_q);
        query.push_some(QueryParameter::Csf, self.csf);
        query.push_projection(QueryParameter::Pick, self.pick);
        query.push_projection(QueryParameter::Omit, self.omit);
        query.push_list(QueryParameter::ExpandValues, self.expand_values);
        query.push_list(QueryParameter::JsonKeys, self.json_keys);
        query.push_list(QueryParameter::DatasetId, self.dataset_ids);
        query.push_language_filter(QueryParameter::Lang, self.lang);
    }

    /// The members of a `Query` body (Table 5.2.23-1) the selection fills, from `entities` to `lang`.
    ///
    /// # Errors
    ///
    /// Returns [`ArgumentError::SelectorWithoutType`] when ids or an id pattern are given without a type, which
    /// the `EntitySelector` of clause 5.2.33 cannot express.
    pub(crate) fn query_members(&self) -> Result<QueryMembers<'a>, ArgumentError> {
        Ok(QueryMembers {
            entities: self.targets.entities()?,
            q: self.q,
            geo_q: self.geo_q,
            scope_q: self.scope_q,
            attrs: attrs(self.attrs),
            pick: self.pick,
            omit: self.omit,
            csf: self.csf,
            // An empty list of the request is no `datasetId` member.
            dataset_ids: (!self.dataset_ids.is_empty()).then_some(self.dataset_ids),
            expand_values: self.expand_values,
            json_keys: self.json_keys,
            lang: self.lang,
            ..QueryMembers::default()
        })
    }
}

/// Where an operation runs, as far as it decides whether a query needs a restriction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    /// Where its `locality` says.
    Stated,
    /// On a snapshot, which is local by itself (clause 5.5.15) and needs no `local=true`.
    Snapshot,
}

/// How the answer of a query is assembled: the order of the entities, whether an entity may be split over several
/// context sources, and the entity map of a distributed query.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResultAssembly<'a> {
    /// `orderBy`, `collation`, `orderFrom`, `orderGeometry` (clause 4.23).
    pub(crate) ordering: Option<&'a EntityOrdering>,
    /// `splitEntities`.
    pub(crate) split_entities: EntitySplitting,
    /// `entityMap`, `entityMapLifetime`, `NGSILD-EntityMap` (clause 4.5.25).
    pub(crate) entity_map: Option<&'a EntityMapRequest>,
}

impl<'a> ResultAssembly<'a> {
    /// `splitEntities` alone, the one part of the assembly of a query's answer an entity map keeps: clauses
    /// 5.14.4.3 and 5.14.5.3 list no ordering, and the entity map flag and header are ignored.
    pub(crate) const fn splitting(split_entities: EntitySplitting) -> ResultAssembly<'a> {
        ResultAssembly {
            ordering: None,
            split_entities,
            entity_map: None,
        }
    }

    /// Adds the parameters to a query string: the ordering, `splitEntities`, `entityMap` and `entityMapLifetime`.
    pub(crate) fn push_query(&self, query: &mut QueryString) {
        if let Some(ordering) = self.ordering {
            entity_ordering::push_query(query, ordering);
        }
        query.push_some(QueryParameter::SplitEntities, self.split_entities.flag());
        if let Some(entity_map) = self.entity_map {
            entity_map.push_query(query);
        }
    }

    /// Fills the members of a `Query` body the assembly holds: `ordering`, `splitEntities`, `entityMap` and
    /// `entityMapLifetime`.
    pub(crate) fn query_members(&self, members: QueryMembers<'a>) -> QueryMembers<'a> {
        QueryMembers {
            ordering: self.ordering,
            split_entities: self.split_entities,
            entity_map: self.entity_map.map_or(EntityMapReturn::Unspecified, |entity_map| entity_map.returned),
            entity_map_lifetime: self.entity_map.and_then(|entity_map| entity_map.lifetime).map(EntityMapLifetime::duration),
            ..members
        }
    }
}
