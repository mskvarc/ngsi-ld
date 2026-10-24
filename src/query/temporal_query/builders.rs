//! The builder of temporal queries, started by [`before`](crate::query::temporal_query::before),
//! [`after`](crate::query::temporal_query::after) and [`between`](crate::query::temporal_query::between).
//!
//! [`TemporalQueryBuilder`].

use crate::{
    model::{BuildError, Duration, evolution::AggregationMethod},
    query::{
        Problem,
        build_error::Deferred,
        temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery},
    },
};
use std::num::NonZeroU64;

/// A temporal query under construction.
///
/// `lastN`, the aggregation methods and the aggregation period are validated on the spot, without panicking;
/// [`build`](Self::build) reports every problem at once.
///
/// ```
/// use ngsi_ld::query::{Problem, temporal_query};
///
/// let error = temporal_query::before("2018-08-01T12:00:00Z".parse()?)
///     .last_n(0)
///     .aggregation_methods([])
///     .build()
///     .err();
/// let problems = error.as_ref().map(|error| error.problems());
/// assert!(matches!(
///     problems,
///     Some([Problem::ZeroLastN, Problem::EmptyAggregationMethods])
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalQueryBuilder {
    /// The relation and its times, with any member already set.
    query: TemporalQuery,
    /// `lastN`, or why it is refused.
    last_n: Deferred<Option<NonZeroU64>>,
    /// `aggrMethods`, or why they are refused.
    aggregation_methods: Deferred<Option<AggregationMethods>>,
    /// `aggrPeriodDuration`, or why it is refused.
    aggregation_period: Deferred<Option<Duration>>,
}

impl TemporalQueryBuilder {
    /// A builder starting from `query`.
    pub(crate) const fn new(query: TemporalQuery) -> Self {
        Self {
            query,
            last_n: Deferred::valid(None),
            aggregation_methods: Deferred::valid(None),
            aggregation_period: Deferred::valid(None),
        }
    }

    /// Applies the query to `property` instead of `observedAt` (`timeproperty`).
    #[must_use]
    pub fn time_property(self, property: TemporalProperty) -> Self {
        Self {
            query: self.query.with_time_property(property),
            ..self
        }
    }

    /// Keeps only the last `count` instances per attribute and entity (`lastN`); 0 is refused.
    #[must_use]
    pub fn last_n(self, count: u64) -> Self {
        let last_n = Deferred::from_result(NonZeroU64::new(count).map(Some).ok_or(Problem::ZeroLastN));
        Self { last_n, ..self }
    }

    /// Asks for the aggregation `methods` (`aggrMethods`), in the order given; none is refused.
    #[must_use]
    pub fn aggregation_methods(self, methods: impl IntoIterator<Item = AggregationMethod>) -> Self {
        let aggregation_methods = Deferred::from_result(AggregationMethods::new(methods).map(Some).ok_or(Problem::EmptyAggregationMethods));
        Self { aggregation_methods, ..self }
    }

    /// Aggregates over periods of `period`, an ISO 8601 duration such as `PT4M` (`aggrPeriodDuration`).
    #[must_use]
    pub fn aggregation_period(self, period: Duration) -> Self {
        Self {
            aggregation_period: Deferred::valid(Some(period)),
            ..self
        }
    }

    /// Returns the temporal query.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing a `lastN` of 0, an empty list of aggregation methods and an aggregation
    /// period that is not a duration.
    pub fn build(self) -> Result<TemporalQuery, BuildError<Problem>> {
        let query = self.query;
        self.last_n
            .zip(self.aggregation_methods)
            .zip(self.aggregation_period)
            .map(|((last_n, methods), period)| TemporalQuery {
                last_n: last_n.or(query.last_n),
                aggregation_methods: methods.or(query.aggregation_methods),
                aggregation_period: period.or(query.aggregation_period),
                ..query
            })
            .finish()
    }
}

/// A temporal query already built.
impl From<TemporalQuery> for TemporalQueryBuilder {
    fn from(query: TemporalQuery) -> Self {
        Self::new(query)
    }
}

impl TryFrom<TemporalQueryBuilder> for TemporalQuery {
    type Error = BuildError<Problem>;

    fn try_from(builder: TemporalQueryBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
