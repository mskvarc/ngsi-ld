//! How much of the requested time range a temporal answer covers (NGSI-LD clause 6.3.10).

use crate::{
    http::{
        headers::{ContentRange, OptionalHeader},
        operation::TemporalSelection,
        response::Success,
    },
    model::{DateTime, Duration},
    query::temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery, TimeRelation},
};
use http::StatusCode;
use std::num::NonZeroU64;

/// Whether an answer holds every instance of the requested time range.
///
/// NGSI-LD clause 6.3.10.
///
/// A broker that cannot answer a temporal request at once answers 206 Partial Content with a `Content-Range` naming
/// the part it covers; every other success covers the whole range. The header is an [`OptionalHeader`]: a 206
/// whose range is missing or unreadable still carries the instances it holds.
///
/// The enum is exhaustive: an answer covers the range or a part of it.
///
/// ```
/// use ngsi_ld::http::{Coverage, headers::OptionalHeader};
///
/// assert!(Coverage::Whole.content_range().is_none());
/// assert!(Coverage::Partial(OptionalHeader::Absent).is_partial());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum Coverage {
    /// 200, 201 or 203: every instance of the requested range.
    Whole,
    /// 206: the part of the range the `Content-Range` names.
    Partial(OptionalHeader<ContentRange>),
}

impl Coverage {
    /// Reads the coverage of a successful response: [`Partial`](Self::Partial) for 206, [`Whole`](Self::Whole)
    /// otherwise.
    pub(crate) fn read(response: &Success) -> Coverage {
        if response.status() == StatusCode::PARTIAL_CONTENT {
            Coverage::Partial(OptionalHeader::read(response.headers()))
        } else {
            Coverage::Whole
        }
    }

    /// Returns the range a partial answer covers, when the broker sent a readable one.
    #[must_use]
    pub const fn content_range(&self) -> Option<&ContentRange> {
        match self {
            Self::Partial(range) => range.valid(),
            Self::Whole => None,
        }
    }

    /// The temporal query of the rest of the range, for a partial answer to a request that selected its instances
    /// with `selection`; `None` for a whole answer or one without a readable range.
    ///
    /// Clause 6.3.10 fixes where an answer ends and leaves the following request to the client. A request with
    /// `lastN` runs backwards and its answer ends at the least recent instance it holds, so the rest is before that
    /// instance: `before` it, or `between` the original `timeAt` and it. A request without `lastN` runs forwards and
    /// its answer ends at the most recent instance it holds, so the rest starts there: `after` it, or `between` it
    /// and the original end (`endTimeAt`, or the `timeAt` of a `before` request). `before` and the end of `between`
    /// exclude their bound, so a backwards walk returns each instance once; `after` and the start of `between`
    /// include theirs (clause 4.11), so a forwards walk returns the instances recorded at the boundary again, which
    /// the rest must include since an instance of another dataset may share that time. The Temporal Property,
    /// `lastN` and the aggregation of the request are kept: no one query can ask each attribute for the instances it
    /// still lacks, so the rest asks for `lastN` again, and the answers of a backwards walk are joined with
    /// [`TemporalEntity::absorb_last`](crate::model::evolution::TemporalEntity::absorb_last), which keeps the last
    /// `lastN` of each attribute.
    ///
    /// There is no rest when the boundary does not lie strictly inside the range the request asked for: an answer that
    /// ends where the request began, or past its far end, made no progress, so a walk that follows the rests always
    /// narrows the range and ends, whatever ranges a broker answers with.
    ///
    /// ```
    /// use headers::Header;
    /// use http::HeaderValue;
    /// use ngsi_ld::{
    ///     http::{
    ///         Coverage,
    ///         headers::{ContentRange, OptionalHeader},
    ///         operation::TemporalSelection,
    ///     },
    ///     query::temporal_query::TemporalQuery,
    /// };
    ///
    /// let value = HeaderValue::from_static("DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/*");
    /// let range = ContentRange::decode(&mut [&value].into_iter()).ok();
    /// let partial = Coverage::Partial(range.map_or(OptionalHeader::Absent, OptionalHeader::Valid));
    /// let query = TemporalQuery::between(
    ///     "2018-08-01T12:00:00Z".parse()?,
    ///     "2018-08-01T13:00:00Z".parse()?,
    /// );
    /// let rest = partial.following_query(TemporalSelection::Query(&query));
    /// assert_eq!(
    ///     rest,
    ///     Some(TemporalQuery::between(
    ///         "2018-08-01T12:05:00Z".parse()?,
    ///         "2018-08-01T13:00:00Z".parse()?
    ///     )),
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn following_query(&self, selection: TemporalSelection<'_>) -> Option<TemporalQuery> {
        let boundary = *self.content_range()?.end();
        let rest = match selection {
            TemporalSelection::Query(query) => following_bounded(query, boundary)?,
            TemporalSelection::Unbounded { last_n: Some(_), .. } => TemporalQuery::before(boundary),
            TemporalSelection::Unbounded { last_n: None, .. } => TemporalQuery::after(boundary),
        };
        Some(with_options(
            rest,
            selection.time_property(),
            selection.last_n(),
            selection.aggregation_methods(),
            selection.aggregation_period(),
        ))
    }
}

/// The relation and times of the rest of a temporal query whose answer ended at `boundary`, when the boundary lies
/// strictly inside the range the query asked for.
fn following_bounded(query: &TemporalQuery, boundary: DateTime) -> Option<TemporalQuery> {
    let start = *query.time_at();
    let end = query.end_time_at().copied();
    // Strictly after the lower bound and strictly before the upper one, when the query has them.
    let inside = |lower: Option<DateTime>, upper: Option<DateTime>| lower.is_none_or(|lower| boundary > lower) && upper.is_none_or(|upper| boundary < upper);
    match (query.last_n(), query.relation()) {
        (None, TimeRelation::After) => inside(Some(start), None).then(|| TemporalQuery::after(boundary)),
        (None, TimeRelation::Between) => {
            let end = end?;
            inside(Some(start), Some(end)).then(|| TemporalQuery::between(boundary, end))
        }
        (None, TimeRelation::Before) => inside(None, Some(start)).then(|| TemporalQuery::between(boundary, start)),
        (Some(_), TimeRelation::Before) => inside(None, Some(start)).then(|| TemporalQuery::before(boundary)),
        (Some(_), TimeRelation::After) => inside(Some(start), None).then(|| TemporalQuery::between(start, boundary)),
        (Some(_), TimeRelation::Between) => inside(Some(start), end).then(|| TemporalQuery::between(start, boundary)),
    }
}

/// `query` with the optional members of the request it continues.
fn with_options(
    query: TemporalQuery,
    time_property: Option<TemporalProperty>,
    last_n: Option<NonZeroU64>,
    methods: Option<&AggregationMethods>,
    period: Option<Duration>,
) -> TemporalQuery {
    let mut query = query;
    if let Some(property) = time_property {
        query = query.with_time_property(property);
    }
    if let Some(count) = last_n {
        query = query.with_last_n(count);
    }
    if let Some(methods) = methods {
        // The continued request keeps its own methods; the new query owns its copy.
        query = query.with_aggregation_methods(methods.clone());
    }
    if let Some(period) = period {
        query = query.with_aggregation_period(period);
    }
    query
}

#[cfg(test)]
mod tests;
