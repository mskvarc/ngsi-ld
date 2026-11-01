//! The temporal pagination of a temporal request beside its `lastN`: `firstN` and `offsetN` (ETSI TS 104 175 clause
//! 7.4.2.5, ETSI TS 104 176 clause 6.4.7.3).

use crate::http::{
    QueryParameter,
    operation::{ArgumentError, ParameterError, TemporalSelection, received_parameters::ReceivedQuery},
    query_string::QueryString,
};
use std::num::NonZeroU64;

/// How a temporal request pages the instances of each attribute, beside the `lastN` its selection carries.
///
/// ETSI TS 104 176 clause 6.4.7.3 pages the instances in ascending order of their timestamps with `firstN`, in
/// descending order with `lastN`, and in ascending order with the broker's default limit when neither is given;
/// `offsetN` selects the page in the chosen order (ETSI TS 104 175 clause 7.4.2.5). `lastN` stays in the
/// [`TemporalSelection`], which carries it in the `TemporalQuery` object too (NGSI-LD Table 5.2.21-1); `firstN` and
/// `offsetN` are parameters of the query string alone, so a POST query sends them there as well.
///
/// The default asks for neither.
///
/// ```
/// use ngsi_ld::http::operation::InstancePaging;
/// use std::num::NonZeroU64;
///
/// let second_page = InstancePaging {
///     first_n: NonZeroU64::new(2),
///     offset_n: Some(2),
/// };
/// assert_eq!(InstancePaging::default().first_n, None);
/// assert_eq!(second_page.offset_n, Some(2));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct InstancePaging {
    /// Only the first instances of each attribute, in ascending order (`firstN`, ETSI TS 104 176 Table 6.4.7.3-1).
    pub first_n: Option<NonZeroU64>,
    /// The offset of the first instance of the page, from the beginning or the end of the instances depending on
    /// whether they are paged in ascending or descending order (`offsetN`, ETSI TS 104 176 Table 6.4.7.3-2).
    pub offset_n: Option<u64>,
}

impl InstancePaging {
    /// Refuses `firstN` beside the `lastN` of `selection`, which ETSI TS 104 176 clause 6.4.7.3 gives no meaning.
    pub(crate) const fn check(self, selection: TemporalSelection<'_>) -> Result<(), ArgumentError> {
        match (self.first_n, selection.last_n()) {
            (Some(_), Some(_)) => Err(ArgumentError::FirstNBesideLastN),
            (Some(_), None) | (None, Some(_) | None) => Ok(()),
        }
    }

    /// Adds `firstN` and `offsetN` to a query string, each when given.
    pub(crate) fn push_query(self, query: &mut QueryString) {
        query.push_some(QueryParameter::FirstN, self.first_n);
        query.push_some(QueryParameter::OffsetN, self.offset_n);
    }

    /// Reads `firstN` and `offsetN` from a received query string.
    pub(crate) fn read(received: &ReceivedQuery<'_>) -> Result<InstancePaging, ParameterError> {
        Ok(InstancePaging {
            first_n: received.parsed(QueryParameter::FirstN, ParameterError::integer)?,
            offset_n: received.parsed(QueryParameter::OffsetN, ParameterError::integer)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{
            operation::{ArgumentError, InstancePaging, TemporalSelection},
            query_string::QueryString,
        },
        query::temporal_query::TemporalQuery,
    };
    use pretty_assertions::assert_eq;
    use std::num::NonZeroU64;

    #[test]
    fn first_n_and_offset_n_are_written_after_each_other_and_only_when_given() {
        let mut query = QueryString::default();
        InstancePaging::default().push_query(&mut query);
        assert_eq!(query.to_string(), "");
        InstancePaging {
            first_n: NonZeroU64::new(2),
            offset_n: Some(4),
        }
        .push_query(&mut query);
        assert_eq!(query.to_string(), "firstN=2&offsetN=4");
    }

    #[test]
    fn first_n_beside_a_last_n_is_refused_and_beside_none_is_kept() {
        let first_two = InstancePaging {
            first_n: NonZeroU64::new(2),
            offset_n: None,
        };
        let last_three = TemporalQuery::after("2018-08-01T12:00:00Z".parse().unwrap()).with_last_n(NonZeroU64::new(3).unwrap());
        assert_eq!(first_two.check(TemporalSelection::Query(&last_three)), Err(ArgumentError::FirstNBesideLastN));
        let after = TemporalQuery::after("2018-08-01T12:00:00Z".parse().unwrap());
        assert_eq!(first_two.check(TemporalSelection::Query(&after)), Ok(()));
        let offset_only = InstancePaging {
            first_n: None,
            offset_n: Some(2),
        };
        assert_eq!(offset_only.check(TemporalSelection::Query(&last_three)), Ok(()));
    }
}
