//! Entity ordering in a request (NGSI-LD clauses 4.23 and 5.2.43): the `orderBy`, `collation`, `orderFrom` and
//! `orderGeometry` parameters of Query Entities (Table 6.4.3.2-1); its `Query` body carries the `ordering` member
//! [`EntityOrdering`] writes itself.

use crate::{
    http::{
        QueryParameter,
        operation::{ParameterError, received_parameters::ReceivedQuery},
        query_string::QueryString,
    },
    query::ordering::{EntityOrdering, OrderingParamsMember},
};
use jstrict::{Object, Print, Value as Json};

/// Adds the parameters to a query string. `orderGeometry` is left out for a point, the default (Table 5.2.43-1),
/// as clause 4.23.3 example 8 writes it.
pub(crate) fn push_query(query: &mut QueryString, ordering: &EntityOrdering) {
    query.push_order_by(QueryParameter::OrderBy, ordering.order_by());
    query.push_some(QueryParameter::Collation, ordering.collation());
    if let Some(origin) = ordering.origin() {
        if !origin.is_point() {
            query.push(QueryParameter::OrderGeometry, origin.type_name());
        }
        query.push(QueryParameter::OrderFrom, origin.coordinates_json().compact_print());
    }
}

/// Reads the parameters of a received query string, `None` when there is no `orderBy`. They are the members of the
/// `OrderingParams` object of clause 5.2.43 under other names (`orderFrom` its `coordinates`, `orderGeometry` its
/// `geometry`), so they are read by its reader: one grammar, one set of rules.
///
/// Without `orderBy` there is nothing to order by. The tables of the `GET` bindings list `collation`, `orderFrom`
/// and `orderGeometry` as optional parameters tied to no other (Table 6.4.3.2-1), so a query string that holds them
/// alone is not malformed: it asks for no ordering, and they are not read.
///
/// # Errors
///
/// Returns [`ParameterError::CoordinatesNotJson`] for an `orderFrom` that is not JSON, and
/// [`ParameterError::Ordering`] when the parameters do not make an ordering.
pub(crate) fn read_query(received: &ReceivedQuery<'_>) -> Result<Option<EntityOrdering>, ParameterError> {
    let Some(order_by) = received.text(QueryParameter::OrderBy)? else {
        return Ok(None);
    };
    let mut object = Object::new();
    object.push(
        <&str>::from(OrderingParamsMember::OrderBy).into(),
        Json::Array(vec![Json::from(order_by.into_owned())].into()),
    );
    if let Some(collation) = received.text(QueryParameter::Collation)? {
        object.push(<&str>::from(OrderingParamsMember::Collation).into(), Json::from(collation.into_owned()));
    }
    if let Some(geometry) = received.text(QueryParameter::OrderGeometry)? {
        object.push(<&str>::from(OrderingParamsMember::Geometry).into(), Json::from(geometry.into_owned()));
    }
    if let Some(origin) = received.text(QueryParameter::OrderFrom)? {
        let coordinates = origin.parse::<Json>().map_err(|_| ParameterError::CoordinatesNotJson {
            parameter: QueryParameter::OrderFrom,
        })?;
        object.push(<&str>::from(OrderingParamsMember::Coordinates).into(), coordinates);
    }
    EntityOrdering::from_json(Json::Object(object))
        .map(Some)
        .map_err(|source| ParameterError::Ordering { source })
}
