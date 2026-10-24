//! The query string of a Query Entities request in typed form (NGSI-LD clauses 6.4.3.2 and 6.23.3.1), as a context
//! source reads a query a broker forwards to it (clause 5.7.2.4).

use crate::{
    http::{
        QueryParameter,
        operation::{
            AssemblyParameters,
            ParameterError,
            RepresentationSpelling,
            ResultsCount,
            SelectionParameters,
            SystemAttributes,
            received_parameters::{
                ReceivedQuery,
                selection::{ASSEMBLY, SELECTION},
            },
            representation_spelling::received::read_representation,
        },
    },
    model::{AttributeName, EntityId, Representation},
    query::{Join, Locality},
};
use std::num::NonZeroU32;

/// The linked entity retrieval of Table 6.4.3.2-1, which the POST binding carries in its `Query` body.
const LINKED: [QueryParameter; 3] = [QueryParameter::Join, QueryParameter::JoinLevel, QueryParameter::ContainedBy];

/// The parameters both bindings carry in the query string, since the `Query` body of clause 5.2.23 cannot: `format`
/// and `options` (clause 6.3.7), `geometryProperty` (clause 6.3.15), `limit` and `offset` (clause 6.3.10), `count`
/// (clause 6.3.13) and `local` (clause 6.3.18).
const SHARED: [QueryParameter; 7] = [
    QueryParameter::Format,
    QueryParameter::Options,
    QueryParameter::GeometryProperty,
    QueryParameter::Limit,
    QueryParameter::Offset,
    QueryParameter::Count,
    QueryParameter::Local,
];

/// The query string of a Query Entities request, read in typed form.
///
/// The request is `GET /entities` (NGSI-LD Table 6.4.3.2-1). A context source receives this query string when a broker
/// forwards a query to it (clause 5.7.2.4).
///
/// A POST query carries the same selection in its `Query` body instead, which
/// [`QueryDocument::parse`](crate::snapshot::QueryDocument::parse) reads, and the rest in a query string that
/// [`QueryEntitiesPostParameters`] reads.
///
/// Each parameter is read with the grammar of its query language. Rules that tie parameters together, such as a
/// type, `attrs`, `q` or geoquery being required unless the query is local, are the broker's to enforce and are
/// not checked, so that a context source sees exactly what it was sent; a parameter the table does not define is
/// reported.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::QueryEntitiesParameters;
///
/// let received =
///     QueryEntitiesParameters::parse("type=Vehicle&q=speed%3E50&format=keyValues&limit=20")?;
/// assert_eq!(
///     received
///         .selection
///         .entity_type
///         .map(|selection| selection.to_string())
///         .as_deref(),
///     Some("Vehicle")
/// );
/// assert_eq!(
///     received.selection.q.map(|q| q.to_string()).as_deref(),
///     Some("speed>50")
/// );
/// assert_eq!(received.limit, Some(20));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryEntitiesParameters {
    /// The entities matched and what of them the answer holds, `type` to `lang`.
    pub selection: SelectionParameters,
    /// `join` (clause 4.5.23).
    pub join: Option<Join>,
    /// `joinLevel`.
    pub join_level: Option<NonZeroU32>,
    /// `containedBy`.
    pub contained_by: Vec<EntityId>,
    /// The ordering, `splitEntities` and the entity map.
    pub assembly: AssemblyParameters,
    /// The representation `format` or `options` asks for, normalized when neither does (clause 6.3.7).
    pub representation: Representation,
    /// How the representation is written.
    pub spelling: RepresentationSpelling,
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `geometryProperty`, of a `GeoJSON` request (clause 6.3.15).
    pub geometry_property: Option<AttributeName>,
    /// `limit` (clause 6.3.10).
    pub limit: Option<u32>,
    /// `offset` (clause 6.3.10).
    pub offset: Option<u32>,
    /// `count` (clause 6.3.13).
    pub count: ResultsCount,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl QueryEntitiesParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.4.3.2-1 does not define, one given
    /// twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<QueryEntitiesParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&SELECTION, &ASSEMBLY, &LINKED, &SHARED])?;
        let QueryEntitiesPostParameters {
            representation,
            spelling,
            system_attributes,
            geometry_property,
            limit,
            offset,
            count,
            locality,
        } = QueryEntitiesPostParameters::read(&received)?;
        Ok(QueryEntitiesParameters {
            selection: SelectionParameters::read(&received)?,
            join: received.parsed(QueryParameter::Join, ParameterError::join)?,
            join_level: received.parsed(QueryParameter::JoinLevel, ParameterError::integer)?,
            contained_by: received.list(QueryParameter::ContainedBy, ParameterError::uri)?,
            assembly: AssemblyParameters::read(&received)?,
            representation,
            spelling,
            system_attributes,
            geometry_property,
            limit,
            offset,
            count,
            locality,
        })
    }
}

/// The query string of a `POST` Query Entities request, read in typed form.
///
/// The request is `POST /entityOperations/query` (NGSI-LD clause 6.23.3.1). A context source receives this query string
/// when a broker forwards a query to it as a batch query (the `queryBatch` operation of clause 4.20).
///
/// The selection travels in the `Query` body, which [`QueryDocument::parse`](crate::snapshot::QueryDocument::parse)
/// reads; the query string holds only what the body cannot, so a selection parameter such as `type` or `q` is reported
/// as undefined here.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::{
///     http::operation::{ParameterError, QueryEntitiesPostParameters, ResultsCount},
///     model::Representation,
/// };
///
/// let received = QueryEntitiesPostParameters::parse("format=concise&limit=10&count=true")?;
/// assert_eq!(received.representation, Representation::Concise);
/// assert_eq!(
///     (received.limit, received.count),
///     (Some(10), ResultsCount::Include)
/// );
/// assert!(matches!(
///     QueryEntitiesPostParameters::parse("type=Vehicle"),
///     Err(ParameterError::Unknown { .. })
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryEntitiesPostParameters {
    /// The representation `format` or `options` asks for, normalized when neither does (clause 6.3.7).
    pub representation: Representation,
    /// How the representation is written.
    pub spelling: RepresentationSpelling,
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `geometryProperty`, of a `GeoJSON` request (clause 6.3.15).
    pub geometry_property: Option<AttributeName>,
    /// `limit` (clause 6.3.10).
    pub limit: Option<u32>,
    /// `offset` (clause 6.3.10).
    pub offset: Option<u32>,
    /// `count` (clause 6.3.13).
    pub count: ResultsCount,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl QueryEntitiesPostParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one the query string of a POST query does not
    /// carry, one given twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<QueryEntitiesPostParameters, ParameterError> {
        QueryEntitiesPostParameters::read(&ReceivedQuery::read(query, &[&SHARED])?)
    }

    /// Reads the parameters both bindings carry in the query string.
    fn read(received: &ReceivedQuery<'_>) -> Result<QueryEntitiesPostParameters, ParameterError> {
        let representation = read_representation::<Representation>(received)?;
        Ok(QueryEntitiesPostParameters {
            representation: representation.representation,
            spelling: representation.spelling,
            system_attributes: representation.system_attributes,
            geometry_property: received.parsed(QueryParameter::GeometryProperty, ParameterError::name)?,
            limit: received.parsed(QueryParameter::Limit, ParameterError::integer)?,
            offset: received.parsed(QueryParameter::Offset, ParameterError::integer)?,
            count: received.results_count()?,
            locality: received.locality()?,
        })
    }
}
