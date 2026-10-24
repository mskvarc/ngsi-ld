//! Batch Entity Creation (NGSI-LD clauses 5.6.7 and 6.14.3.1).

use crate::{
    http::{
        BatchCreateOutcome,
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
    model::{Entity, Input, Lossless, NgsiLdMediaType, representation::write::write_entity},
};
use bytes::Bytes;
use http::{Request, Response};

/// `POST /entityOperations/create`: creates every entity of the batch (NGSI-LD clause 6.14.3.1). Of several
/// instances of one entity, the first creates it and every later one is reported as `AlreadyExists` (clause
/// 5.5.11.1).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{BatchCreate, Operation},
///     },
///     model::Entity,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles = [
///     Entity::builder()
///         .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///         .entity_type("Vehicle")
///         .property("speed", 80)
///         .build()?,
///     Entity::builder()
///         .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///         .entity_type("Vehicle")
///         .property("speed", 55)
///         .build()?,
/// ];
/// let request = BatchCreate::builder()
///     .entities(&vehicles)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityOperations/create"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct BatchCreate<'a> {
    /// The entities, in the order the broker processes them (clause 4.6.6).
    pub entities: &'a [Entity<Input>],
    /// The representation the entities are written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for BatchCreate<'_> {}

impl Operation for BatchCreate<'_> {
    type Outcome = BatchCreateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchCreateOutcome, ResponseError> {
        BatchCreateOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for BatchCreate<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        documents(BatchResource::Create, QueryString::default(), self.entities, |entity| {
            write_entity(entity, self.form)
        })?
        .accepting(NgsiLdMediaType::Json)
        .into_request(endpoint, scope)
    }
}
