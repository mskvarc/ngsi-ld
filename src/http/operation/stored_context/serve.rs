//! Serve @context (NGSI-LD clauses 5.13.4 and 6.30.3.1, ETSI TS 104 175 clause 13.4), as the context itself, as its
//! metadata, or as references to it and to a core context.

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            SummaryDetails,
            parameters::Details,
            sealed::{BuildsRequest, Sealed},
            stored_document::DocumentReader,
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{ConformanceVersion, Context, ContextId, ContextMetadata, DocumentError, LocalContext, NgsiLdMediaType},
};
use bytes::Bytes;
use http::{Method, Request, Response};
use jstrict::Value as Json;

/// Reads a stored context.
///
/// Its request is `GET /jsonldContexts/{contextId}` (NGSI-LD clause 6.30.3.1), answered with the `@context` member of
/// the local context document the broker serves. A context of kind `Cached` is never served: the broker answers 422
/// `OperationNotSupported` (clause 5.13.4.4).
///
/// The outcome is the `@context` member alone: a stored context is that subtree, the broker having discarded every
/// other member when it was added (clause 5.13.2.3), so a member beside it in the served document is no part of the
/// stored context and is not kept. [`LocalContext`] reads the whole document when it is needed.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, ServeContext},
///     },
///     model::ContextId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = ContextId::from(iri!("urn:ngsi-ld:context:c1"));
/// let request = ServeContext::builder()
///     .id(&id)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bon::Builder)]
pub struct ServeContext<'a> {
    /// The stored context.
    pub id: &'a ContextId,
    /// Whether `details=false` is stated or left out (Table 6.30.3.1-1, clause 5.13.4.4).
    #[builder(default)]
    pub details: SummaryDetails,
}

/// Reads the metadata of a stored context, of any kind.
///
/// Its request is `GET /jsonldContexts/{contextId}?details=true` (NGSI-LD clauses 5.13.4.4 and 6.30.3.1).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, ServeContextDetails},
///     },
///     model::ContextId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = ContextId::from(iri!("urn:ngsi-ld:context:c1"));
/// let request = ServeContextDetails { id: &id }.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1?details=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServeContextDetails<'a> {
    /// The stored context.
    pub id: &'a ContextId,
}

/// Reads the `@context` that references a stored context together with the core context of a version of the
/// specification.
///
/// Its request is `GET /jsonldContexts/{contextId}?core={major.minor}` (ETSI TS 104 176 Table 10.3.3.1-1). The broker
/// answers with a local context document whose `@context` is an array of two references: the original location of
/// the stored context and the location of the core context (ETSI TS 104 175 clause 13.4.4).
///
/// The outcome is that `@context` as served. Clause 13.4.4 fixes the two items of the array but not their order, so
/// the outcome does not tell the user reference from the core one.
///
/// Clause 13.4.4 answers a request that names both `core` and `details` with a `BadRequest`, so the operation has no
/// `details`.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, ServeContextWithCore},
///     },
///     model::{ConformanceVersion, ContextId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = ContextId::from(iri!("urn:ngsi-ld:context:c1"));
/// let request = ServeContextWithCore {
///     id: &id,
///     core: ConformanceVersion::new(1, 9),
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1?core=1.9"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServeContextWithCore<'a> {
    /// The stored context.
    pub id: &'a ContextId,
    /// The version of the specification whose core context the answer references (`core`).
    pub core: ConformanceVersion,
}

/// Builds the GET of the stored context `id` in the representation `details` names.
fn request(id: &ContextId, details: Details, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
    details.get(ResourcePath::stored_context(id), |_| ()).into_request(endpoint, scope)
}

/// Reads the context a served local context document holds.
fn read_context(json: Json) -> Result<Context, DocumentError> {
    LocalContext::from_json(json).map(|document| document.context)
}

impl Sealed for ServeContext<'_> {}

impl Operation for ServeContext<'_> {
    type Outcome = Context;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Context, ResponseError> {
        DocumentReader::document(read_context).decode_one(response)
    }
}

impl BuildsRequest for ServeContext<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(self.id, Details::Summary(self.details), endpoint, scope)
    }
}

impl Sealed for ServeContextDetails<'_> {}

impl Operation for ServeContextDetails<'_> {
    type Outcome = ContextMetadata;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ContextMetadata, ResponseError> {
        DocumentReader::document(ContextMetadata::from_json).decode_one(response)
    }
}

impl BuildsRequest for ServeContextDetails<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(self.id, Details::Detailed, endpoint, scope)
    }
}

impl Sealed for ServeContextWithCore<'_> {}

impl Operation for ServeContextWithCore<'_> {
    type Outcome = Context;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Context, ResponseError> {
        DocumentReader::document(read_context).decode_one(response)
    }
}

impl BuildsRequest for ServeContextWithCore<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push(QueryParameter::Core, self.core);
        RequestParts::resource(Method::GET, ResourcePath::stored_context(self.id), query)
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
