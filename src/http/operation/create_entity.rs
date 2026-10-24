//! Create Entity (NGSI-LD clauses 5.6.1 and 6.4.3.1).

use crate::{
    http::{
        CreateOutcome,
        Endpoint,
        RequestError,
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
    model::{Entity, Input, Lossless, representation::write::write_entity},
};
use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};

/// Creates an entity.
///
/// Its request is `POST /entities` (NGSI-LD clause 6.4.3.1).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateEntity, Operation},
///     },
///     model::Entity,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicle = Entity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///     .entity_type("Vehicle")
///     .property("speed", 80)
///     .build()?;
/// let request = CreateEntity::builder()
///     .entity(&vehicle)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/entities");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct CreateEntity<'a> {
    /// The entity.
    pub entity: &'a Entity<Input>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for CreateEntity<'_> {}

impl Operation for CreateEntity<'_> {
    type Outcome = CreateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<CreateOutcome, ResponseError> {
        let response = classify(response)?;
        let location = response.location();
        match response.status() {
            StatusCode::CREATED => Ok(CreateOutcome::Created { location }),
            StatusCode::MULTI_STATUS => Ok(CreateOutcome::PartiallyCreated {
                location,
                result: response.deserialize()?,
            }),
            _ => Err(response.unexpected()),
        }
    }
}

impl BuildsRequest for CreateEntity<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::entities(), QueryString::default())
            .with_body(write_entity(self.entity, self.form))
            .into_request(endpoint, scope)
    }
}
