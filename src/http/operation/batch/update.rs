//! Batch Entity Update (NGSI-LD clauses 5.6.9 and 6.16.3.1).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            Overwrite,
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

/// Updates the attributes of every entity of the batch.
///
/// Its request is `POST /entityOperations/update` (NGSI-LD clause 6.16.3.1). Each entity is updated as Update
/// Attributes, or Append Attributes without overwriting, would update it (clause 5.6.9.4). Each fragment names its
/// entity (clause 5.6.9.3).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{BatchUpdate, Operation, Overwrite},
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
/// let request = BatchUpdate::builder()
///     .fragments(&stopped)
///     .overwrite(Overwrite::Replace)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityOperations/update"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct BatchUpdate<'a> {
    /// The changes, one fragment per entity instance, in the order the broker processes them (clause 4.6.6).
    pub fragments: &'a [EntityFragment<Complete>],
    /// Whether existing attributes are overwritten.
    #[builder(default)]
    pub overwrite: Overwrite,
    /// The representation the fragments are written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for BatchUpdate<'_> {}

impl Operation for BatchUpdate<'_> {
    type Outcome = BatchOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for BatchUpdate<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.overwrite.push_query(&mut query);
        documents(BatchResource::Update, query, self.fragments, |fragment| write_fragment(fragment, self.form))?.into_request(endpoint, scope)
    }
}
