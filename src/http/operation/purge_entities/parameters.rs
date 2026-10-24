//! The query string of a Purge Entities request in typed form (NGSI-LD clause 6.4.3.3, Table 6.4.3.3-1), as a
//! context source reads a purge a broker forwards to it (clause 5.6.21.4).

use crate::{
    http::{
        QueryParameter,
        operation::{
            ParameterError,
            received_parameters::{ReceivedQuery, selection::read_geo_query},
        },
    },
    model::{AttributeName, EntityId},
    query::{IdPattern, Locality, SelectorType, geoquery::GeoQuery, q::Query, scope_query::ScopeQuery},
};

/// The parameters Purge Entities accepts: Table 6.4.3.3-1, with `local` (clause 6.3.18).
const ACCEPTED: [QueryParameter; 14] = [
    QueryParameter::Coordinates,
    QueryParameter::Csf,
    QueryParameter::Drop,
    QueryParameter::Geometry,
    QueryParameter::GeometryProperty,
    QueryParameter::Geoproperty,
    QueryParameter::Georel,
    QueryParameter::Id,
    QueryParameter::IdPattern,
    QueryParameter::Keep,
    QueryParameter::Q,
    QueryParameter::ScopeQ,
    QueryParameter::Type,
    QueryParameter::Local,
];

/// The query string of a Purge Entities request (`DELETE /entities`, Table 6.4.3.3-1), read in typed form.
///
/// Each parameter is read with its grammar; of the rules that tie parameters together, only those without which the
/// typed value cannot be built are checked, as for a received query: `georel`, `geometry` and `coordinates` make one
/// geoquery. The entity map the purge uses travels in the `NGSILD-EntityMap` header (Table 6.4.3.3-2), not here.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::PurgeEntitiesParameters;
///
/// let received = PurgeEntitiesParameters::parse("type=Vehicle&drop=speed,brandName")?;
/// assert_eq!(
///     received
///         .drop
///         .iter()
///         .map(AsRef::as_ref)
///         .collect::<Vec<&str>>(),
///     ["speed", "brandName"]
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PurgeEntitiesParameters {
    /// `type`: the entity types (clause 4.17), or every type (`*`).
    pub entity_type: Option<SelectorType>,
    /// `id`.
    pub ids: Vec<EntityId>,
    /// `idPattern`.
    pub id_pattern: Option<IdPattern>,
    /// `q` (clause 4.9).
    pub q: Option<Query>,
    /// `georel`, `geometry`, `coordinates` and `geoproperty` (clause 4.10).
    pub geo_q: Option<GeoQuery>,
    /// A `geoproperty` given without a geoquery, which Table 6.4.3.3-1 says is ignored.
    pub lone_geoproperty: Option<AttributeName>,
    /// `geometryProperty`.
    pub geometry_property: Option<AttributeName>,
    /// `scopeQ` (clause 4.19).
    pub scope_q: Option<ScopeQuery>,
    /// `csf` (clause 4.9).
    pub csf: Option<Query>,
    /// `drop`: the attributes removed from each matching entity.
    pub drop: Vec<AttributeName>,
    /// `keep`: the attributes kept on each matching entity.
    pub keep: Vec<AttributeName>,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl PurgeEntitiesParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.4.3.3-1 does not define, one given
    /// twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<PurgeEntitiesParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&ACCEPTED])?;
        let (geo_q, lone_geoproperty) = read_geo_query(&received)?;
        Ok(PurgeEntitiesParameters {
            entity_type: received.parsed(QueryParameter::Type, ParameterError::syntax)?,
            ids: received.list(QueryParameter::Id, ParameterError::uri)?,
            id_pattern: received.parsed(QueryParameter::IdPattern, ParameterError::id_pattern)?,
            q: received.parsed(QueryParameter::Q, ParameterError::syntax)?,
            geo_q,
            lone_geoproperty,
            geometry_property: received.parsed(QueryParameter::GeometryProperty, ParameterError::name)?,
            scope_q: received.parsed(QueryParameter::ScopeQ, ParameterError::syntax)?,
            csf: received.parsed(QueryParameter::Csf, ParameterError::syntax)?,
            drop: received.list(QueryParameter::Drop, ParameterError::name)?,
            keep: received.list(QueryParameter::Keep, ParameterError::name)?,
            locality: received.locality()?,
        })
    }
}
