//! The NGSI-LD Temporal Query Language (NGSI-LD clause 4.11), written in the `timerel`, `timeAt`, `endTimeAt` and
//! `timeproperty` parameters, and the `TemporalQuery` object that carries them in a body with `lastN`,
//! `aggrMethods` and `aggrPeriodDuration` (clause 5.2.21).
//!
//! A temporal query tests a Temporal Property of each instance (`observedAt` unless another is named) against one
//! point in time, or a range for `between`. Only the relation has a grammar; the times are `DateTime`s
//! (clause 4.6.3).
//!
//! ```
//! use ngsi_ld::query::temporal_query::{self, TemporalProperty};
//!
//! let query = temporal_query::between(
//!     "2017-12-13T14:20:00Z".parse()?,
//!     "2017-12-13T14:40:00Z".parse()?,
//! )
//! .time_property(TemporalProperty::ModifiedAt)
//! .build()?;
//! assert_eq!(
//!     query.render().to_string(),
//!     concat!(
//!         r#"{"timerel":"between","timeAt":"2017-12-13T14:20:00Z","#,
//!         r#""endTimeAt":"2017-12-13T14:40:00Z","#,
//!         r#""timeproperty":"modifiedAt"}"#
//!     )
//! );
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Temporal queries
//!
//! [`TemporalQuery`], [`TimeRelation`], [`TemporalProperty`], [`AggregationMethods`], [`AggregationMethodsError`].
//!
//! # Start functions
//!
//! [`before`], [`after`], [`between`].
//!
//! # JSON form
//!
//! [`TemporalQueryMember`], [`TemporalQueryError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::query::temporal_query::json::{TemporalQueryError, TemporalQueryMember};
use crate::{
    model::{DateTime, Duration, evolution::AggregationMethod},
    query::temporal_query::builders::TemporalQueryBuilder,
};
use std::{fmt, num::NonZeroU64, str::FromStr};

pub mod builders;
pub(crate) mod json;

/// The temporal relationship a temporal query tests (the `timerel` production of NGSI-LD clause 4.11).
///
/// The enum is exhaustive: clause 4.11 closes the set.
///
/// ```
/// use ngsi_ld::query::temporal_query::TimeRelation;
///
/// assert_eq!("between".parse::<TimeRelation>(), Ok(TimeRelation::Between));
/// assert_eq!(TimeRelation::Before.to_string(), "before");
/// assert!("during".parse::<TimeRelation>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum TimeRelation {
    /// `before`: the Temporal Property is before `timeAt`, an exclusive bound.
    Before,
    /// `after`: the Temporal Property is at or after `timeAt`, an inclusive bound.
    After,
    /// `between`: the Temporal Property is at or after `timeAt` and before `endTimeAt`.
    Between,
}

/// The Temporal Property a temporal query is applied to (the `timeproperty` of NGSI-LD Table 5.2.21-1); `observedAt`
/// when none is named.
///
/// The enum is exhaustive: Table 5.2.21-1 closes the set.
///
/// ```
/// use ngsi_ld::query::temporal_query::TemporalProperty;
///
/// assert_eq!(
///     "modifiedAt".parse::<TemporalProperty>(),
///     Ok(TemporalProperty::ModifiedAt)
/// );
/// assert!("expiresAt".parse::<TemporalProperty>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum TemporalProperty {
    /// `observedAt`, the default.
    ObservedAt,
    /// `createdAt`.
    CreatedAt,
    /// `modifiedAt`.
    ModifiedAt,
    /// `deletedAt`.
    DeletedAt,
}

/// Why text is not a list of aggregation methods.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum AggregationMethodsError {
    /// The list, or an item of it, is empty.
    #[error("a list of aggregation methods has no empty item")]
    EmptyItem,
    /// An item names no aggregation method of clause 4.5.19.1.
    #[error("`{method}` is not an aggregation method")]
    UnknownMethod {
        /// The item, as written.
        method: String,
    },
}

/// The aggregation methods a temporal query asks for (the `aggrMethods` of NGSI-LD Table 5.2.21-1): at least one,
/// in the order given, printed comma-separated.
///
/// ```
/// use ngsi_ld::query::temporal_query::AggregationMethods;
///
/// let methods: AggregationMethods = "max,avg".parse()?;
/// assert_eq!(methods.to_string(), "max,avg");
/// assert!("max,,avg".parse::<AggregationMethods>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AggregationMethods(Vec<AggregationMethod>);

impl AggregationMethods {
    /// The methods given, unless there are none.
    #[must_use]
    pub fn new(methods: impl IntoIterator<Item = AggregationMethod>) -> Option<Self> {
        let methods: Vec<_> = methods.into_iter().collect();
        (!methods.is_empty()).then_some(Self(methods))
    }

    /// Returns the methods, in order; never empty.
    #[must_use]
    pub fn as_slice(&self) -> &[AggregationMethod] {
        &self.0
    }
}

// The methods are `,`-separated and none may be empty, which a derive cannot express.
impl FromStr for AggregationMethods {
    type Err = AggregationMethodsError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.split(',')
            .map(|item| {
                if item.is_empty() {
                    Err(AggregationMethodsError::EmptyItem)
                } else {
                    item.parse().map_err(|_| AggregationMethodsError::UnknownMethod { method: item.to_owned() })
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}

// Joining a list has no derive.
impl fmt::Display for AggregationMethods {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, method) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(",")?;
            }
            write!(f, "{method}")?;
        }
        Ok(())
    }
}

/// A temporal query (NGSI-LD clauses 4.11 and 5.2.21): a relation with its times, the Temporal Property it is
/// applied to, and the optional `lastN`, `aggrMethods` and `aggrPeriodDuration`.
///
/// `endTimeAt` is present exactly when the relation is `between`, which Table 5.2.21-1 requires and the
/// constructors guarantee. Built by [`before`], [`after`] and [`between`], or from validated parts with
/// [`TemporalQuery::before`] and the other constructors.
///
/// ```
/// use ngsi_ld::query::temporal_query::{TemporalQuery, TimeRelation};
///
/// let query = TemporalQuery::after("2018-08-01T12:00:00Z".parse()?);
/// assert_eq!(query.relation(), TimeRelation::After);
/// assert!(query.end_time_at().is_none());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemporalQuery {
    /// `timerel`.
    relation: TimeRelation,
    /// `timeAt`.
    time_at: DateTime,
    /// `endTimeAt`, exactly for `between`.
    end_time_at: Option<DateTime>,
    /// `timeproperty`, when one is named.
    time_property: Option<TemporalProperty>,
    /// `lastN`.
    last_n: Option<NonZeroU64>,
    /// `aggrMethods`.
    aggregation_methods: Option<AggregationMethods>,
    /// `aggrPeriodDuration`.
    aggregation_period: Option<Duration>,
}

impl TemporalQuery {
    /// A query for the instances recorded before `time`.
    #[must_use]
    pub const fn before(time: DateTime) -> TemporalQuery {
        Self::with_relation(TimeRelation::Before, time, None)
    }

    /// A query for the instances recorded at or after `time`.
    #[must_use]
    pub const fn after(time: DateTime) -> TemporalQuery {
        Self::with_relation(TimeRelation::After, time, None)
    }

    /// A query for the instances recorded at or after `start` and before `end`.
    #[must_use]
    pub const fn between(start: DateTime, end: DateTime) -> TemporalQuery {
        Self::with_relation(TimeRelation::Between, start, Some(end))
    }

    /// A query with no optional member.
    const fn with_relation(relation: TimeRelation, time_at: DateTime, end_time_at: Option<DateTime>) -> TemporalQuery {
        TemporalQuery {
            relation,
            time_at,
            end_time_at,
            time_property: None,
            last_n: None,
            aggregation_methods: None,
            aggregation_period: None,
        }
    }

    /// Applies the query to `property` instead of `observedAt`.
    #[must_use]
    pub fn with_time_property(self, property: TemporalProperty) -> Self {
        Self {
            time_property: Some(property),
            ..self
        }
    }

    /// Keeps only the last `count` instances per attribute and entity (`lastN`).
    #[must_use]
    pub fn with_last_n(self, count: NonZeroU64) -> Self {
        Self { last_n: Some(count), ..self }
    }

    /// Asks for the aggregation `methods` (`aggrMethods`).
    #[must_use]
    pub fn with_aggregation_methods(self, methods: AggregationMethods) -> Self {
        Self {
            aggregation_methods: Some(methods),
            ..self
        }
    }

    /// Aggregates over periods of `period` (`aggrPeriodDuration`).
    #[must_use]
    pub fn with_aggregation_period(self, period: Duration) -> Self {
        Self {
            aggregation_period: Some(period),
            ..self
        }
    }

    /// Returns the relation (`timerel`).
    #[must_use]
    pub const fn relation(&self) -> TimeRelation {
        self.relation
    }

    /// Returns the point in time compared against, the start of the range for `between` (`timeAt`).
    #[must_use]
    pub const fn time_at(&self) -> &DateTime {
        &self.time_at
    }

    /// Returns the end of the range of `between` (`endTimeAt`); `None` for the other relations.
    #[must_use]
    pub const fn end_time_at(&self) -> Option<&DateTime> {
        self.end_time_at.as_ref()
    }

    /// Returns the Temporal Property the query is applied to (`timeproperty`); `None` applies it to `observedAt`.
    #[must_use]
    pub const fn time_property(&self) -> Option<TemporalProperty> {
        self.time_property
    }

    /// Returns how many of the last instances are kept (`lastN`).
    #[must_use]
    pub const fn last_n(&self) -> Option<NonZeroU64> {
        self.last_n
    }

    /// Returns the aggregation methods (`aggrMethods`).
    #[must_use]
    pub const fn aggregation_methods(&self) -> Option<&AggregationMethods> {
        self.aggregation_methods.as_ref()
    }

    /// Returns the duration of each aggregation period (`aggrPeriodDuration`).
    #[must_use]
    pub const fn aggregation_period(&self) -> Option<Duration> {
        self.aggregation_period
    }
}

/// Starts a temporal query for the instances recorded before `time` (`timerel=before`).
#[must_use]
pub const fn before(time: DateTime) -> TemporalQueryBuilder {
    TemporalQueryBuilder::new(TemporalQuery::before(time))
}

/// Starts a temporal query for the instances recorded at or after `time` (`timerel=after`).
#[must_use]
pub const fn after(time: DateTime) -> TemporalQueryBuilder {
    TemporalQueryBuilder::new(TemporalQuery::after(time))
}

/// Starts a temporal query for the instances recorded at or after `start` and before `end` (`timerel=between`).
#[must_use]
pub const fn between(start: DateTime, end: DateTime) -> TemporalQueryBuilder {
    TemporalQueryBuilder::new(TemporalQuery::between(start, end))
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// A temporal query carries no `@context`.
    impl for TemporalQuery
);

#[cfg(test)]
mod tests;
