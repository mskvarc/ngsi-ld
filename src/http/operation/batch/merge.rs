//! Batch Entity Merge (NGSI-LD clauses 5.6.20 and 6.31.3.1).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            batch::documents,
            sealed::{BuildsRequest, Sealed},
        },
        path::BatchResource,
        query_string::QueryString,
        request::scope::RequestScope,
        response::classify,
    },
    model::{Complete, EntityFragment, Lossless, representation::write::write_fragment},
};
use bytes::Bytes;
use http::{Request, Response};

/// `POST /entityOperations/merge`: merges every fragment of the batch into the entity it names (NGSI-LD clause
/// 6.31.3.1), as Merge Entity would for each (clause 5.6.20.4). Later fragments of one entity are merged after
/// earlier ones (clause 5.5.11.5).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{BatchMerge, Operation},
///     },
///     model::EntityFragment,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let stopped = [
///     EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
///         .property("speed", 0)
///         .build()?,
/// ];
/// let request = BatchMerge::builder()
///     .fragments(&stopped)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityOperations/merge"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct BatchMerge<'a> {
    /// The changes, one fragment per entity instance, in the order the broker processes them (clause 4.6.6).
    pub fragments: &'a [EntityFragment<Complete>],
    /// The representation the fragments are written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for BatchMerge<'_> {}

impl Operation for BatchMerge<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for BatchMerge<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        documents(BatchResource::Merge, QueryString::default(), self.fragments, |fragment| {
            write_fragment(fragment, self.form)
        })?
        .into_request(endpoint, scope)
    }
}
