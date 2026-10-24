//! The batch operations (NGSI-LD clauses 5.6.7 to 5.6.10 and 5.6.20), each bound to `POST` on a sub-resource of
//! `entityOperations` (clauses 6.14 to 6.17 and 6.31): one request type per operation, and what their requests
//! share.

use crate::http::{
    RequestError,
    operation::ArgumentError,
    path::{BatchResource, ResourcePath},
    query_string::QueryString,
    request::RequestParts,
};
use http::Method;
use jstrict::Object;

pub(super) mod create;
pub(super) mod delete;
pub(super) mod merge;
pub(super) mod parameters;
pub(super) mod update;
pub(super) mod upsert;

/// The elements of a batch, refused when there are none (clauses 5.6.7.4 to 5.6.10.4 and 5.6.20).
const fn non_empty<T>(elements: &[T]) -> Result<&[T], ArgumentError> {
    if elements.is_empty() { Err(ArgumentError::EmptyBatch) } else { Ok(elements) }
}

/// A `POST` to the batch sub-resource `resource`, with no body yet.
fn post(resource: BatchResource, query: QueryString) -> RequestParts {
    RequestParts::resource(Method::POST, ResourcePath::batch(resource), query)
}

/// A `POST` to the batch sub-resource `resource` whose body is the array of `elements`, each written by `write`,
/// in the order given.
fn documents<T>(resource: BatchResource, query: QueryString, elements: &[T], write: impl Fn(&T) -> Object) -> Result<RequestParts, RequestError> {
    Ok(post(resource, query).with_documents(non_empty(elements)?.iter().map(write).collect()))
}
