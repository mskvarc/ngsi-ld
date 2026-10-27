//! The query string of Query Context Source Registrations in typed form (NGSI-LD clauses 5.10.2 and 6.8.3.2), as a
//! broker reads the request.

use crate::{
    http::{
        QueryParameter,
        operation::{ParameterError, ResultsCount, SelectionParameters, SystemAttributes, TemporalSelectionParameters, received_parameters::ReceivedQuery},
    },
    model::AttributeName,
};

/// The parameters of Table 6.8.3.2-1 that select registrations: the entity selection and the time bound.
const SELECTION: [QueryParameter; 18] = [
    QueryParameter::Type,
    QueryParameter::Id,
    QueryParameter::IdPattern,
    QueryParameter::Attrs,
    QueryParameter::Q,
    QueryParameter::Csf,
    QueryParameter::Georel,
    QueryParameter::Geometry,
    QueryParameter::Coordinates,
    QueryParameter::Geoproperty,
    QueryParameter::ScopeQ,
    QueryParameter::Pick,
    QueryParameter::Omit,
    QueryParameter::Lang,
    QueryParameter::Timerel,
    QueryParameter::TimeAt,
    QueryParameter::EndTimeAt,
    QueryParameter::Timeproperty,
];

/// The rest of Table 6.8.3.2-1 and the paging of clause 5.5.9: `geometryProperty`, `limit`, `offset`, `count`, and
/// `options` (clause 6.3.11).
const PAGING: [QueryParameter; 5] = [
    QueryParameter::GeometryProperty,
    QueryParameter::Limit,
    QueryParameter::Offset,
    QueryParameter::Count,
    QueryParameter::Options,
];

/// The query string of a Query Context Source Registrations request, read in typed form.
///
/// The request is `GET /csourceRegistrations` (NGSI-LD Table 6.8.3.2-1). A context source registry receives the
/// entity selection of a Query Entities request to match against the registrations' `information` (clause 5.10.2.4),
/// the time bound matched against their intervals, and the page.
///
/// Each parameter is read with its grammar. Rules that tie parameters together, such as one of `type`, `attrs`, `q`
/// or a geoquery being required (clause 5.10.2.4), are the broker's to enforce and are not checked, so that the
/// receiver sees exactly what it was sent. A parameter the table does not define is reported: `expandValues`,
/// `jsonKeys` and `datasetId` of an entity selection are, so those lists of [`selection`](Self::selection) are empty,
/// and so are `lastN` and the aggregation of a temporal selection (clause 5.10.2.4).
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{QueryRegistrationsParameters, ResultsCount};
///
/// let received =
///     QueryRegistrationsParameters::parse("type=Vehicle&attrs=speed&limit=10&count=true")?;
/// assert_eq!(
///     received
///         .selection
///         .entity_type
///         .map(|selection| selection.to_string())
///         .as_deref(),
///     Some("Vehicle")
/// );
/// assert_eq!(received.limit, Some(10));
/// assert_eq!(received.count, ResultsCount::Include);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryRegistrationsParameters {
    /// The entities the registrations are matched against, `type` to `lang`.
    pub selection: SelectionParameters,
    /// The time bound matched against the registrations' intervals (clause 4.11).
    pub temporal: TemporalSelectionParameters,
    /// `geometryProperty`, of a `GeoJSON` request (clause 6.3.15).
    pub geometry_property: Option<AttributeName>,
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `limit` (clause 6.3.10).
    pub limit: Option<u32>,
    /// `offset` (clause 6.3.10).
    pub offset: Option<u32>,
    /// `count` (clause 6.3.13).
    pub count: ResultsCount,
}

impl QueryRegistrationsParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.8.3.2-1 does not define, one given
    /// twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<QueryRegistrationsParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&SELECTION, &PAGING])?;
        Ok(QueryRegistrationsParameters {
            selection: SelectionParameters::read(&received)?,
            temporal: TemporalSelectionParameters::read(&received)?,
            geometry_property: received.parsed(QueryParameter::GeometryProperty, ParameterError::name)?,
            system_attributes: received.system_attributes()?,
            limit: received.parsed(QueryParameter::Limit, ParameterError::integer)?,
            offset: received.parsed(QueryParameter::Offset, ParameterError::integer)?,
            count: received.results_count()?,
        })
    }
}
