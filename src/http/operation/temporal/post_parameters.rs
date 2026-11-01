//! The query string of a `POST` Query Temporal Evolution request in typed form (NGSI-LD clause 6.24.3.1): the
//! parameters both bindings of the query carry in the query string, since the `Query` body of clause 5.2.23 cannot.

use crate::{
    http::{
        QueryParameter,
        operation::{
            InstancePaging,
            ParameterError,
            RepresentationSpelling,
            ResultsCount,
            SystemAttributes,
            TemporalRepresentation,
            received_parameters::ReceivedQuery,
            representation_spelling::received::read_representation,
        },
    },
    query::Locality,
};

/// The parameters Query Temporal Evolution accepts beside the selection, the assembly and the temporal selection,
/// and the only ones its POST binding carries in the query string, since the `Query` body of clause 5.2.23 cannot:
/// `format` and `options` (clauses 6.3.11 and 6.3.12), `limit` and `offset` (clause 6.3.10), `count` (clause
/// 6.3.13), `local` (clause 6.3.18), and `firstN` and `offsetN` (ETSI TS 104 176 clause 6.4.7.3).
pub(super) const QUERY: [QueryParameter; 8] = [
    QueryParameter::Format,
    QueryParameter::Options,
    QueryParameter::Limit,
    QueryParameter::Offset,
    QueryParameter::Count,
    QueryParameter::Local,
    QueryParameter::FirstN,
    QueryParameter::OffsetN,
];

/// The query string of a `POST` Query Temporal Evolution request, read in typed form.
///
/// The request is `POST /temporal/entityOperations/query` (NGSI-LD clause 6.24.3.1). A context source receives this
/// query string when a broker forwards a temporal query to it through the POST binding (clause 5.7.4.4).
///
/// The selection and the temporal query travel in the `Query` body, which
/// [`QueryDocument::parse`](crate::snapshot::QueryDocument::parse) reads; the query string holds only what the body
/// cannot, so a selection or temporal parameter such as `type` or `timerel` is reported as undefined here.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{
///     ParameterError,
///     QueryTemporalEvolutionPostParameters,
///     TemporalRepresentation,
/// };
///
/// let received =
///     QueryTemporalEvolutionPostParameters::parse("format=temporalValues&limit=5&offset=10")?;
/// assert_eq!(received.representation, TemporalRepresentation::Simplified);
/// assert_eq!((received.limit, received.offset), (Some(5), Some(10)));
/// assert!(matches!(
///     QueryTemporalEvolutionPostParameters::parse("timerel=after"),
///     Err(ParameterError::Unknown { .. })
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryTemporalEvolutionPostParameters {
    /// The representation `format` or `options` asks for, normalized when neither does (clause 6.3.12).
    pub representation: TemporalRepresentation,
    /// How the representation is written.
    pub spelling: RepresentationSpelling,
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `limit` (clause 6.3.10).
    pub limit: Option<u32>,
    /// `offset` (clause 6.3.10).
    pub offset: Option<u32>,
    /// `count` (clause 6.3.13).
    pub count: ResultsCount,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
    /// `firstN` and `offsetN`, which the `TemporalQuery` object of the body cannot carry (ETSI TS 104 176 clause
    /// 6.4.7.3).
    pub instance_paging: InstancePaging,
}

impl QueryTemporalEvolutionPostParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one the query string of a POST temporal query does
    /// not carry, one given twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<QueryTemporalEvolutionPostParameters, ParameterError> {
        QueryTemporalEvolutionPostParameters::read(&ReceivedQuery::read(query, &[&QUERY])?)
    }

    /// Reads the parameters both bindings carry in the query string.
    pub(super) fn read(received: &ReceivedQuery<'_>) -> Result<QueryTemporalEvolutionPostParameters, ParameterError> {
        let representation = read_representation::<TemporalRepresentation>(received)?;
        Ok(QueryTemporalEvolutionPostParameters {
            representation: representation.representation,
            spelling: representation.spelling,
            system_attributes: representation.system_attributes,
            limit: received.parsed(QueryParameter::Limit, ParameterError::integer)?,
            offset: received.parsed(QueryParameter::Offset, ParameterError::integer)?,
            count: received.results_count()?,
            locality: received.locality()?,
            instance_paging: InstancePaging::read(received)?,
        })
    }
}
