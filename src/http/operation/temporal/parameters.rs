//! The query strings of Retrieve and Query Temporal Evolution in typed form (NGSI-LD clauses 6.18.3.2 and 6.19.3.1,
//! Tables 6.18.3.2-1 and 6.19.3.1-1), as a context source reads the temporal requests a broker forwards to it
//! (clauses 5.7.3.4 and 5.7.4.4).

use crate::{
    http::{
        QueryParameter,
        operation::{
            AssemblyParameters,
            EntityMapLifetime,
            ParameterError,
            RepresentationSpelling,
            ResultsCount,
            SelectionParameters,
            SystemAttributes,
            TemporalRepresentation,
            received_parameters::{
                ReceivedQuery,
                selection::{ASSEMBLY, SELECTION},
            },
            representation_spelling::received::read_representation,
        },
    },
    model::{DatasetId, DocumentError, Duration, MemberError, evolution::TemporalAttributeName},
    query::{
        EntityMapReturn,
        Locality,
        language_filter::LanguageFilter,
        projection::Projection,
        temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery, TemporalQueryError, TemporalQueryMember, json::TemporalQueryParts},
    },
};
use jstrict::{Object, Value as Json};
use std::num::NonZeroU64;

/// The parameters of the temporal selection (clause 4.11, Tables 6.18.3.2-1 and 6.19.3.1-1), each with the member
/// of the `TemporalQuery` object (clause 5.2.21) that holds it.
const TEMPORAL: [(QueryParameter, TemporalQueryMember); 7] = [
    (QueryParameter::Timerel, TemporalQueryMember::Timerel),
    (QueryParameter::TimeAt, TemporalQueryMember::TimeAt),
    (QueryParameter::EndTimeAt, TemporalQueryMember::EndTimeAt),
    (QueryParameter::Timeproperty, TemporalQueryMember::Timeproperty),
    (QueryParameter::LastN, TemporalQueryMember::LastN),
    (QueryParameter::AggrMethods, TemporalQueryMember::AggrMethods),
    (QueryParameter::AggrPeriodDuration, TemporalQueryMember::AggrPeriodDuration),
];

/// The parameters of the temporal selection, as names: the first column of [`TEMPORAL`].
const TEMPORAL_PARAMETERS: [QueryParameter; TEMPORAL.len()] = {
    // Iterators are not available in a constant; every slot the fill value takes is overwritten.
    let mut parameters = [QueryParameter::Timerel; TEMPORAL.len()];
    let mut index = 0;
    while index < TEMPORAL.len() {
        parameters[index] = TEMPORAL[index].0;
        index += 1;
    }
    parameters
};

/// The parameters Retrieve Temporal Evolution accepts beside the temporal selection: Table 6.19.3.1-1, the entity
/// map inputs of clause 5.7.3.3, `format` and `options` (clauses 6.3.11 and 6.3.12) and `local` (clause 6.3.18).
const RETRIEVE: [QueryParameter; 10] = [
    QueryParameter::Attrs,
    QueryParameter::Pick,
    QueryParameter::Omit,
    QueryParameter::DatasetId,
    QueryParameter::Lang,
    QueryParameter::EntityMap,
    QueryParameter::EntityMapLifetime,
    QueryParameter::Format,
    QueryParameter::Options,
    QueryParameter::Local,
];

/// The parameters Query Temporal Evolution accepts beside the selection, the assembly and the temporal selection,
/// and the only ones its POST binding carries in the query string, since the `Query` body of clause 5.2.23 cannot:
/// `format` and `options` (clauses 6.3.11 and 6.3.12), `limit` and `offset` (clause 6.3.10), `count` (clause
/// 6.3.13) and `local` (clause 6.3.18).
const QUERY: [QueryParameter; 6] = [
    QueryParameter::Format,
    QueryParameter::Options,
    QueryParameter::Limit,
    QueryParameter::Offset,
    QueryParameter::Count,
    QueryParameter::Local,
];

/// The instances a received temporal request asks for (clause 4.11): a temporal query, or, for Retrieve Temporal
/// Evolution, none (Table 6.19.3.1-1 makes `timerel` optional), with `timeproperty`, `lastN` and the aggregation
/// standing alone.
///
/// The enum is exhaustive: a request has a temporal query or does not.
///
/// ```
/// use ngsi_ld::http::operation::{RetrieveTemporalEvolutionParameters, TemporalSelectionParameters};
/// use std::num::NonZeroU64;
///
/// let received = RetrieveTemporalEvolutionParameters::parse("lastN=3")?;
/// assert!(matches!(received.selection, TemporalSelectionParameters::Unbounded { last_n, .. } if last_n == NonZeroU64::new(3)));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalSelectionParameters {
    /// `timerel`, `timeAt`, `endTimeAt` and the members the temporal query carries with them.
    Query(TemporalQuery),
    /// No time bound.
    Unbounded {
        /// `timeproperty`.
        time_property: Option<TemporalProperty>,
        /// `lastN`.
        last_n: Option<NonZeroU64>,
        /// `aggrMethods`.
        aggregation_methods: Option<AggregationMethods>,
        /// `aggrPeriodDuration`.
        aggregation_period: Option<Duration>,
    },
}

/// The error of a temporal selection parameter.
const fn temporal(source: DocumentError<TemporalQueryError>) -> ParameterError {
    ParameterError::TemporalQuery { source }
}

impl TemporalSelectionParameters {
    /// Reads the temporal selection. With a time bound, the parameters are the members of the `TemporalQuery` object
    /// of clause 5.2.21 under the same names, so they are read by its reader: one grammar, one set of rules.
    fn read(received: &ReceivedQuery<'_>) -> Result<TemporalSelectionParameters, ParameterError> {
        let mut object = Object::new();
        for (parameter, member) in TEMPORAL {
            if let Some(text) = received.text(parameter)? {
                // `lastN` is a JSON number in the object; any other text is left a string, which its reader refuses.
                let number = (member == TemporalQueryMember::LastN)
                    .then(|| text.parse::<Json>().ok())
                    .flatten()
                    .filter(Json::is_number);
                let value = number.unwrap_or_else(|| Json::from(text.into_owned()));
                object.push(<&str>::from(member).into(), value);
            }
        }
        let parts = TemporalQueryParts::from_json(Json::Object(object)).map_err(temporal)?;
        if parts.is_unbounded() {
            return Ok(Self::Unbounded {
                time_property: parts.time_property,
                last_n: parts.last_n,
                aggregation_methods: parts.aggregation_methods,
                aggregation_period: parts.aggregation_period,
            });
        }
        parts.into_query().map(Self::Query).map_err(temporal)
    }
}

/// The query string of a Retrieve Temporal Evolution request (`GET /temporal/entities/{entityId}`, Table
/// 6.19.3.1-1), read in typed form: what a context source receives when a broker forwards it (clause 5.7.3.4).
///
/// Each parameter is read with its grammar; rules that tie parameters together are the broker's to enforce and are
/// not checked, and a parameter the table does not define is reported.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{RetrieveTemporalEvolutionParameters, TemporalRepresentation};
///
/// let received = RetrieveTemporalEvolutionParameters::parse(
///     "timerel=after&timeAt=2024-05-01T10%3A00%3A00Z&format=temporalValues",
/// )?;
/// assert_eq!(received.representation, TemporalRepresentation::Simplified);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetrieveTemporalEvolutionParameters {
    /// The instances asked for.
    pub selection: TemporalSelectionParameters,
    /// `attrs`, deprecated; it may name `scope` (clause 4.18).
    pub attrs: Vec<TemporalAttributeName>,
    /// `pick` (clause 4.21).
    pub pick: Option<Projection>,
    /// `omit` (clause 4.21).
    pub omit: Option<Projection>,
    /// `datasetId` (clause 4.5.5).
    pub dataset_ids: Vec<DatasetId>,
    /// `lang` (clause 4.15).
    pub lang: Option<LanguageFilter>,
    /// `entityMap` (clause 5.7.3.3).
    pub entity_map: EntityMapReturn,
    /// `entityMapLifetime`.
    pub entity_map_lifetime: Option<EntityMapLifetime>,
    /// The representation `format` or `options` asks for, normalized when neither does (clause 6.3.12).
    pub representation: TemporalRepresentation,
    /// How the representation is written.
    pub spelling: RepresentationSpelling,
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl RetrieveTemporalEvolutionParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.19.3.1-1 does not define, one given
    /// twice, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<RetrieveTemporalEvolutionParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&TEMPORAL_PARAMETERS, &RETRIEVE])?;
        let representation = read_representation::<TemporalRepresentation>(&received)?;
        Ok(RetrieveTemporalEvolutionParameters {
            selection: TemporalSelectionParameters::read(&received)?,
            attrs: received.list(QueryParameter::Attrs, ParameterError::name)?,
            pick: received.parsed(QueryParameter::Pick, ParameterError::syntax)?,
            omit: received.parsed(QueryParameter::Omit, ParameterError::syntax)?,
            dataset_ids: received.list(QueryParameter::DatasetId, ParameterError::uri)?,
            lang: received.parsed(QueryParameter::Lang, ParameterError::syntax)?,
            entity_map: received
                .flag(QueryParameter::EntityMap)?
                .map_or(EntityMapReturn::Unspecified, EntityMapReturn::from_flag),
            entity_map_lifetime: received.parsed(QueryParameter::EntityMapLifetime, ParameterError::lifetime)?,
            representation: representation.representation,
            spelling: representation.spelling,
            system_attributes: representation.system_attributes,
            locality: received.locality()?,
        })
    }
}

/// The query string of a Query Temporal Evolution request (`GET /temporal/entities`, Table 6.18.3.2-1), read in
/// typed form: what a context source receives when a broker forwards it (clause 5.7.4.4). A POST query carries its
/// selection in a `Query` body instead, which
/// [`QueryDocument::parse`](crate::snapshot::QueryDocument::parse) reads, and the rest in a query
/// string that [`QueryTemporalEvolutionPostParameters`] reads.
///
/// The temporal query is mandatory (Table 6.18.3.2-1 gives `timerel` and `timeAt` a cardinality of 1); other rules
/// that tie parameters together are the broker's to enforce, and a parameter the table does not define is reported.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::QueryTemporalEvolutionParameters;
///
/// let received = QueryTemporalEvolutionParameters::parse(
///     "type=Vehicle&timerel=before&timeAt=2024-05-01T10%3A00%3A00Z&lastN=2",
/// )?;
/// assert_eq!(
///     received.temporal_q.last_n().map(|count| count.get()),
///     Some(2)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryTemporalEvolutionParameters {
    /// The entities matched and what of them the answer holds, `type` to `lang`; `attrs` may name `scope` (clause
    /// 4.18).
    pub selection: SelectionParameters<TemporalAttributeName>,
    /// `timerel`, `timeAt`, `endTimeAt`, `timeproperty`, `lastN`, `aggrMethods` and `aggrPeriodDuration`.
    pub temporal_q: TemporalQuery,
    /// The ordering, `splitEntities` and the entity map.
    pub assembly: AssemblyParameters,
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
}

impl QueryTemporalEvolutionParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.18.3.2-1 does not define, one given
    /// twice, a value that does not follow its grammar, or a missing `timerel` ([`ParameterError::TemporalQuery`]).
    pub fn parse(query: &str) -> Result<QueryTemporalEvolutionParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&SELECTION, &ASSEMBLY, &TEMPORAL_PARAMETERS, &QUERY])?;
        let temporal_q = match TemporalSelectionParameters::read(&received)? {
            TemporalSelectionParameters::Query(query) => query,
            TemporalSelectionParameters::Unbounded { .. } => {
                return Err(temporal(DocumentError::top(
                    MemberError::MissingMember {
                        member: TemporalQueryMember::Timerel.into(),
                    }
                    .into(),
                )));
            }
        };
        let QueryTemporalEvolutionPostParameters {
            representation,
            spelling,
            system_attributes,
            limit,
            offset,
            count,
            locality,
        } = QueryTemporalEvolutionPostParameters::read(&received)?;
        Ok(QueryTemporalEvolutionParameters {
            selection: SelectionParameters::read(&received)?,
            temporal_q,
            assembly: AssemblyParameters::read(&received)?,
            representation,
            spelling,
            system_attributes,
            limit,
            offset,
            count,
            locality,
        })
    }
}

/// The query string of a Query Temporal Evolution request through `POST /temporal/entityOperations/query` (clause
/// 6.24.3.1), read in typed form: what a context source receives when a broker forwards a temporal query to it
/// through the POST binding (clause 5.7.4.4). The selection and the temporal query travel in the `Query` body, which
/// [`QueryDocument::parse`](crate::snapshot::QueryDocument::parse) reads; the query string holds
/// only what the body cannot, so a selection or temporal parameter such as `type` or `timerel` is reported as
/// undefined here.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{ParameterError, TemporalRepresentation, QueryTemporalEvolutionPostParameters};
///
/// let received = QueryTemporalEvolutionPostParameters::parse("format=temporalValues&limit=5&offset=10")?;
/// assert_eq!(received.representation, TemporalRepresentation::Simplified);
/// assert_eq!((received.limit, received.offset), (Some(5), Some(10)));
/// assert!(matches!(
///     QueryTemporalEvolutionPostParameters::parse("timerel=after"),
///     Err(ParameterError::Unknown { name }) if name == "timerel"
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
    fn read(received: &ReceivedQuery<'_>) -> Result<QueryTemporalEvolutionPostParameters, ParameterError> {
        let representation = read_representation::<TemporalRepresentation>(received)?;
        Ok(QueryTemporalEvolutionPostParameters {
            representation: representation.representation,
            spelling: representation.spelling,
            system_attributes: representation.system_attributes,
            limit: received.parsed(QueryParameter::Limit, ParameterError::integer)?,
            offset: received.parsed(QueryParameter::Offset, ParameterError::integer)?,
            count: received.results_count()?,
            locality: received.locality()?,
        })
    }
}
