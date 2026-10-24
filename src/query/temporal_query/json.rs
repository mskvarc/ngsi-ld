//! The JSON form of a temporal query: the `TemporalQuery` object of NGSI-LD clause 5.2.21, which the `Query` body of
//! Query Temporal Evolution (Table 5.2.23-1) and a subscription (Table 5.2.12-1) carry as their `temporalQ` member.

use crate::{
    model::{
        DateTime,
        DocumentError,
        Duration,
        MemberError,
        RawJson,
        reader::{date_time, duration, object, once, parse_json, required, string, token, unexpected, within},
    },
    query::temporal_query::{AggregationMethods, AggregationMethodsError, TemporalProperty, TemporalQuery, TimeRelation},
};
use jstrict::{Object, Value as Json, object::Entry};
use std::num::NonZeroU64;

/// A member of the `TemporalQuery` object (Table 5.2.21-1).
///
/// The enum is `#[non_exhaustive]`: later versions of the specification may add members to the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum TemporalQueryMember {
    /// `timerel`.
    #[strum(serialize = "timerel")]
    Timerel,
    /// `timeAt`.
    #[strum(serialize = "timeAt")]
    TimeAt,
    /// `endTimeAt`.
    #[strum(serialize = "endTimeAt")]
    EndTimeAt,
    /// `timeproperty`.
    #[strum(serialize = "timeproperty")]
    Timeproperty,
    /// `lastN`.
    #[strum(serialize = "lastN")]
    LastN,
    /// `aggrMethods`.
    #[strum(serialize = "aggrMethods")]
    AggrMethods,
    /// `aggrPeriodDuration`.
    #[strum(serialize = "aggrPeriodDuration")]
    AggrPeriodDuration,
}

/// Why JSON data is not a `TemporalQuery` object (NGSI-LD clause 5.2.21); a reader reports it as a
/// [`DocumentError<TemporalQueryError>`](DocumentError), whose `path` names the member at fault.
// `DurationError` chains `jiff`'s error, which carries no equality, so neither can this error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TemporalQueryError {
    /// A problem every reader can meet: the text is not JSON, the data is not an object, a member is missing,
    /// repeated, unknown or not a string, `timerel` or `timeproperty` is not one of its tokens (clause 4.11 and
    /// Table 5.2.21-1), `timeAt` or `endTimeAt` is not a `DateTime` (clause 4.6.3), or `aggrPeriodDuration` is not
    /// a duration (clause 4.5.19.1).
    #[error(transparent)]
    Member(#[from] MemberError),
    /// `endTimeAt` is given with a relation other than `between`, which alone uses it (clause 4.11).
    #[error("`endTimeAt` is only given with `between`")]
    EndTimeWithoutBetween,
    /// `lastN` is not a positive integer.
    #[error("`lastN` must be a positive integer")]
    LastN,
    /// `aggrMethods` is not a list of aggregation methods.
    #[error("invalid aggregation methods")]
    AggregationMethods {
        /// Why the list is invalid.
        #[source]
        source: AggregationMethodsError,
    },
}

/// The members of a `TemporalQuery` object, each read with its grammar and none yet required: the members that
/// bound the query in time, `timerel`, `timeAt` and `endTimeAt`, and those that go with them.
#[derive(Debug, Default)]
pub(crate) struct TemporalQueryParts {
    /// `timerel`.
    relation: Option<TimeRelation>,
    /// `timeAt`.
    time_at: Option<DateTime>,
    /// `endTimeAt`.
    end_time_at: Option<DateTime>,
    /// `timeproperty`.
    pub(crate) time_property: Option<TemporalProperty>,
    /// `lastN`.
    pub(crate) last_n: Option<NonZeroU64>,
    /// `aggrMethods`.
    pub(crate) aggregation_methods: Option<AggregationMethods>,
    /// `aggrPeriodDuration`.
    pub(crate) aggregation_period: Option<Duration>,
}

impl TemporalQueryParts {
    /// Reads the members of a `TemporalQuery` object (Table 5.2.21-1).
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<TemporalQueryError>> {
        let mut parts = Self::default();
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            let Ok(member) = key.parse::<TemporalQueryMember>() else {
                return Err(unexpected(key));
            };
            let key = member.into();
            match member {
                TemporalQueryMember::Timerel => once(&mut parts.relation, key, within(key, token(value, key))?)?,
                TemporalQueryMember::TimeAt => once(&mut parts.time_at, key, within(key, date_time(value, key))?)?,
                TemporalQueryMember::EndTimeAt => once(&mut parts.end_time_at, key, within(key, date_time(value, key))?)?,
                TemporalQueryMember::Timeproperty => once(&mut parts.time_property, key, within(key, token(value, key))?)?,
                TemporalQueryMember::LastN => once(&mut parts.last_n, key, within(key, last_n(&value))?)?,
                TemporalQueryMember::AggrMethods => once(&mut parts.aggregation_methods, key, within(key, aggregation_methods(value))?)?,
                TemporalQueryMember::AggrPeriodDuration => once(&mut parts.aggregation_period, key, within(key, duration(value, key))?)?,
            }
        }
        Ok(parts)
    }

    /// Whether no member bounds the query in time: none of `timerel`, `timeAt` and `endTimeAt` is given; only the
    /// received parameters of the HTTP binding ask.
    #[cfg(feature = "http")]
    pub(crate) const fn is_unbounded(&self) -> bool {
        self.relation.is_none() && self.time_at.is_none() && self.end_time_at.is_none()
    }

    /// The temporal query the members make: `timerel` and `timeAt` are required, and `endTimeAt` is given with
    /// `between` and only with it (clause 4.11).
    pub(crate) fn into_query(self) -> Result<TemporalQuery, DocumentError<TemporalQueryError>> {
        let relation = required(self.relation, TemporalQueryMember::Timerel.into())?;
        let time_at = required(self.time_at, TemporalQueryMember::TimeAt.into())?;
        let query = match (relation, self.end_time_at) {
            (TimeRelation::Between, end) => TemporalQuery::between(time_at, required(end, TemporalQueryMember::EndTimeAt.into())?),
            (TimeRelation::Before | TimeRelation::After, Some(_)) => {
                return Err(DocumentError::top(TemporalQueryError::EndTimeWithoutBetween).within(TemporalQueryMember::EndTimeAt.into()));
            }
            (TimeRelation::Before, None) => TemporalQuery::before(time_at),
            (TimeRelation::After, None) => TemporalQuery::after(time_at),
        };
        Ok(TemporalQuery {
            time_property: self.time_property,
            last_n: self.last_n,
            aggregation_methods: self.aggregation_methods,
            aggregation_period: self.aggregation_period,
            ..query
        })
    }
}

impl TemporalQuery {
    /// Reads the text of a `TemporalQuery` object (NGSI-LD clause 5.2.21), as a subscription or the `Query` body of
    /// Query Temporal Evolution carries it in its `temporalQ` member.
    ///
    /// ```
    /// use ngsi_ld::query::temporal_query::{TemporalQuery, TimeRelation};
    ///
    /// let query = TemporalQuery::parse(r#"{"timerel": "before", "timeAt": "2017-12-13T14:20:00Z"}"#)?;
    /// assert_eq!(query.relation(), TimeRelation::Before);
    /// assert_eq!(
    ///     query.render().to_string(),
    ///     r#"{"timerel":"before","timeAt":"2017-12-13T14:20:00Z"}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError<TemporalQueryError>`](DocumentError) naming the member at fault and what is missing
    /// or invalid, including a `between` without `endTimeAt` and an `endTimeAt` with another relation.
    pub fn parse(text: &str) -> Result<Self, DocumentError<TemporalQueryError>> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the `TemporalQuery` object in canonical form: `timerel`, `timeAt`, `endTimeAt`, `timeproperty`,
    /// `lastN`, `aggrMethods`, `aggrPeriodDuration`, each when present.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(self.to_json())
    }

    /// Reads a `TemporalQuery` object (Table 5.2.21-1).
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<TemporalQueryError>> {
        TemporalQueryParts::from_json(json)?.into_query()
    }

    /// Writes the `TemporalQuery` object in canonical member order.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        let mut push = |member: TemporalQueryMember, value: Json| object.push(<&str>::from(member).into(), value);
        push(TemporalQueryMember::Timerel, Json::from(<&str>::from(self.relation)));
        push(TemporalQueryMember::TimeAt, Json::from(self.time_at.to_string()));
        if let Some(end) = &self.end_time_at {
            push(TemporalQueryMember::EndTimeAt, Json::from(end.to_string()));
        }
        if let Some(property) = self.time_property {
            push(TemporalQueryMember::Timeproperty, Json::from(<&str>::from(property)));
        }
        if let Some(count) = self.last_n {
            push(TemporalQueryMember::LastN, Json::from(count.get()));
        }
        if let Some(methods) = &self.aggregation_methods {
            push(TemporalQueryMember::AggrMethods, Json::from(methods.to_string()));
        }
        if let Some(period) = self.aggregation_period {
            push(TemporalQueryMember::AggrPeriodDuration, Json::from(period.to_string()));
        }
        Json::Object(object)
    }
}

/// Reads `lastN`: a JSON integer greater than zero.
fn last_n(json: &Json) -> Result<NonZeroU64, DocumentError<TemporalQueryError>> {
    match json {
        Json::Number(number) => number
            .as_i64()
            .and_then(|count| u64::try_from(count).ok())
            .and_then(NonZeroU64::new)
            .ok_or(DocumentError::top(TemporalQueryError::LastN)),
        Json::Null | Json::Boolean(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => Err(DocumentError::top(TemporalQueryError::LastN)),
    }
}

/// Reads `aggrMethods`.
fn aggregation_methods(json: Json) -> Result<AggregationMethods, DocumentError<TemporalQueryError>> {
    string(json, TemporalQueryMember::AggrMethods.into())?
        .parse()
        .map_err(|source| DocumentError::top(TemporalQueryError::AggregationMethods { source }))
}
