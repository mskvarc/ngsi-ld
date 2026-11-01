//! The Temporal API operations (NGSI-LD clauses 5.6.11 to 5.6.16, 5.7.3 and 5.7.4), bound to `temporal/entities`
//! and its sub-resources and to `temporal/entityOperations/query` (clauses 6.18 to 6.22 and 6.24): one request type
//! per operation, the representations a temporal answer is asked in, and how the answers are read.

use crate::{
    http::{
        BodyError,
        Page,
        ResponseContext,
        ResponseError,
        Retrieved,
        operation::{
            ArgumentError,
            RepresentationSpelling,
            RepresentationToken,
            TemporalSelection,
            projection_rule::ProjectionRule,
            representation_spelling::RepresentationFamily,
        },
        response::{Success, read_array},
    },
    model::{
        Completeness,
        DocumentError,
        Lossless,
        Output,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalEntity, read::read_temporal_entity},
    },
};
use bytes::Bytes;
use http::{Response, StatusCode};
use jstrict::Value as Json;

pub(super) mod add_attributes;
pub(super) mod create_or_update;
pub(super) mod delete;
pub(super) mod delete_attribute;
pub(super) mod delete_instance;
pub(super) mod instance_paging;
pub(super) mod interval_request;
pub(super) mod modify_instance;
pub(super) mod parameters;
pub(super) mod post_parameters;
pub(super) mod query;
pub(super) mod retrieve;
pub(super) mod selection;

/// Asks for the simplified temporal representation.
///
/// It is written `format=temporalValues` (NGSI-LD clauses 4.5.9 and 6.3.12) and answered with
/// [`SimplifiedTemporalEntity`] values.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{
///             Operation,
///             RetrieveTemporalEvolution,
///             SimplifiedTemporal,
///             TemporalSelection,
///         },
///     },
///     model::EntityId,
///     query::temporal_query::TemporalQuery,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let since = TemporalQuery::after("2018-08-01T12:00:00Z".parse()?);
/// let retrieve = RetrieveTemporalEvolution::builder()
///     .id(&id)
///     .selection(TemporalSelection::Query(&since))
///     .format(SimplifiedTemporal)
///     .build();
/// let request = retrieve.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/temporal/entities",
///         "/urn:ngsi-ld:Vehicle:B9211?timerel=after&",
///         "timeAt=2018-08-01T12%3A00%3A00Z&format=temporalValues"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SimplifiedTemporal;

/// Asks for the aggregated temporal representation.
///
/// It is written `format=aggregatedValues` (NGSI-LD clauses 4.5.19 and 6.3.12) and answered with
/// [`AggregatedTemporalEntity`] values.
///
/// Tables 6.18.3.2-1 and 6.19.3.1-1 require `aggrMethods` with this format, so a request whose
/// [`TemporalSelection`] names no aggregation method is refused before sending
/// ([`ArgumentError::AggregationWithoutMethods`]).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Aggregated, Operation, RetrieveTemporalEvolution, TemporalSelection},
///     },
///     model::{EntityId, evolution::AggregationMethod},
///     query::temporal_query,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let hourly_maximum = temporal_query::after("2018-08-01T12:00:00Z".parse()?)
///     .aggregation_methods([AggregationMethod::Max])
///     .aggregation_period("PT1H".parse()?)
///     .build()?;
/// let retrieve = RetrieveTemporalEvolution::builder()
///     .id(&id)
///     .selection(TemporalSelection::Query(&hourly_maximum))
///     .format(Aggregated)
///     .build();
/// let request = retrieve.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/temporal/entities",
///         "/urn:ngsi-ld:Vehicle:B9211?timerel=after&",
///         "timeAt=2018-08-01T12%3A00%3A00Z&aggrMethods=max&",
///         "aggrPeriodDuration=PT1H&format=aggregatedValues"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Aggregated;

/// The representations of a temporal answer.
///
/// They are the normalized and concise temporal representations (NGSI-LD clauses 4.5.7 and 4.5.8), the simplified one
/// (`temporalValues`, clause 4.5.9) and the aggregated one (`aggregatedValues`, clause 4.5.19), as clause 6.3.12 and
/// Table 6.3.12-1 list them.
///
/// The enum is exhaustive: clause 6.3.12 closes the set.
///
/// ```
/// use ngsi_ld::http::operation::TemporalRepresentation;
///
/// assert_eq!(
///     TemporalRepresentation::default(),
///     TemporalRepresentation::Normalized
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TemporalRepresentation {
    /// The normalized temporal representation, the default.
    #[default]
    Normalized,
    /// The concise temporal representation.
    Concise,
    /// The simplified temporal representation, `temporalValues`.
    Simplified,
    /// `aggregatedValues`.
    Aggregated,
}

/// The representations of temporal entities. Table 6.3.12-1 writes `temporalValues` and `aggregatedValues`, and no
/// token for the normalized default. The concise temporal representation has no token in that table, but clauses
/// 4.5.7 and 4.5.8 make each instance of a temporal attribute a Property or Relationship "as mandated by clause
/// 4.5.2" (and 4.5.3), whose concise form is selected "through specific request parameters" (clauses 4.5.2 and
/// 4.5.3), the `concise` token of clause 6.3.7, so it is written as such.
impl RepresentationFamily for TemporalRepresentation {
    fn table_token(self) -> Option<RepresentationToken> {
        match self {
            Self::Normalized => None,
            Self::Concise => Some(RepresentationToken::Concise),
            Self::Simplified => Some(RepresentationToken::TemporalValues),
            Self::Aggregated => Some(RepresentationToken::AggregatedValues),
        }
    }

    fn from_token(token: RepresentationToken) -> Option<Self> {
        match token {
            RepresentationToken::Concise => Some(Self::Concise),
            RepresentationToken::TemporalValues => Some(Self::Simplified),
            RepresentationToken::AggregatedValues => Some(Self::Aggregated),
            RepresentationToken::Normalized | RepresentationToken::Simplified | RepresentationToken::KeyValues => None,
        }
    }
}

/// Refuses how a temporal request writes its representation when the spelling cannot ask for the answer read: an
/// `options` keyword with no `format` to take precedence over it (clause 6.3.12), and `aggregatedValues` in
/// `options` without aggregation methods, which Tables 6.18.3.2-1 and 6.19.3.1-1 require "if aggregatedValues is
/// present in the options parameter", whichever representation `format` asks for.
pub(crate) fn check_spelling(
    spelling: RepresentationSpelling,
    representation: TemporalRepresentation,
    selection: TemporalSelection<'_>,
) -> Result<(), ArgumentError> {
    spelling.check(representation)?;
    if spelling.options_keyword(representation) == Some(RepresentationToken::AggregatedValues) && selection.aggregation_methods().is_none() {
        return Err(ArgumentError::AggregationWithoutMethods);
    }
    Ok(())
}

/// How a temporal request asks for its answer and reads it; `C` is what the entities hold of their `id` and
/// `type`.
pub(crate) trait TemporalFormat<C: Completeness>: Copy {
    /// What one temporal entity of the answer is.
    type Item;

    /// The representation asked for (Table 6.3.12-1).
    fn representation(self) -> TemporalRepresentation;

    /// Refuses a selection the answer cannot be asked for with.
    fn check(self, _selection: TemporalSelection<'_>) -> Result<(), ArgumentError> {
        Ok(())
    }

    /// Reads one temporal entity.
    fn read(self, json: Json) -> Result<Self::Item, DocumentError>;
}

impl<C: ProjectionRule> TemporalFormat<C> for Lossless {
    type Item = TemporalEntity<Output, C>;

    fn representation(self) -> TemporalRepresentation {
        match self {
            Self::Normalized => TemporalRepresentation::Normalized,
            Self::Concise => TemporalRepresentation::Concise,
        }
    }

    fn read(self, json: Json) -> Result<TemporalEntity<Output, C>, DocumentError> {
        read_temporal_entity(json, self)
    }
}

impl<C: ProjectionRule> TemporalFormat<C> for SimplifiedTemporal {
    type Item = SimplifiedTemporalEntity<C>;

    fn representation(self) -> TemporalRepresentation {
        TemporalRepresentation::Simplified
    }

    fn read(self, json: Json) -> Result<SimplifiedTemporalEntity<C>, DocumentError> {
        SimplifiedTemporalEntity::from_json(json)
    }
}

impl<C: ProjectionRule> TemporalFormat<C> for Aggregated {
    type Item = AggregatedTemporalEntity<C>;

    fn representation(self) -> TemporalRepresentation {
        TemporalRepresentation::Aggregated
    }

    fn check(self, selection: TemporalSelection<'_>) -> Result<(), ArgumentError> {
        selection.aggregation_methods().map(|_| ()).ok_or(ArgumentError::AggregationWithoutMethods)
    }

    fn read(self, json: Json) -> Result<AggregatedTemporalEntity<C>, DocumentError> {
        AggregatedTemporalEntity::from_json(json)
    }
}

/// Reads the answer of Retrieve Temporal Evolution: 200, 203 when the broker amended it to another version (clause
/// 4.3.6.8), or 206 when it covers part of the range (clause 6.3.10).
fn decode_retrieved<C: Completeness, F: TemporalFormat<C>>(
    context: &ResponseContext,
    response: Response<Bytes>,
    format: F,
) -> Result<Retrieved<F::Item>, ResponseError> {
    Retrieved::decode(
        context,
        response,
        &[StatusCode::OK, StatusCode::NON_AUTHORITATIVE_INFORMATION, StatusCode::PARTIAL_CONTENT],
        |response| format.read(response.json()?).map_err(|source| response.body_error(source)),
    )
}

/// Reads a page of Query Temporal Evolution: 200, 201 when an entity map was created, 203, or 206 (Table
/// 6.18.3.2-2, clause 6.3.10).
fn decode_page<C: Completeness, F: TemporalFormat<C>>(context: &ResponseContext, response: Response<Bytes>, format: F) -> Result<Page<F::Item>, ResponseError> {
    Page::decode(
        context,
        response,
        &[
            StatusCode::OK,
            StatusCode::CREATED,
            StatusCode::NON_AUTHORITATIVE_INFORMATION,
            StatusCode::PARTIAL_CONTENT,
        ],
        |response| Ok((read_list(response, format)?, None)),
    )
}

/// Reads a body that is a list of temporal entities; an error names the index of the entity at fault.
fn read_list<C: Completeness, F: TemporalFormat<C>>(response: &Success, format: F) -> Result<Vec<F::Item>, ResponseError> {
    read_array(response.json()?, |item| format.read(item), BodyError::Document).map_err(|source| response.body_error(source))
}
