//! Delete and Reload @context (NGSI-LD clauses 5.13.5 and 6.30.3.2).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            no_content::decode_no_content,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::ContextId,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// What Delete and Reload @context does with a stored context (NGSI-LD clause 5.13.5.4, the `reload` parameter of
/// Table 6.30.3.2-1): removes it, with `reload` left out or stated `false`, or reloads it.
///
/// The enum is exhaustive: a boolean parameter is absent, `false` or `true`.
///
/// ```
/// use ngsi_ld::http::operation::ContextDeletion;
///
/// assert_eq!(ContextDeletion::default(), ContextDeletion::Remove);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContextDeletion {
    /// Removes the context from the broker's storage, with no `reload` parameter, `false` being its default.
    #[default]
    Remove,
    /// `reload=false`: removes the context, as [`Remove`](Self::Remove) does, stating the parameter.
    RemoveExplicitly,
    /// `reload=true`: downloads a `Cached` context again from its original URL and replaces the stored copy; a
    /// failed download answers 504 `LdContextNotAvailable` and keeps the copy, and any other kind answers
    /// `BadRequestData`.
    Reload,
}

impl ContextDeletion {
    /// The `reload` parameter value.
    const fn parameter(self) -> Option<bool> {
        match self {
            Self::Remove => None,
            Self::RemoveExplicitly => Some(false),
            Self::Reload => Some(true),
        }
    }
}

/// `DELETE /jsonldContexts/{contextId}`: removes or reloads a stored context (NGSI-LD clause 6.30.3.2).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{ContextDeletion, DeleteContext, Operation},
///     },
///     model::ContextId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = ContextId::from(iri!("urn:ngsi-ld:context:c1"));
/// let reload = DeleteContext::builder()
///     .id(&id)
///     .deletion(ContextDeletion::Reload)
///     .build();
/// let request = reload.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1?reload=true"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bon::Builder)]
pub struct DeleteContext<'a> {
    /// The stored context: its local identifier, or for a `Cached` context the URL it was downloaded from.
    pub id: &'a ContextId,
    /// Whether the context is removed or reloaded.
    #[builder(default)]
    pub deletion: ContextDeletion,
}

impl Sealed for DeleteContext<'_> {}

impl Operation for DeleteContext<'_> {
    type Outcome = ();

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for DeleteContext<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        query.push_some(QueryParameter::Reload, self.deletion.parameter());
        RequestParts::resource(Method::DELETE, ResourcePath::stored_context(self.id), query).into_request(endpoint, scope)
    }
}
