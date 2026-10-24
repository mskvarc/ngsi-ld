//! Reading the answers that carry documents other than entities: subscriptions (clause 5.2.12), context source
//! registrations (clause 5.2.9), the discovery documents (clauses 5.2.24 to 5.2.28) and the stored `@context`s
//! (clause 5.13), one at a time, as a list, or as the page of a query.

use crate::{
    http::{
        BodyError,
        Page,
        ResponseContext,
        ResponseError,
        response::{Success, classify, read_array},
    },
    model::{DocumentError, DocumentErrorKind},
};
use bytes::Bytes;
use http::{Response, StatusCode};
use jstrict::Value as Json;

/// How one kind of document is read from JSON, and which body error names it; `K` is the error kind of its
/// reader.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DocumentReader<T, K> {
    /// Reads one document.
    pub(crate) read: fn(Json) -> Result<T, DocumentError<K>>,
    /// The body error of a document that could not be read.
    pub(crate) error: fn(DocumentError<K>) -> BodyError,
}

impl<T> DocumentReader<T, DocumentErrorKind> {
    /// Reads a `model` document with `read`, strictly: a document it cannot read fails the response with
    /// [`BodyError::Document`].
    pub(crate) const fn document(read: fn(Json) -> Result<T, DocumentError>) -> Self {
        Self {
            read,
            error: BodyError::Document,
        }
    }
}

impl<T, K> DocumentReader<T, K> {
    /// Reads a body that is one document.
    pub(crate) fn one(self, response: &Success) -> Result<T, ResponseError> {
        (self.read)(response.json()?).map_err(|source| response.body_error((self.error)(source)))
    }

    /// Reads a body that is a list of documents; an error names the index of the document at fault.
    pub(crate) fn list(self, response: &Success) -> Result<Vec<T>, ResponseError> {
        read_array(response.json()?, self.read, self.error).map_err(|source| response.body_error(source))
    }

    /// Reads the answer of a retrieval: 200 only (Tables 6.9.3.1-1, 6.11.3.1-1, 6.13.3.1-1, 6.25.3.1-2 to
    /// 6.28.3.1-1 and 6.30.3.1-2).
    pub(crate) fn decode_one(self, response: Response<Bytes>) -> Result<T, ResponseError> {
        let response = classify(response)?;
        match response.status() {
            StatusCode::OK => self.one(&response),
            _ => Err(response.unexpected()),
        }
    }

    /// Reads the answer of a retrieval whose body is a list of documents: 200 only (Tables 6.25.3.1-2,
    /// 6.27.3.1-2 and 6.29.3.2-2).
    pub(crate) fn decode_list(self, response: Response<Bytes>) -> Result<Vec<T>, ResponseError> {
        let response = classify(response)?;
        match response.status() {
            StatusCode::OK => self.list(&response),
            _ => Err(response.unexpected()),
        }
    }

    /// Reads a page of a query: 200 only (Tables 6.8.3.2-2, 6.10.3.2-2 and 6.12.3.2-2).
    pub(crate) fn decode_page(self, context: &ResponseContext, response: Response<Bytes>) -> Result<Page<T>, ResponseError> {
        Page::decode(context, response, &[StatusCode::OK], |response| Ok((self.list(response)?, None)))
    }
}
