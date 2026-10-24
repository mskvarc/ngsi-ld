//! List @contexts (NGSI-LD clauses 5.13.3 and 6.29.3.2), as a list of URLs or with the metadata of each.

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
        request::scope::RequestScope,
    },
    model::{ContextMetadata, DocumentError, reader::url},
};
use bytes::Bytes;
use http::{Request, Response};
use jstrict::Value as Json;
use url::Url;

/// The kind of stored contexts a List @contexts request asks for.
///
/// It is the `kind` parameter of NGSI-LD Table 6.29.3.2-1, which "can be either `Cached`, `Hosted`, or
/// `ImplicitlyCreated`". A context a broker reports may be of another kind
/// ([`StoredContextKind`](crate::model::StoredContextKind) reads it), but a request filters on these three only.
///
/// The enum is exhaustive: Table 6.29.3.2-1 closes the set.
///
/// ```
/// use ngsi_ld::http::operation::ContextKindFilter;
///
/// assert_eq!(
///     ContextKindFilter::ImplicitlyCreated.to_string(),
///     "ImplicitlyCreated"
/// );
/// assert_eq!(
///     "Hosted".parse::<ContextKindFilter>()?,
///     ContextKindFilter::Hosted
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum ContextKindFilter {
    /// `Cached`: contexts the broker downloaded and keeps as a cache.
    Cached,
    /// `Hosted`: contexts added with Add @context.
    Hosted,
    /// `ImplicitlyCreated`: contexts the broker created for a request.
    ImplicitlyCreated,
}

/// Lists the URLs of the contexts the broker stores.
///
/// Its request is `GET /jsonldContexts` (NGSI-LD clause 6.29.3.2). Each URL downloads the context, and for a `Cached`
/// one it is the URL the broker downloaded it from (clause 5.13.3.1).
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{ContextKindFilter, ListContexts, Operation},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let hosted = ListContexts::builder()
///     .kind(ContextKindFilter::Hosted)
///     .build();
/// let request = hosted.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/jsonldContexts?kind=Hosted"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct ListContexts {
    /// The kind of contexts listed; every kind when `None` (clause 5.13.3.4).
    pub kind: Option<ContextKindFilter>,
    /// Whether `details=false` is stated or left out (Table 6.29.3.2-1).
    #[builder(default)]
    pub details: SummaryDetails,
}

/// Lists the metadata of the contexts the broker stores.
///
/// Its request is `GET /jsonldContexts?details=true` (NGSI-LD clauses 5.13.3.5 and 6.29.3.2).
///
/// ```
/// use ngsi_ld::http::{
///     Endpoint,
///     operation::{ContextKindFilter, ListContextDetails, Operation},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let cached = ListContextDetails::builder()
///     .kind(ContextKindFilter::Cached)
///     .build();
/// let request = cached.to_request(&endpoint)?;
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/jsonldContexts?details=true&kind=Cached"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, bon::Builder)]
pub struct ListContextDetails {
    /// The kind of contexts listed; every kind when `None` (clause 5.13.3.4).
    pub kind: Option<ContextKindFilter>,
}

/// Builds the GET of `jsonldContexts` in the representation `details` names, for contexts of `kind`.
fn request(details: Details, kind: Option<ContextKindFilter>, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
    details
        .get(ResourcePath::contexts(), |query| query.push_some(QueryParameter::Kind, kind))
        .into_request(endpoint, scope)
}

/// Reads one URL of the list.
fn read_url(json: Json) -> Result<Url, DocumentError> {
    url(json, "URL")
}

impl Sealed for ListContexts {}

impl Operation for ListContexts {
    type Outcome = Vec<Url>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Vec<Url>, ResponseError> {
        DocumentReader::document(read_url).decode_list(response)
    }
}

impl BuildsRequest for ListContexts {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(Details::Summary(self.details), self.kind, endpoint, scope)
    }
}

impl Sealed for ListContextDetails {}

impl Operation for ListContextDetails {
    type Outcome = Vec<ContextMetadata>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<Vec<ContextMetadata>, ResponseError> {
        DocumentReader::document(ContextMetadata::from_json).decode_list(response)
    }
}

impl BuildsRequest for ListContextDetails {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        request(Details::Detailed, self.kind, endpoint, scope)
    }
}
