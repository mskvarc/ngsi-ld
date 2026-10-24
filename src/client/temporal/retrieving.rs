//! Retrieve Temporal Evolution of an Entity (clause 6.19.3.1), once per representation and once per completeness.

use crate::{
    client::{Answering, ClientError, Temporal, Transport, arguments::built, temporal::SelectionArguments},
    http::{
        Retrieved,
        operation::{Aggregated, EntityMapRequest, RepresentationSpelling, RetrieveTemporalEvolution, SimplifiedTemporal, SystemAttributes},
    },
    model::{
        Complete,
        DatasetId,
        Duration,
        EntityId,
        Lossless,
        Output,
        Projected,
        Tenant,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalAttributeName, TemporalEntity},
    },
    query::{
        Locality,
        language_filter::{LanguageFilter, builders::LanguageFilterBuilder},
        projection::{Projection, builders::ProjectionBuilder},
        temporal_query::{AggregationMethods, TemporalProperty, builders::TemporalQueryBuilder},
    },
};
use bon::bon;

/// Generates the Retrieve Temporal Evolution method for one representation and one completeness.
macro_rules! retrieve_method {
    (
        $(#[doc = $doc:literal])*
        $name:ident, $item:ty,
        $format:ident => $format_value:expr, [$($format_parameter:tt)*],
        completeness => $completeness:expr,
        send => $send:ident
    ) => {
        #[bon]
        impl<T: Transport, M: Answering> Temporal<'_, T, M> {
            $(#[doc = $doc])*
            #[builder(derive(Debug), finish_fn(name = send, doc {
                /// Sends the request and reads the response.
                ///
                /// # Errors
                ///
                /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, the
                /// broker answers with an error, or the transport fails.
            }))]
            pub async fn $name(
                &self,
                #[builder(start_fn)] id: &EntityId,
                #[builder(into)] temporal_q: Option<TemporalQueryBuilder>,
                time_property: Option<TemporalProperty>,
                last_n: Option<u64>,
                aggregation_methods: Option<AggregationMethods>,
                aggregation_period: Option<Duration>,
                #[builder(default)] attrs: Vec<TemporalAttributeName>,
                #[builder(into)] pick: Option<ProjectionBuilder>,
                #[builder(into)] omit: Option<ProjectionBuilder>,
                #[builder(default)] dataset_ids: Vec<DatasetId>,
                #[builder(into)] lang: Option<LanguageFilterBuilder>,
                entity_map: Option<&EntityMapRequest>,
                #[builder(default)] system_attributes: SystemAttributes,
                $($format_parameter)*
                #[builder(default)] spelling: RepresentationSpelling,
                #[builder(default)] locality: Locality,
                tenant: Option<&Tenant>,
            ) -> Result<M::Answer<Retrieved<$item>>, ClientError<T::Error>> {
                let arguments = SelectionArguments::new(temporal_q, time_property, last_n, aggregation_methods, aggregation_period)?;
                let pick: Option<Projection> = built(pick)?;
                let omit: Option<Projection> = built(omit)?;
                let lang: Option<LanguageFilter> = built(lang)?;
                let $format = $format_value;
                let retrieve = RetrieveTemporalEvolution {
                    id,
                    selection: arguments.selection()?,
                    attrs: &attrs,
                    pick: pick.as_ref(),
                    omit: omit.as_ref(),
                    dataset_ids: &dataset_ids,
                    lang: lang.as_ref(),
                    entity_map,
                    system_attributes,
                    format: $format,
                    spelling,
                    completeness: $completeness,
                };
                self.$send(locality, retrieve, tenant).await
            }
        }
    };
}

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1), in normalized or concise representation.
    ///
    /// The instances are those of `temporal_q`, or, without one, the broker's default range, of which `last_n`
    /// keeps the last ones by `time_property` (`observedAt` by default, clause 5.7.3.4); `time_property`, `last_n`
    /// and the aggregation beside a temporal query are refused before sending, as the temporal query carries them.
    /// A `pick` or `omit` must keep `id` and `type` ([`retrieve_projected`] reads an answer without them).
    ///
    /// [`retrieve_projected`]: Temporal::retrieve_projected
    retrieve, TemporalEntity<Output>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Complete,
    send => answer
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1), in simplified temporal representation
    /// (`format=temporalValues`, clause 4.5.9).
    ///
    /// The instances are selected as for [`retrieve`](Temporal::retrieve).
    retrieve_values, SimplifiedTemporalEntity,
    format => SimplifiedTemporal, [],
    completeness => Complete,
    send => answer
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1), in aggregated temporal representation
    /// (`format=aggregatedValues`, clause 4.5.19).
    ///
    /// The aggregation methods are those of `temporal_q`, or `aggregation_methods` without one; a request with
    /// neither is refused before sending ([`ArgumentError::AggregationWithoutMethods`]).
    ///
    /// [`ArgumentError::AggregationWithoutMethods`]: crate::http::operation::ArgumentError::AggregationWithoutMethods
    retrieve_aggregated, AggregatedTemporalEntity,
    format => Aggregated, [],
    completeness => Complete,
    send => answer
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1) with a `pick` or `omit` that may remove `id` or
    /// `type` (clause 4.21), in normalized or concise representation.
    retrieve_projected, TemporalEntity<Output, Projected>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Projected,
    send => answer
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1) with a `pick` or `omit` that may remove `id` or
    /// `type` (clause 4.21), in simplified temporal representation.
    retrieve_projected_values, SimplifiedTemporalEntity<Projected>,
    format => SimplifiedTemporal, [],
    completeness => Projected,
    send => answer
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1) with a `pick` or `omit` that may remove `id` or
    /// `type` (clause 4.21), in aggregated temporal representation.
    retrieve_projected_aggregated, AggregatedTemporalEntity<Projected>,
    format => Aggregated, [],
    completeness => Projected,
    send => answer
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1), in normalized or concise representation, over the
    /// whole time range: while the broker answers 206 Partial Content (clause 6.3.10), the rest of the range is asked
    /// for and the answers are joined, each instance once (see [`TemporalEntity::absorb`]), and, with `lastN`, the
    /// last `lastN` instances of each attribute over the whole range (see [`TemporalEntity::absorb_last`]). The
    /// coverage of the answer is that of the last one: [`Coverage::Whole`](crate::http::Coverage::Whole)
    /// once every part is in.
    ///
    /// The instances are selected as for [`retrieve`](Temporal::retrieve). The simplified and aggregated
    /// representations are not walked: a simplified data point carries no `instanceId` or `datasetId`, so one returned
    /// twice at a boundary cannot be told from two instances recorded with the same value at the same time, and an
    /// aggregate of a period two ranges cut is two partial aggregates that cannot be combined.
    retrieve_whole_range, TemporalEntity<Output>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Complete,
    send => whole_range
);

retrieve_method!(
    /// Retrieve Temporal Evolution of an Entity (clause 6.19.3.1) over the whole time range, as
    /// [`retrieve_whole_range`](Temporal::retrieve_whole_range) does, with a `pick` or `omit` that may remove `id` or
    /// `type` (clause 4.21).
    retrieve_projected_whole_range, TemporalEntity<Output, Projected>,
    format => form, [#[builder(default)] form: Lossless,],
    completeness => Projected,
    send => whole_range
);
