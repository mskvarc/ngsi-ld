//! The entity type and attribute discovery operations (NGSI-LD clauses 5.7.5 to 5.7.10), bound to `types`,
//! `types/{type}`, `attributes` and `attributes/{attrId}` (clauses 6.25 to 6.28): one request type per operation,
//! and how their responses are read.

use crate::{
    http::{Endpoint, QueryParameter, RequestError, operation::parameters::Details, path::ResourcePath, request::scope::RequestScope},
    query::Locality,
};
use bytes::Bytes;
use http::Request;

pub(super) mod attribute_details;
pub(super) mod attribute_info;
pub(super) mod attributes;
pub(super) mod entity_type_details;
pub(super) mod entity_type_info;
pub(super) mod entity_types;
pub(super) mod parameters;

/// Builds the GET of the discovery resource `path`, asking for the representation `details` names, limited to
/// the broker's own data or not as `locality` says.
pub(crate) fn request(
    path: ResourcePath,
    details: Details,
    locality: Locality,
    endpoint: &Endpoint,
    scope: RequestScope<'_>,
) -> Result<Request<Bytes>, RequestError> {
    details
        .get(path, |query| query.push_some(QueryParameter::Local, locality.flag()))
        .into_request(endpoint, scope)
}
