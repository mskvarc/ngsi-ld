//! A geoquery in a request (NGSI-LD clauses 4.10 and 5.2.13): the `georel`, `geometry`, `coordinates` and
//! `geoproperty` parameters of Query Entities and Purge Entities (Tables 6.4.3.2-1 and 6.4.3.3-1), or the `geoQ`
//! member of a `Query` body.

use crate::{
    http::{
        QueryParameter,
        operation::{ParameterError, received_parameters::ReceivedQuery},
        query_string::QueryString,
    },
    query::geoquery::{GeoQuery, GeoQueryMember},
};
use jstrict::{Object, Print, Value as Json};

/// Adds the parameters to a query string: `coordinates` is the compact JSON of the reference geometry's
/// coordinates, as clause 4.10 example 1 writes it (`coordinates=[8,40]`).
pub(crate) fn push_query(query: &mut QueryString, geoquery: &GeoQuery) {
    query.push_geo_relation(QueryParameter::Georel, geoquery.relation());
    query.push(QueryParameter::Geometry, geoquery.geometry().type_name());
    query.push(QueryParameter::Coordinates, geoquery.geometry().coordinates_json().compact_print());
    query.push_some(QueryParameter::Geoproperty, geoquery.geoproperty());
}

/// Reads the geoquery of a received query string, `None` when neither `georel`, `geometry` nor `coordinates` is
/// given; a `geoproperty` alone is no geoquery, and Table 6.4.3.2-1 says it is ignored then. The four parameters are
/// the members of the `GeoQuery` object of clause 5.2.13, whose `coordinates` may be a string holding JSON (clause
/// 4.7.2), so they are read by its reader: one grammar, one set of rules.
///
/// # Errors
///
/// Returns [`ParameterError::GeoQuery`] when the parameters do not make a geoquery: Table 6.4.3.2-1 makes each of
/// `georel`, `geometry` and `coordinates` mandatory beside the others.
pub(crate) fn read_query(received: &ReceivedQuery<'_>) -> Result<Option<GeoQuery>, ParameterError> {
    let bounding = [QueryParameter::Georel, QueryParameter::Geometry, QueryParameter::Coordinates];
    if !bounding.into_iter().any(|parameter| received.contains(parameter)) {
        return Ok(None);
    }
    let members = [
        (QueryParameter::Georel, GeoQueryMember::Georel),
        (QueryParameter::Geometry, GeoQueryMember::Geometry),
        (QueryParameter::Coordinates, GeoQueryMember::Coordinates),
        (QueryParameter::Geoproperty, GeoQueryMember::Geoproperty),
    ];
    let mut object = Object::new();
    for (parameter, member) in members {
        if let Some(text) = received.text(parameter)? {
            object.push(<&str>::from(member).into(), Json::from(text.into_owned()));
        }
    }
    GeoQuery::from_json(Json::Object(object))
        .map(Some)
        .map_err(|source| ParameterError::GeoQuery { source })
}
