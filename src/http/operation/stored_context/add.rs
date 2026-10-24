//! Add @context (NGSI-LD clauses 5.13.2 and 6.29.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResourceCreated,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{Context, ContextId, ExtraMemberName, LocalContext, Value},
};
use bytes::Bytes;
use http::{Method, Request, Response};
use indexmap::IndexMap;

/// Stores a context at the broker, which serves it from then on.
///
/// Its request is `POST /jsonldContexts` (NGSI-LD clause 6.29.3.1).
///
/// The body is `{"@context": <context>}`, followed by the members of `extra_members` in order, sent as
/// `application/ld+json` with no `Link` header (clause 6.3.5). The broker discards every member beside `@context`
/// (clause 5.13.2.3). It names the stored context in the `Location` of its 201, which [`ResourceCreated::id`] reads.
///
/// ```
/// use indexmap::IndexMap;
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{AddContext, Operation},
///     },
///     model::{Context, Value},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let context = Context::from(IriRefBuf::new("https://example.org/context.jsonld")?);
/// let notes = IndexMap::from([("note".parse()?, Value::from("ignored by the broker"))]);
/// let add = AddContext::builder()
///     .context(&context)
///     .extra_members(&notes)
///     .build();
/// let request = add.to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.body().as_ref(),
///     br#"{"@context":"https://example.org/context.jsonld","note":"ignored by the broker"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, bon::Builder)]
pub struct AddContext<'a> {
    /// The context to store.
    pub context: &'a Context,
    /// The members sent beside `@context`, which the broker discards (clause 5.13.2.3); `None` sends none.
    pub extra_members: Option<&'a IndexMap<ExtraMemberName, Value>>,
}

impl Sealed for AddContext<'_> {}

impl Operation for AddContext<'_> {
    type Outcome = ResourceCreated<ContextId>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ResourceCreated<ContextId>, ResponseError> {
        ResourceCreated::decode(&classify(response)?)
    }
}

impl BuildsRequest for AddContext<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::contexts(), QueryString::default())
            .with_local_context(LocalContext::object(self.context, self.extra_members.into_iter().flatten()))
            .into_request(endpoint, scope)
    }
}
