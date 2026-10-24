//! Retrieve Temporal Evolution of an Entity (NGSI-LD clauses 5.7.3 and 6.19.3.1).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        Retrieved,
        operation::{
            Aggregated,
            EntityMapRequest,
            Operation,
            RepresentationSpelling,
            SimplifiedTemporal,
            SystemAttributes,
            TemporalSelection,
            projection_rule::{ProjectionRequest, ProjectionRule},
            sealed::{BuildsRequest, Sealed},
            temporal::{TemporalFormat, check_spelling, decode_retrieved},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{
        Complete,
        Completeness,
        DatasetId,
        EntityId,
        Lossless,
        NgsiLdMediaType,
        Output,
        Projected,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalAttributeName, TemporalEntity},
    },
    query::{language_filter::LanguageFilter, projection::Projection},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `GET /temporal/entities/{entityId}`: reads the temporal evolution of one entity (NGSI-LD clause 6.19.3.1).
///
/// `F` is the representation asked for: a [`Lossless`] one answers with a [`TemporalEntity`], [`SimplifiedTemporal`]
/// with a [`SimplifiedTemporalEntity`] and [`Aggregated`] with an [`AggregatedTemporalEntity`] (clause 6.3.12). `C`
/// is what the entity holds of its `id` and `type`: [`Complete`], so `send()` refuses a `pick` or `omit` that
/// removes either, or [`Projected`] (clause 4.21).
///
/// Table 6.19.3.1-1 lists the deprecated `attrs`, `pick`, `omit`, `datasetId`, `lang`, the temporal query,
/// `timeproperty`, `lastN`, `aggrMethods` and `aggrPeriodDuration`; clauses 6.3.11 and 6.3.12 add `options=sysAttrs`
/// and `format`, and clause 6.3.18 `local`, which [`WithLocality`](crate::http::operation::WithLocality)
/// adds. Clause 5.7.3.3 also lists the entity map inputs (`entityMap`, `entityMapLifetime` and the
/// `NGSILD-EntityMap` header), which Table 6.19.3.1-1 omits; the request carries them, as the clause allows.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{http::{Endpoint, operation::{Operation, TemporalSelection, RetrieveTemporalEvolution}}, model::{EntityId, Lossless}, query::temporal_query::TemporalProperty};
/// use std::num::NonZeroU64;
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let last_ten = RetrieveTemporalEvolution::builder()
///     .id(&id)
///     .selection(TemporalSelection::Unbounded {
///         time_property: Some(TemporalProperty::ModifiedAt),
///         last_n: NonZeroU64::new(10),
///         aggregation_methods: None,
///         aggregation_period: None,
///     })
///     .format(Lossless::Normalized)
///     .build();
/// let request = last_ten.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211?timeproperty=modifiedAt&lastN=10"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
#[builder(start_fn(name = generic_builder, vis = ""))]
pub struct RetrieveTemporalEvolution<'a, F = Lossless, C = Complete> {
    /// What the entity holds of its `id` and `type`, which the start function names.
    #[builder(start_fn)]
    pub completeness: C,
    /// The entity.
    pub id: &'a EntityId,
    /// The instances asked for: a temporal query, or none with `lastN` and the aggregation standing alone.
    pub selection: TemporalSelection<'a>,
    /// The attributes to keep (`attrs`): deprecated by the specification, a synonym of a `pick` of these attributes
    /// (Table 6.19.3.1-1); prefer `pick`. It may name `scope`, which the temporal representation reifies (clause
    /// 4.18). Empty sends no `attrs`.
    #[builder(default)]
    pub attrs: &'a [TemporalAttributeName],
    /// The members to keep (`pick`, clause 4.21).
    pub pick: Option<&'a Projection>,
    /// The members to remove (`omit`, clause 4.21).
    pub omit: Option<&'a Projection>,
    /// The attribute instances to keep, by dataset id, `@none` naming the default instance (`datasetId`, clause
    /// 4.5.5); empty keeps every instance.
    #[builder(default)]
    pub dataset_ids: &'a [DatasetId],
    /// The languages `LanguageProperty` attributes are reduced to (`lang`, clause 4.15).
    pub lang: Option<&'a LanguageFilter>,
    /// The entity map to use or to return (`entityMap`, `entityMapLifetime`, `NGSILD-EntityMap`, clause 5.7.3.3).
    pub entity_map: Option<&'a EntityMapRequest>,
    /// Whether system attributes, and `deletedAt` of deleted instances, are included (clause 6.3.11).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
    /// The representation asked for.
    pub format: F,
    /// How the representation is written: in `format`, `options` or both (clause 6.3.12).
    #[builder(default)]
    pub spelling: RepresentationSpelling,
}

impl<'a, F> RetrieveTemporalEvolution<'a, F> {
    /// Starts the builder of a request for the temporal evolution of one complete entity.
    pub fn builder() -> RetrieveTemporalEvolutionBuilder<'a, F> {
        Self::generic_builder(Complete)
    }
}

impl<'a, F> RetrieveTemporalEvolution<'a, F, Projected> {
    /// Starts the builder of a request for the temporal evolution of one entity a `pick` or `omit` may leave without its `id` or `type`.
    pub fn projected_builder() -> RetrieveTemporalEvolutionBuilder<'a, F, Projected> {
        Self::generic_builder(Projected)
    }
}

impl<F, C> Sealed for RetrieveTemporalEvolution<'_, F, C> {}

/// Implements [`Operation`] for one representation and one completeness, answered with `$item`.
macro_rules! retrieve_operation {
    ($format:ty, $completeness:ty, $item:ty) => {
        impl Operation for RetrieveTemporalEvolution<'_, $format, $completeness> {
            type Outcome = Retrieved<$item>;

            fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<$item>, ResponseError> {
                decode_retrieved::<$completeness, _>(response, self.format)
            }
        }

        impl BuildsRequest for RetrieveTemporalEvolution<'_, $format, $completeness> {
            fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                parts(self)?.into_request(endpoint, scope)
            }
        }
    };
}

retrieve_operation!(Lossless, Complete, TemporalEntity<Output>);
retrieve_operation!(SimplifiedTemporal, Complete, SimplifiedTemporalEntity);
retrieve_operation!(Aggregated, Complete, AggregatedTemporalEntity);
retrieve_operation!(Lossless, Projected, TemporalEntity<Output, Projected>);
retrieve_operation!(SimplifiedTemporal, Projected, SimplifiedTemporalEntity<Projected>);
retrieve_operation!(Aggregated, Projected, AggregatedTemporalEntity<Projected>);

/// The request, after the checks the format and the completeness call for.
fn parts<C: Completeness + ProjectionRule, F: TemporalFormat<C>>(retrieve: &RetrieveTemporalEvolution<'_, F, C>) -> Result<RequestParts, RequestError> {
    retrieve.format.check(retrieve.selection)?;
    check_spelling(retrieve.spelling, retrieve.format.representation(), retrieve.selection)?;
    C::check(ProjectionRequest {
        pick: retrieve.pick,
        omit: retrieve.omit,
        join: None,
    })?;
    let mut query = QueryString::default();
    query.push_list(QueryParameter::Attrs, retrieve.attrs);
    query.push_projection(QueryParameter::Pick, retrieve.pick);
    query.push_projection(QueryParameter::Omit, retrieve.omit);
    query.push_list(QueryParameter::DatasetId, retrieve.dataset_ids);
    query.push_language_filter(QueryParameter::Lang, retrieve.lang);
    retrieve.selection.push_query(&mut query);
    if let Some(entity_map) = retrieve.entity_map {
        entity_map.push_query(&mut query);
    }
    retrieve.spelling.push_format(&mut query, retrieve.format.representation());
    retrieve
        .spelling
        .push_options(&mut query, retrieve.format.representation(), retrieve.system_attributes);
    Ok(RequestParts::resource(Method::GET, ResourcePath::temporal_entity(retrieve.id), query)
        .with_entity_map(retrieve.entity_map.and_then(EntityMapRequest::header))
        .accepting(NgsiLdMediaType::Json))
}
