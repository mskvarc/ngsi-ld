//! The parameters Query Entities and Query Temporal Evolution share, read in typed form (Tables 6.4.3.2-1 and
//! 6.18.3.2-1): the entities a query matches and what of them the answer holds, and how the answer is assembled.

use crate::{
    http::{
        QueryParameter,
        operation::{EntityMapLifetime, ParameterError, entity_ordering, geoquery, received_parameters::ReceivedQuery},
    },
    model::{AttributeName, DatasetId, EntityId, NameError},
    query::{
        EntityMapReturn,
        EntitySplitting,
        IdPattern,
        SelectorType,
        geoquery::GeoQuery,
        language_filter::LanguageFilter,
        ordering::EntityOrdering,
        projection::Projection,
        q::Query,
        scope_query::ScopeQuery,
    },
};
use std::str::FromStr;

/// The parameters of the entity selection.
pub(crate) const SELECTION: [QueryParameter; 17] = [
    QueryParameter::Type,
    QueryParameter::Id,
    QueryParameter::IdPattern,
    QueryParameter::Attrs,
    QueryParameter::Q,
    QueryParameter::Georel,
    QueryParameter::Geometry,
    QueryParameter::Coordinates,
    QueryParameter::Geoproperty,
    QueryParameter::ScopeQ,
    QueryParameter::Csf,
    QueryParameter::Pick,
    QueryParameter::Omit,
    QueryParameter::ExpandValues,
    QueryParameter::JsonKeys,
    QueryParameter::DatasetId,
    QueryParameter::Lang,
];

/// The parameters of the assembly of the answer.
pub(crate) const ASSEMBLY: [QueryParameter; 7] = [
    QueryParameter::OrderBy,
    QueryParameter::Collation,
    QueryParameter::OrderFrom,
    QueryParameter::OrderGeometry,
    QueryParameter::SplitEntities,
    QueryParameter::EntityMap,
    QueryParameter::EntityMapLifetime,
];

/// The entities a received query matches and what of them the answer holds.
///
/// The parameters are `type`, `id`, `idPattern`, `attrs`, `q`, the geoquery, `scopeQ`, `csf`, `pick`, `omit`,
/// `expandValues`, `jsonKeys`, `datasetId` and `lang` (NGSI-LD Tables 6.4.3.2-1 and 6.18.3.2-1).
///
/// Each parameter is read with its grammar. Of the rules that tie parameters together, only those without which
/// the typed value cannot be built are checked: `georel`, `geometry` and `coordinates` make one geoquery, so each
/// needs the others (Table 6.4.3.2-1). A `geoproperty` without them is kept on its own, which a broker ignores.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::QueryEntitiesParameters;
///
/// let received = QueryEntitiesParameters::parse("type=*&idPattern=%5Ea%24%7C%5Eb%24")?;
/// assert_eq!(
///     received
///         .selection
///         .id_pattern
///         .map(|pattern| pattern.to_string())
///         .as_deref(),
///     Some("^a$|^b$")
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// `N` is what `attrs` names: attributes, or, in a temporal query, attributes and the entity's `scope` (clause 4.18).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SelectionParameters<N = AttributeName> {
    /// `type`: the entity types (clause 4.17), or every type (`*`).
    pub entity_type: Option<SelectorType>,
    /// `id`.
    pub ids: Vec<EntityId>,
    /// `idPattern`.
    pub id_pattern: Option<IdPattern>,
    /// `attrs`, deprecated.
    pub attrs: Vec<N>,
    /// `q` (clause 4.9).
    pub q: Option<Query>,
    /// `georel`, `geometry`, `coordinates` and `geoproperty` (clause 4.10).
    pub geo_q: Option<GeoQuery>,
    /// A `geoproperty` given without a geoquery, which Table 6.4.3.2-1 says is ignored; beside a geoquery it is the
    /// geoquery's.
    pub lone_geoproperty: Option<AttributeName>,
    /// `scopeQ` (clause 4.19).
    pub scope_q: Option<ScopeQuery>,
    /// `csf` (clause 4.9).
    pub csf: Option<Query>,
    /// `pick` (clause 4.21).
    pub pick: Option<Projection>,
    /// `omit` (clause 4.21).
    pub omit: Option<Projection>,
    /// `expandValues`.
    pub expand_values: Vec<AttributeName>,
    /// `jsonKeys`.
    pub json_keys: Vec<AttributeName>,
    /// `datasetId` (clause 4.5.5).
    pub dataset_ids: Vec<DatasetId>,
    /// `lang` (clause 4.15).
    pub lang: Option<LanguageFilter>,
}

impl<N: FromStr<Err = NameError>> SelectionParameters<N> {
    /// Reads the parameters of the selection.
    pub(crate) fn read(received: &ReceivedQuery<'_>) -> Result<SelectionParameters<N>, ParameterError> {
        let (geo_q, lone_geoproperty) = read_geo_query(received)?;
        Ok(SelectionParameters {
            entity_type: received.parsed(QueryParameter::Type, ParameterError::syntax)?,
            ids: received.list(QueryParameter::Id, ParameterError::uri)?,
            id_pattern: received.parsed(QueryParameter::IdPattern, ParameterError::id_pattern)?,
            attrs: received.list(QueryParameter::Attrs, ParameterError::name)?,
            q: received.parsed(QueryParameter::Q, ParameterError::syntax)?,
            geo_q,
            lone_geoproperty,
            scope_q: received.parsed(QueryParameter::ScopeQ, ParameterError::syntax)?,
            csf: received.parsed(QueryParameter::Csf, ParameterError::syntax)?,
            pick: received.parsed(QueryParameter::Pick, ParameterError::syntax)?,
            omit: received.parsed(QueryParameter::Omit, ParameterError::syntax)?,
            expand_values: received.list(QueryParameter::ExpandValues, ParameterError::name)?,
            json_keys: received.list(QueryParameter::JsonKeys, ParameterError::name)?,
            dataset_ids: received.list(QueryParameter::DatasetId, ParameterError::uri)?,
            lang: received.parsed(QueryParameter::Lang, ParameterError::syntax)?,
        })
    }
}

/// Reads the geoquery (`georel`, `geometry`, `coordinates` and `geoproperty`, clause 4.10), and a `geoproperty`
/// given without one, which Tables 6.4.3.2-1 and 6.4.3.3-1 say is ignored: beside a geoquery it is the geoquery's.
pub(crate) fn read_geo_query(received: &ReceivedQuery<'_>) -> Result<(Option<GeoQuery>, Option<AttributeName>), ParameterError> {
    let geo_q = geoquery::read_query(received)?;
    let lone_geoproperty = match geo_q {
        Some(_) => None,
        None => received.parsed(QueryParameter::Geoproperty, ParameterError::name)?,
    };
    Ok((geo_q, lone_geoproperty))
}

/// How a received query asks for its answer to be assembled.
///
/// The parameters are the ordering (NGSI-LD clause 4.23), `splitEntities`, and the entity map (`entityMap`,
/// `entityMapLifetime`, clause 4.5.25).
///
/// `collation`, `orderFrom` and `orderGeometry` complete an `orderBy` (clause 4.23.3), so each is refused without
/// it; every other parameter is read on its own.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::{http::operation::QueryEntitiesParameters, query::EntitySplitting};
///
/// let received = QueryEntitiesParameters::parse("type=Vehicle&splitEntities=false")?;
/// assert_eq!(received.assembly.split_entities, EntitySplitting::Whole);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AssemblyParameters {
    /// `orderBy`, `collation`, `orderFrom` and `orderGeometry` (clause 4.23).
    pub ordering: Option<EntityOrdering>,
    /// `splitEntities`.
    pub split_entities: EntitySplitting,
    /// `entityMap` (clause 4.5.25).
    pub entity_map: EntityMapReturn,
    /// `entityMapLifetime`.
    pub entity_map_lifetime: Option<EntityMapLifetime>,
}

impl AssemblyParameters {
    /// Reads the parameters of the assembly.
    pub(crate) fn read(received: &ReceivedQuery<'_>) -> Result<AssemblyParameters, ParameterError> {
        Ok(AssemblyParameters {
            ordering: entity_ordering::read_query(received)?,
            split_entities: received
                .flag(QueryParameter::SplitEntities)?
                .map_or(EntitySplitting::Unspecified, EntitySplitting::from_flag),
            entity_map: received
                .flag(QueryParameter::EntityMap)?
                .map_or(EntityMapReturn::Unspecified, EntityMapReturn::from_flag),
            entity_map_lifetime: received.parsed(QueryParameter::EntityMapLifetime, ParameterError::lifetime)?,
        })
    }
}
