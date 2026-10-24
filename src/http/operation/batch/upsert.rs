//! Batch Entity Creation or Update, "upsert" (NGSI-LD clauses 5.6.8 and 6.15.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        UpsertOutcome,
        operation::{
            Operation,
            UpsertMode,
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

/// `POST /entityOperations/upsert`: creates every entity of the batch that does not exist and replaces or updates
/// every one that does (NGSI-LD clause 6.15.3.1). Later instances of one entity replace or update it in array
/// order (clause 5.5.11.2).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{BatchUpsert, Operation, UpsertMode},
///     },
///     model::Entity,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles = [Entity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///     .entity_type("Vehicle")
///     .property("speed", 80)
///     .build()?];
/// let request = BatchUpsert::builder()
///     .entities(&vehicles)
///     .mode(UpsertMode::Update)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityOperations/upsert?options=update"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct BatchUpsert<'a> {
    /// The entities, in the order the broker processes them (clause 4.6.6).
    pub entities: &'a [Entity<Input>],
    /// What happens to an entity that exists.
    #[builder(default)]
    pub mode: UpsertMode,
    /// The representation the entities are written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for BatchUpsert<'_> {}

impl Operation for BatchUpsert<'_> {
    type Outcome = UpsertOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<UpsertOutcome, ResponseError> {
        UpsertOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for BatchUpsert<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.mode.push_query(&mut query);
        documents(BatchResource::Upsert, query, self.entities, |entity| write_entity(entity, self.form))?
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
