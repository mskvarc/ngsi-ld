//! The query string of Query Subscriptions in typed form (NGSI-LD clauses 6.10.3.2 and 6.12.3.2), as a broker reads
//! the request.

use crate::http::{
    QueryParameter,
    operation::{ParameterError, ResultsCount, SystemAttributes, received_parameters::ReceivedQuery},
};

/// The parameters of Query Subscriptions: Tables 6.10.3.2-1 and 6.12.3.2-1 list `limit`, clause 4.12 makes the
/// query paginated, so it takes `offset` and `count`, and clause 6.3.11 adds `options`.
const QUERY: [QueryParameter; 4] = [QueryParameter::Limit, QueryParameter::Offset, QueryParameter::Count, QueryParameter::Options];

/// The query string of a Query Subscriptions request, read in typed form.
///
/// The request is `GET /subscriptions` (NGSI-LD Table 6.10.3.2-1) or `GET /csourceSubscriptions` (Table 6.12.3.2-1),
/// which take the same parameters: the page (`limit`, `offset`, `count`) and `options=sysAttrs`.
///
/// Each parameter is read with its grammar. A `limit` of `0` without `count` is a broker's to refuse (clause 6.3.13)
/// and is not checked here, so that the receiver sees exactly what it was sent; a parameter the tables do not define
/// is reported.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{QuerySubscriptionsParameters, ResultsCount};
///
/// let received = QuerySubscriptionsParameters::parse("limit=10&offset=20&count=true")?;
/// assert_eq!(received.limit, Some(10));
/// assert_eq!(received.offset, Some(20));
/// assert_eq!(received.count, ResultsCount::Include);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct QuerySubscriptionsParameters {
    /// `options=sysAttrs` (clause 6.3.11).
    pub system_attributes: SystemAttributes,
    /// `limit` (clause 6.3.10).
    pub limit: Option<u32>,
    /// `offset` (clause 6.3.10).
    pub offset: Option<u32>,
    /// `count` (clause 6.3.13).
    pub count: ResultsCount,
}

impl QuerySubscriptionsParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one the tables do not define, one given twice, a
    /// number out of range, a `count` that is neither `true` nor `false`, and an `options` keyword other than
    /// `sysAttrs`.
    pub fn parse(query: &str) -> Result<QuerySubscriptionsParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&QUERY])?;
        Ok(QuerySubscriptionsParameters {
            system_attributes: received.system_attributes()?,
            limit: received.parsed(QueryParameter::Limit, ParameterError::integer)?,
            offset: received.parsed(QueryParameter::Offset, ParameterError::integer)?,
            count: received.results_count()?,
        })
    }
}
