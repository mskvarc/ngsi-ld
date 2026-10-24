//! The query string of a Retrieve Entity request in typed form (NGSI-LD clause 6.5.3.1, Table 6.5.3.1-1), as a
//! context source reads a retrieval a broker forwards to it (clause 5.7.1.4).

use crate::{
    http::{
        QueryParameter,
        operation::{
            EntityMapLifetime,
            ParameterError,
            RepresentationSpelling,
            SystemAttributes,
            received_parameters::ReceivedQuery,
            representation_spelling::received::read_representation,
        },
    },
    model::{AttributeName, DatasetId, EntityId, Representation},
    query::{EntityMapReturn, Join, Locality, language_filter::LanguageFilter, projection::Projection, type_selection::TypeSelection},
};
use std::num::NonZeroU32;

/// The parameters Retrieve Entity accepts: Table 6.5.3.1-1, with `format` and `options` (clause 6.3.7) and `local`
/// (clause 6.3.18).
const ACCEPTED: [QueryParameter; 15] = [
    QueryParameter::Type,
    QueryParameter::Attrs,
    QueryParameter::Pick,
    QueryParameter::Omit,
    QueryParameter::DatasetId,
    QueryParameter::Lang,
    QueryParameter::Join,
    QueryParameter::JoinLevel,
    QueryParameter::ContainedBy,
    QueryParameter::EntityMap,
    QueryParameter::EntityMapLifetime,
    QueryParameter::GeometryProperty,
    QueryParameter::Format,
    QueryParameter::Options,
    QueryParameter::Local,
];

/// The query string of a Retrieve Entity request, read in typed form.
///
/// The request is `GET /entities/{entityId}` (NGSI-LD Table 6.5.3.1-1). A context source receives this query string
/// when a broker forwards a retrieval to it (clause 5.7.1.4).
///
/// Each parameter is read with the grammar of its query language; rules that tie parameters together are the
/// broker's to enforce and are not checked, and a parameter the table does not define is reported. Unlike Query
/// Entities, Table 6.5.3.1-1 does not allow `type=*`.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::{http::operation::RetrieveEntityParameters, query::Join};
///
/// let received =
///     RetrieveEntityParameters::parse("pick=name,isParked%7Bname%7D&join=inline&joinLevel=2")?;
/// assert_eq!(
///     received.pick.map(|pick| pick.to_string()).as_deref(),
///     Some("name,isParked{name}")
/// );
/// assert_eq!(received.join, Some(Join::Inline));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetrieveEntityParameters {
    /// `type` (clause 4.17).
    pub entity_type: Option<TypeSelection>,
    /// `attrs`, deprecated.
    pub attrs: Vec<AttributeName>,
    /// `pick` (clause 4.21).
    pub pick: Option<Projection>,
    /// `omit` (clause 4.21).
    pub omit: Option<Projection>,
    /// `datasetId` (clause 4.5.5).
    pub dataset_ids: Vec<DatasetId>,
    /// `lang` (clause 4.15).
    pub lang: Option<LanguageFilter>,
    /// `join` (clause 4.5.23).
    pub join: Option<Join>,
    /// `joinLevel`.
    pub join_level: Option<NonZeroU32>,
    /// `containedBy`.
    pub contained_by: Vec<EntityId>,
    /// `entityMap` (clause 4.5.25).
    pub entity_map: EntityMapReturn,
    /// `entityMapLifetime`.
    pub entity_map_lifetime: Option<EntityMapLifetime>,
    /// The representation `format` or `options` asks for, normalized when neither does (clause 6.3.7).
    pub representation: Representation,
    /// How the representation is written.
    pub spelling: RepresentationSpelling,
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `geometryProperty`, of a `GeoJSON` request (clause 6.3.15).
    pub geometry_property: Option<AttributeName>,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl RetrieveEntityParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.5.3.1-1 does not define, one given
    /// twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<RetrieveEntityParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&ACCEPTED])?;
        let representation = read_representation::<Representation>(&received)?;
        Ok(RetrieveEntityParameters {
            entity_type: received.parsed(QueryParameter::Type, ParameterError::syntax)?,
            attrs: received.list(QueryParameter::Attrs, ParameterError::name)?,
            pick: received.parsed(QueryParameter::Pick, ParameterError::syntax)?,
            omit: received.parsed(QueryParameter::Omit, ParameterError::syntax)?,
            dataset_ids: received.list(QueryParameter::DatasetId, ParameterError::uri)?,
            lang: received.parsed(QueryParameter::Lang, ParameterError::syntax)?,
            join: received.parsed(QueryParameter::Join, ParameterError::join)?,
            join_level: received.parsed(QueryParameter::JoinLevel, ParameterError::integer)?,
            contained_by: received.list(QueryParameter::ContainedBy, ParameterError::uri)?,
            entity_map: received
                .flag(QueryParameter::EntityMap)?
                .map_or(EntityMapReturn::Unspecified, EntityMapReturn::from_flag),
            entity_map_lifetime: received.parsed(QueryParameter::EntityMapLifetime, ParameterError::lifetime)?,
            representation: representation.representation,
            spelling: representation.spelling,
            system_attributes: representation.system_attributes,
            geometry_property: received.parsed(QueryParameter::GeometryProperty, ParameterError::name)?,
            locality: received.locality()?,
        })
    }
}
