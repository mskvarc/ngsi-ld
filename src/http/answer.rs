//! The answering side of the HTTP binding (NGSI-LD clauses 6.3.2, 6.3.3 and 6.3.6): the responses a context source
//! writes for the requests a broker forwards to it, the mirror of the response decoding of
//! every [`Operation`](crate::http::operation::Operation).
//!
//! The crate implements no server; a context source reads a forwarded request with the received-parameter
//! decoders of [`operation`](crate::http::operation), builds the documents of its answer with the data model, and
//! hands the response built here to its own HTTP server. Every response is `application/json`: a body that holds
//! NGSI-LD terms carries the `Link` to its `@context` beside it, and a problem or a partial success carries none,
//! since its names are fully qualified (clause 6.3.6).
//!
//! ```
//! use http::header;
//! use headers::HeaderMapExt;
//! use http::StatusCode;
//! use iri_rs::iri;
//! use ngsi_ld::{http::{answer, headers::{Link, NgsiLdResultsCount}}, model::{Entity, Representation}};
//!
//! let vehicle = Entity::builder().id(iri!("urn:ngsi-ld:Vehicle:A4567")).entity_type("Vehicle").property("speed", 80).build()?;
//! let mut response = answer::list([vehicle.render(Representation::Normalized)], &Link::try_core_context()?);
//! response.headers_mut().typed_insert(NgsiLdResultsCount::new(1));
//! assert_eq!(response.status(), StatusCode::OK);
//! assert_eq!(response.headers()[&header::CONTENT_TYPE], "application/json");
//! assert_eq!(response.body().as_ref(), br#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80}}]"#);
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Success
//!
//! [`document`], [`list`], [`created`], [`no_content`], [`update_result`], [`batch_result`].
//!
//! # Errors
//!
//! [`problem`], [`error_status`].

use crate::{
    http::{
        BatchOperationResult,
        UpdateResult,
        headers::{ContentType, Link, Location},
    },
    model::{ErrorType, NgsiLdMediaType, ProblemDetails, RawJson},
};
use bytes::Bytes;
use headers::HeaderMapExt;
use http::{Response, StatusCode};

/// A response with `status` and `body` as `application/json`.
fn json(status: StatusCode, body: &RawJson) -> Response<Bytes> {
    let mut response = Response::new(Bytes::from(body.to_string()));
    *response.status_mut() = status;
    response.headers_mut().typed_insert(ContentType::NgsiLd(NgsiLdMediaType::Json));
    response
}

/// Answers 200 with `body`, a document holding NGSI-LD terms (an entity, a temporal evolution, a registration, ...),
/// and the `Link` to the `@context` those terms are compacted with (clause 6.3.6).
#[must_use]
pub fn document(body: &RawJson, context: &Link) -> Response<Bytes> {
    let mut response = json(StatusCode::OK, body);
    // The response owns its header; the caller keeps the link for its next answer.
    response.headers_mut().typed_insert(context.clone());
    response
}

/// Answers 200 with the JSON array of `items`, the body of a query (clauses 6.4.3.2 and 6.3.10), and the `Link` to
/// the `@context` their terms are compacted with (clause 6.3.6). The caller adds the `NGSILD-Results-Count` a
/// query asked for with `count` (clause 6.3.13) and the `Link` to the next page (clause 6.3.10) as typed headers.
#[must_use]
pub fn list(items: impl IntoIterator<Item = RawJson>, context: &Link) -> Response<Bytes> {
    document(&RawJson::array(items), context)
}

/// Answers 201 with no body and the `Location` of the created resource, as Create Entity and the other creations
/// answer (clause 6.4.3.1); a response without a body carries no `Link` (clause 6.3.6).
#[must_use]
pub fn created(location: Location) -> Response<Bytes> {
    let mut response = Response::new(Bytes::new());
    *response.status_mut() = StatusCode::CREATED;
    response.headers_mut().typed_insert(location);
    response
}

/// Answers 204 with no body, as an operation that changed everything it was asked to answers (clause 6.6.3.1 and
/// the others); it carries no `Link` (clause 6.3.6) and, being empty, no `Content-Type`.
#[must_use]
pub fn no_content() -> Response<Bytes> {
    let mut response = Response::new(Bytes::new());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response
}

/// Answers 207 with `result`, the attributes an attribute operation changed and those it did not (clause 5.2.18).
#[must_use]
pub fn update_result(result: &UpdateResult) -> Response<Bytes> {
    json(StatusCode::MULTI_STATUS, &result.render())
}

/// Answers 207 with `result`, the entities a batch or distributed operation treated and those in error (clause
/// 5.2.16).
#[must_use]
pub fn batch_result(result: &BatchOperationResult) -> Response<Bytes> {
    json(StatusCode::MULTI_STATUS, &result.render())
}

/// Answers `status` with `details` (clause 6.3.3), whose `status` member is set to `status`: the member only repeats
/// the status of the response (IETF RFC 9457 section 3.1.2), so the two never disagree. [`error_status`] gives the
/// status Table 6.3.2-1 maps an error type to; a status outside the table, such as 501 for an operation a context
/// source does not implement, is the caller's.
///
/// ```
/// use http::{StatusCode, header};
/// use ngsi_ld::{
///     http::answer,
///     model::{ErrorType, ProblemDetails},
/// };
///
/// let problem = ProblemDetails::builder()
///     .error_type(ErrorType::ResourceNotFound)
///     .title("Not found")
///     .detail("urn:ngsi-ld:Vehicle:B9211")
///     .build();
/// assert_eq!(
///     answer::error_status(&ErrorType::ResourceNotFound),
///     Some(StatusCode::NOT_FOUND)
/// );
/// let response = answer::problem(StatusCode::NOT_FOUND, problem);
/// assert_eq!(response.status(), StatusCode::NOT_FOUND);
/// assert!(response.headers().get(&header::LINK).is_none());
/// ```
#[must_use]
pub fn problem(status: StatusCode, mut details: ProblemDetails) -> Response<Bytes> {
    details.status = Some(status.as_u16());
    json(status, &details.render())
}

/// The status Table 6.3.2-1 maps `error_type` to, `None` for a type outside the table: `Conflict`, which Table
/// 5.5.2-1 defines and Table 6.3.2-1 does not map, and every [`ErrorType::Other`].
#[must_use]
pub const fn error_status(error_type: &ErrorType) -> Option<StatusCode> {
    match error_type {
        ErrorType::AlreadyExists => Some(StatusCode::CONFLICT),
        ErrorType::BadRequestData | ErrorType::InvalidRequest => Some(StatusCode::BAD_REQUEST),
        ErrorType::InternalError => Some(StatusCode::INTERNAL_SERVER_ERROR),
        ErrorType::LdContextNotAvailable => Some(StatusCode::GATEWAY_TIMEOUT),
        ErrorType::NoMultiTenantSupport => Some(StatusCode::NOT_IMPLEMENTED),
        ErrorType::NonexistentTenant | ErrorType::ResourceNotFound => Some(StatusCode::NOT_FOUND),
        ErrorType::OperationNotSupported => Some(StatusCode::UNPROCESSABLE_ENTITY),
        ErrorType::TooComplexQuery | ErrorType::TooManyResults => Some(StatusCode::FORBIDDEN),
        ErrorType::Conflict | ErrorType::Other(_) => None,
    }
}

#[cfg(test)]
mod tests;
