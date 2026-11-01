//! The instances of each attribute a temporal request asks for (NGSI-LD clause 4.11), and how they are written into
//! a query string.

use crate::{
    http::{QueryParameter, query_string::QueryString},
    model::Duration,
    query::temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery},
};
use std::num::NonZeroU64;

/// The instances of each attribute a temporal request asks for.
///
/// NGSI-LD clause 4.11, Tables 6.18.3.2-1 and 6.19.3.1-1.
///
/// A temporal query selects instances by time, and carries `timeproperty`, `lastN`, `aggrMethods` and
/// `aggrPeriodDuration` with it (Table 5.2.21-1). Retrieve Temporal Evolution may also be sent without one (Table
/// 6.19.3.1-1 makes `timerel` optional), in which case the broker applies its own default range (clause 6.3.10)
/// and `timeproperty`, `lastN` and the aggregation stand alone: Table 6.19.3.1-1 lists `timeproperty` with no
/// condition on `timerel`, and clause 5.7.3.4 ranks the `lastN` instances by it.
///
/// The enum is exhaustive: a request has a temporal query or does not.
///
/// ```
/// use ngsi_ld::{
///     http::operation::TemporalSelection,
///     query::temporal_query::{TemporalProperty, TemporalQuery},
/// };
/// use std::num::NonZeroU64;
///
/// let after = TemporalQuery::after("2018-08-01T12:00:00Z".parse()?);
/// assert_eq!(TemporalSelection::Query(&after).last_n(), None);
/// let last_two = TemporalSelection::Unbounded {
///     time_property: Some(TemporalProperty::ModifiedAt),
///     last_n: NonZeroU64::new(2),
///     aggregation_methods: None,
///     aggregation_period: None,
/// };
/// assert_eq!(last_two.last_n(), NonZeroU64::new(2));
/// assert_eq!(last_two.time_property(), Some(TemporalProperty::ModifiedAt));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalSelection<'a> {
    /// The instances the temporal query selects: `timerel`, `timeAt`, `endTimeAt`, `timeproperty`, and its `lastN`,
    /// `aggrMethods` and `aggrPeriodDuration`.
    Query(&'a TemporalQuery),
    /// No time bound: the broker's default range.
    Unbounded {
        /// The Temporal Property the instances are ranked by (`timeproperty`); `observedAt` when `None`.
        time_property: Option<TemporalProperty>,
        /// Only the last instances of each attribute (`lastN`).
        last_n: Option<NonZeroU64>,
        /// The aggregation methods (`aggrMethods`, clause 4.5.19).
        aggregation_methods: Option<&'a AggregationMethods>,
        /// The duration of each aggregation period (`aggrPeriodDuration`).
        aggregation_period: Option<Duration>,
    },
}

impl<'a> TemporalSelection<'a> {
    /// Returns the Temporal Property the selection applies to (`timeproperty`); `None` applies it to `observedAt`.
    #[must_use]
    pub const fn time_property(self) -> Option<TemporalProperty> {
        match self {
            Self::Query(query) => query.time_property(),
            Self::Unbounded { time_property, .. } => time_property,
        }
    }

    /// Returns how many of the last instances of each attribute are asked for (`lastN`).
    #[must_use]
    pub const fn last_n(self) -> Option<NonZeroU64> {
        match self {
            Self::Query(query) => query.last_n(),
            Self::Unbounded { last_n, .. } => last_n,
        }
    }

    /// Returns the aggregation methods asked for (`aggrMethods`).
    #[must_use]
    pub const fn aggregation_methods(self) -> Option<&'a AggregationMethods> {
        match self {
            Self::Query(query) => query.aggregation_methods(),
            Self::Unbounded { aggregation_methods, .. } => aggregation_methods,
        }
    }

    /// Returns the duration of each aggregation period (`aggrPeriodDuration`).
    #[must_use]
    pub const fn aggregation_period(self) -> Option<Duration> {
        match self {
            Self::Query(query) => query.aggregation_period(),
            Self::Unbounded { aggregation_period, .. } => aggregation_period,
        }
    }

    /// Adds the parameters to a query string: `timerel`, `timeAt`, `endTimeAt`, `timeproperty` (clause 4.11), then
    /// `lastN`, `aggrMethods` (a comma-separated list, its `,` literal) and `aggrPeriodDuration`.
    pub(crate) fn push_query(self, query: &mut QueryString) {
        match self {
            Self::Query(temporal) => push_time_bound(query, temporal),
            Self::Unbounded { time_property, .. } => query.push_some(QueryParameter::Timeproperty, time_property),
        }
        query.push_some(QueryParameter::LastN, self.last_n());
        if let Some(methods) = self.aggregation_methods() {
            query.push_list(QueryParameter::AggrMethods, methods.as_slice());
        }
        query.push_some(QueryParameter::AggrPeriodDuration, self.aggregation_period());
    }
}

/// Adds the time bound of a temporal query to a query string: `timerel`, `timeAt`, `endTimeAt` and `timeproperty`
/// (clause 4.11), the parameters every operation taking a temporal query in its query string lists.
pub(crate) fn push_time_bound(query: &mut QueryString, temporal: &TemporalQuery) {
    query.push(QueryParameter::Timerel, temporal.relation());
    query.push(QueryParameter::TimeAt, temporal.time_at());
    query.push_some(QueryParameter::EndTimeAt, temporal.end_time_at());
    query.push_some(QueryParameter::Timeproperty, temporal.time_property());
}
