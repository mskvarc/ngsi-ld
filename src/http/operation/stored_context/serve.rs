//! Serve @context (NGSI-LD clauses 5.13.4 and 6.30.3.1), as the context itself or as its metadata.

use crate::{
    http::{
        Endpoint,
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
        request::scope::RequestScope,
    },
    model::{Context, ContextId, ContextMetadata, DocumentError, LocalContext},
};
use bytes::Bytes;
use http::{Request, Response};
use jstrict::Value as Json;

/// `GET /jsonldContexts/{contextId}`: a stored context, as the `@context` member of the local context document the
/// broker serves (NGSI-LD clause 6.30.3.1). A context of kind `Cached` is never served: the broker answers 422
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

/// `GET /jsonldContexts/{contextId}?details=true`: the metadata of a stored context, of any kind (NGSI-LD clauses
/// 5.13.4.4 and 6.30.3.1).
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
