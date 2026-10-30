//! Decoding responses: the status code selects the outcome, and every error response is decoded the same way
//! (NGSI-LD clauses 6.3.2 to 6.3.4).

use crate::{
    http::{
        ResponseHead,
        headers::{Location, OptionalHeader},
    },
    model::{DocumentError, InvalidJson, ProblemDetails, RawJson},
    registration::Problem as RegistrationProblem,
    snapshot::Problem as SnapshotProblem,
    subscription::{Problem as SubscriptionProblem, ReadErrorKind},
};
use bytes::Bytes;
use headers::{Header, HeaderMapExt};
use http::{HeaderMap, HeaderName, Response, StatusCode};
use iri_rs::IriRefBuf;
use jstrict::{DeserializeError, Value as Json};
use mediatype::{MediaTypeBuf, MediaTypeList};
use serde::de::DeserializeOwned;
use std::str::{self, Utf8Error};

/// Why a response is not an outcome of the operation.
///
/// Every variant keeps the status and the rest of the head of the response (version, headers and the transport's
/// extensions), so a header an error carries, such as `Allow` on a 405, `Retry-After` on a 503 or the
/// `NGSILD-Tenant` the broker echoed, is readable with [`head`](Self::head).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResponseError {
    /// The broker reported a problem (clause 6.3.3).
    ///
    /// The problem is read leniently: a member that does not have its typed shape is left out of `details` rather
    /// than failing the decoding, and `body` keeps the JSON exactly as received, so nothing the broker said is lost.
    #[error("the broker answered {status} with a problem")]
    Problem {
        /// The status code.
        status: StatusCode,
        /// The rest of the head of the response, boxed to keep the error small.
        head: Box<ResponseHead>,
        /// The members of the problem that have their typed shape, boxed to keep the error small.
        details: Box<ProblemDetails>,
        /// The JSON body as received, boxed to keep the error small.
        body: Box<RawJson>,
    },
    /// The broker answered an error status without a body: 405, 411, 413 or 415 (clauses 6.3.2 and 6.3.4).
    #[error("the broker answered {status} without a body")]
    Status {
        /// The status code.
        status: StatusCode,
        /// The rest of the head of the response, boxed to keep the error small.
        head: Box<ResponseHead>,
    },
    /// The broker cannot answer in an acceptable representation (406, clause 6.3.4). The body lists the
    /// representations it offers, in a form the specification does not fix.
    ///
    /// The body is read leniently, as a comma-separated list of media types: the entries that are media types are
    /// kept in the order the broker wrote them, and every other entry is dropped. A body in another shape, such as a
    /// JSON object naming the representations in a member of its own, lists nothing in `available`; `body` keeps it as
    /// received, so nothing the broker said is lost.
    #[error("the broker answered 406: no acceptable representation")]
    NotAcceptable {
        /// The rest of the head of the response, boxed to keep the error small.
        head: Box<ResponseHead>,
        /// The media types the body lists.
        available: Vec<MediaTypeBuf>,
        /// The body as received.
        body: Bytes,
    },
    /// A success status the operation's table in clause 6 does not list.
    #[error("the broker answered {status}, which this operation does not define")]
    UnexpectedStatus {
        /// The status code.
        status: StatusCode,
        /// The rest of the head of the response, boxed to keep the error small.
        head: Box<ResponseHead>,
    },
    /// A header the outcome depends on is malformed, such as an `NGSILD-Results-Count` that is not a number
    /// (clause 6.3.13).
    #[error("the {status} response has a malformed {name} header")]
    Header {
        /// The status code.
        status: StatusCode,
        /// The rest of the head of the response, boxed to keep the error small.
        head: Box<ResponseHead>,
        /// The header.
        name: HeaderName,
    },
    /// The body does not hold what the status code says it holds.
    #[error("the {status} response body could not be read")]
    Body {
        /// The status code.
        status: StatusCode,
        /// The rest of the head of the response, boxed to keep the error small.
        head: Box<ResponseHead>,
        /// Why, boxed to keep the error small.
        #[source]
        source: Box<BodyError>,
    },
}

impl ResponseError {
    /// The head of the response besides its status: version, headers, and the extensions the transport kept.
    #[must_use]
    pub fn head(&self) -> &ResponseHead {
        match self {
            Self::Problem { head, .. }
            | Self::Status { head, .. }
            | Self::NotAcceptable { head, .. }
            | Self::UnexpectedStatus { head, .. }
            | Self::Header { head, .. }
            | Self::Body { head, .. } => head,
        }
    }

    /// The status code of the response.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::Problem { status, .. }
            | Self::Status { status, .. }
            | Self::UnexpectedStatus { status, .. }
            | Self::Header { status, .. }
            | Self::Body { status, .. } => *status,
            Self::NotAcceptable { .. } => StatusCode::NOT_ACCEPTABLE,
        }
    }
}

/// Why a response body could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BodyError {
    /// The body is not UTF-8.
    #[error("the body is not UTF-8")]
    Utf8(#[from] Utf8Error),
    /// The body is not JSON.
    #[error("the body is not JSON")]
    Json(#[from] InvalidJson),
    /// The JSON does not have the shape the status code promises.
    #[error("the body does not have the expected shape")]
    Shape(#[from] ShapeError),
    /// The body is not the JSON array of entities a query answers with.
    #[error("the body is not a JSON array")]
    NotAnArray,
    /// The JSON is not the NGSI-LD document the status code promises.
    #[error("the body is not a valid NGSI-LD document")]
    Document(#[from] DocumentError),
    /// The JSON is not the subscription, or the list of subscriptions, the status code promises (clause 5.2.12).
    #[error("the body is not a valid subscription")]
    Subscription(#[source] DocumentError<ReadErrorKind<SubscriptionProblem>>),
    /// The JSON is not the context source registration, or the list of registrations, the status code promises
    /// (clause 5.2.9).
    #[error("the body is not a valid context source registration")]
    Registration(#[source] DocumentError<ReadErrorKind<RegistrationProblem>>),
    /// The JSON is not the snapshot the status code promises (clause 5.2.41).
    #[error("the body is not a valid snapshot")]
    Snapshot(#[source] DocumentError<ReadErrorKind<SnapshotProblem>>),
}

/// Why a JSON body is not the typed result its status code promises.
///
/// One such result is the `BatchOperationResult` of NGSI-LD clause 5.2.16. The error displays the member that is
/// missing or of the wrong type.
///
/// The deserializer's error type is not part of the crate's API: a [`ShapeError`] carries its message.
#[derive(Debug, Clone, thiserror::Error)]
#[error(transparent)]
pub struct ShapeError(DeserializeError);

/// Reads a body that is a JSON array, each element with `read`; `error` turns the error of an element, placed under
/// its index, into the body error.
pub(crate) fn read_array<T, K>(
    json: Json,
    mut read: impl FnMut(Json) -> Result<T, DocumentError<K>>,
    error: impl Fn(DocumentError<K>) -> BodyError,
) -> Result<Vec<T>, BodyError> {
    match json {
        Json::Array(items) => items
            .into_iter()
            .enumerate()
            .map(|(index, item)| read(item).map_err(|source| error(source.at_index(index))))
            .collect(),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_) => Err(BodyError::NotAnArray),
    }
}

/// A response whose status is a success, ready for the operation to read.
#[derive(Debug)]
pub(crate) struct Success(pub(crate) Response<Bytes>);

impl Success {
    /// The status code.
    pub(crate) fn status(&self) -> StatusCode {
        self.0.status()
    }

    /// The error for a body that does not hold what the status code says it holds.
    pub(crate) fn body_error(&self, source: impl Into<BodyError>) -> ResponseError {
        ResponseError::Body {
            status: self.status(),
            head: Box::new(ResponseHead::of(&self.0)),
            source: Box::new(source.into()),
        }
    }

    /// Reads the body as JSON.
    pub(crate) fn json(&self) -> Result<Json, ResponseError> {
        body_json(self.0.body()).map_err(|source| self.body_error(source))
    }

    /// Reads the body as a typed value.
    pub(crate) fn deserialize<T: DeserializeOwned>(&self) -> Result<T, ResponseError> {
        jstrict::from_value(self.json()?).map_err(|source| self.body_error(ShapeError(source)))
    }

    /// The headers.
    pub(crate) fn headers(&self) -> &HeaderMap {
        self.0.headers()
    }

    /// Reads a typed header the outcome depends on, if present; a malformed one fails the response.
    pub(crate) fn header<H: Header>(&self) -> Result<Option<H>, ResponseError> {
        self.0.headers().typed_try_get::<H>().map_err(|_| ResponseError::Header {
            status: self.status(),
            head: Box::new(ResponseHead::of(&self.0)),
            // The error owns the name; a header name is a static or shared value, so cloning it is cheap.
            name: H::name().clone(),
        })
    }

    /// The `Location` of a created resource (clause 6.3.2), read leniently: the resource exists whatever the
    /// header holds, so a malformed one never fails the outcome.
    pub(crate) fn location(&self) -> OptionalHeader<IriRefBuf> {
        OptionalHeader::<Location>::read(self.headers()).map(Location::into_iri_ref)
    }

    /// The error for a success status the operation does not define.
    pub(crate) fn unexpected(&self) -> ResponseError {
        ResponseError::UnexpectedStatus {
            status: self.status(),
            head: Box::new(ResponseHead::of(&self.0)),
        }
    }
}

/// Parses a body as JSON.
fn body_json(body: &Bytes) -> Result<Json, BodyError> {
    str::from_utf8(body)?.parse::<Json>().map_err(|error| InvalidJson::of(&error).into())
}

/// Separates success from failure. A 1xx, 2xx or 3xx status is a success for the operation to read; every
/// error status is decoded here, once for all operations.
pub(crate) fn classify(response: Response<Bytes>) -> Result<Success, ResponseError> {
    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return Ok(Success(response));
    }
    let (parts, body) = response.into_parts();
    let head = Box::new(ResponseHead::from_parts(parts));
    if status == StatusCode::NOT_ACCEPTABLE {
        return Err(ResponseError::NotAcceptable {
            head,
            available: available_representations(&body),
            body,
        });
    }
    if body.iter().all(u8::is_ascii_whitespace) {
        return Err(ResponseError::Status { status, head });
    }
    match body_json(&body) {
        Ok(json) => Err(ResponseError::Problem {
            status,
            head,
            details: Box::new(ProblemDetails::from_json(&json)),
            body: Box::new(RawJson::from_json(json)),
        }),
        Err(source) => Err(ResponseError::Body {
            status,
            head,
            source: Box::new(source),
        }),
    }
}

/// The media types a 406 body lists, read leniently: an entry that is not a media type is dropped.
fn available_representations(body: &[u8]) -> Vec<MediaTypeBuf> {
    MediaTypeList::new(String::from_utf8_lossy(body).trim())
        .filter_map(Result::ok)
        .map(MediaTypeBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{
            BodyError,
            ResponseError,
            response::{Success, classify},
        },
        model::{ErrorType, ProblemDetails},
    };
    use bytes::Bytes;
    use http::{Response, StatusCode};
    use mediatype::MediaTypeBuf;
    use pretty_assertions::assert_eq;

    fn raw_response(status: u16, body: &'static [u8]) -> Response<Bytes> {
        let mut response = Response::new(Bytes::from_static(body));
        *response.status_mut() = StatusCode::from_u16(status).unwrap();
        response
    }

    fn response(status: u16, body: &'static str) -> Response<Bytes> {
        raw_response(status, body.as_bytes())
    }

    fn available(body: &'static [u8]) -> Vec<String> {
        let ResponseError::NotAcceptable { available, .. } = classify(raw_response(406, body)).unwrap_err() else {
            panic!("expected a 406")
        };
        available.iter().map(MediaTypeBuf::as_str).map(str::to_owned).collect()
    }

    #[test]
    fn a_409_with_an_already_exists_problem_decodes_to_that_error_type() {
        let error = classify(response(
            409,
            r#"{"type": "https://uri.etsi.org/ngsi-ld/errors/AlreadyExists", "title": "Already exists", "detail": "urn:ngsi-ld:Vehicle:A4567"}"#,
        ))
        .unwrap_err();
        assert!(matches!(error, ResponseError::Problem { status: StatusCode::CONFLICT, details, .. } if details.error_type == Some(ErrorType::AlreadyExists)));
    }

    #[test]
    fn a_400_with_an_unknown_problem_type_keeps_its_url() {
        let error = classify(response(400, r#"{"type": "https://broker.example/errors/Unusual", "title": "Unusual"}"#)).unwrap_err();
        assert!(
            matches!(error, ResponseError::Problem { details, .. } if details.error_type.as_ref().map(ToString::to_string) == Some("https://broker.example/errors/Unusual".to_owned()))
        );
    }

    #[test]
    fn a_415_without_a_body_is_a_bare_status() {
        assert!(matches!(
            classify(response(415, "")).unwrap_err(),
            ResponseError::Status {
                status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
                ..
            }
        ));
    }

    #[test]
    fn a_problem_with_members_out_of_shape_keeps_the_readable_ones_and_the_raw_body() {
        let body = r#"{"type": "errors/Unusual", "title": "Unusual", "status": "418", "instance": "a b", "detail": 7}"#;
        let ResponseError::Problem {
            status, details, body: raw, ..
        } = classify(response(418, body)).unwrap_err()
        else {
            panic!("expected a problem")
        };
        assert_eq!(status.as_u16(), 418);
        assert_eq!(details.error_type.as_ref().map(ToString::to_string), Some("errors/Unusual".to_owned()));
        assert!(matches!(details.error_type, Some(ErrorType::Other(_))));
        assert_eq!(details.title.as_deref(), Some("Unusual"));
        assert_eq!(details.status, Some(418));
        assert_eq!(details.instance, None);
        assert_eq!(details.detail, None);
        assert_eq!(
            raw.to_string(),
            r#"{"type":"errors/Unusual","title":"Unusual","status":"418","instance":"a b","detail":7}"#
        );
    }

    #[test]
    fn an_error_body_that_is_json_but_not_an_object_is_a_problem_without_members() {
        let ResponseError::Problem { details, body, .. } = classify(response(500, r#"["oops"]"#)).unwrap_err() else {
            panic!("expected a problem")
        };
        assert_eq!(*details, ProblemDetails::default());
        assert_eq!(body.to_string(), r#"["oops"]"#);
    }

    #[test]
    fn a_406_lists_every_available_media_type_in_order() {
        assert_eq!(available(b"application/json, application/ld+json"), ["application/json", "application/ld+json"]);
    }

    #[test]
    fn a_406_drops_an_entry_that_is_not_a_media_type() {
        assert_eq!(available(b"text/html garbage, application/json"), ["application/json"]);
    }

    #[test]
    fn a_406_ignores_surrounding_whitespace() {
        assert_eq!(available(b"application/json\n"), ["application/json"]);
    }

    #[test]
    fn a_406_with_an_empty_body_lists_nothing() {
        assert_eq!(available(b""), Vec::<String>::new());
    }

    #[test]
    fn a_406_drops_an_entry_that_is_not_utf8() {
        assert_eq!(available(b"application/json, \xFF/x"), ["application/json"]);
    }

    #[test]
    fn a_406_whose_body_is_not_utf8_lists_nothing() {
        assert_eq!(available(b"\xFF\xFE"), Vec::<String>::new());
    }

    #[test]
    fn a_406_keeps_its_body_as_received_whatever_shape_it_has() {
        let json: &'static [u8] = br#"{"title":"Not Acceptable","availableRepresentations":["application/json","application/ld+json"]}"#;
        let ResponseError::NotAcceptable { available, body, .. } = classify(raw_response(406, json)).unwrap_err() else {
            panic!("expected a 406")
        };
        assert_eq!(available, Vec::<MediaTypeBuf>::new());
        assert_eq!(body.as_ref(), json);
        let ResponseError::NotAcceptable { body, .. } = classify(raw_response(406, b"text/html garbage, application/json")).unwrap_err() else {
            panic!("expected a 406")
        };
        assert_eq!(body.as_ref(), b"text/html garbage, application/json");
    }

    #[test]
    fn an_error_body_that_is_not_json_is_a_body_error_with_its_status() {
        assert!(matches!(
            classify(response(502, "<html>bad gateway</html>")).unwrap_err(),
            ResponseError::Body { status: StatusCode::BAD_GATEWAY, source, .. } if matches!(*source, BodyError::Json(json) if json.position() == 0)
        ));
    }

    #[test]
    fn a_success_body_out_of_shape_is_a_shape_error_that_displays_the_mismatch() {
        let ResponseError::Body { source, .. } = Success(response(200, r#"{"a":1}"#)).deserialize::<Vec<u8>>().unwrap_err() else {
            panic!("expected a body error")
        };
        let BodyError::Shape(shape) = *source else { panic!("expected a shape error") };
        assert_eq!(shape.to_string(), "invalid type: map, expected a sequence");
    }

    #[test]
    fn a_success_status_passes_through() {
        assert_eq!(classify(response(204, "")).unwrap().status(), StatusCode::NO_CONTENT);
    }
}
