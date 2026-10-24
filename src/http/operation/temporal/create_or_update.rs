//! Create or Update Temporal Evolution of an Entity (NGSI-LD clauses 5.6.11 and 6.18.3.1), with the temporal
//! evolution in normalized or concise representation, or in simplified temporal representation (clause 5.2.20).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        TemporalUpsertOutcome,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{
        Input,
        Lossless,
        evolution::{SimplifiedTemporalInput, TemporalEntity, write::temporal_entity_object},
    },
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `POST /temporal/entities`: creates the temporal evolution of an entity, or adds the instances sent to the one
/// that exists (NGSI-LD clause 6.18.3.1).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateOrUpdateTemporalEvolution, Operation},
///     },
///     model::{Property, evolution::TemporalEntity},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicle = TemporalEntity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///     .entity_type("Vehicle")
///     .property(
///         "speed",
///         Property::builder(120).observed_at("2018-08-01T12:03:00Z".parse()?),
///     )
///     .build()?;
/// let request = CreateOrUpdateTemporalEvolution::builder()
///     .entity(&vehicle)
///     .build()
///     .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/temporal/entities"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct CreateOrUpdateTemporalEvolution<'a> {
    /// The temporal evolution.
    pub entity: &'a TemporalEntity<Input>,
    /// The representation the body is written in.
    #[builder(default)]
    pub form: Lossless,
}

impl Sealed for CreateOrUpdateTemporalEvolution<'_> {}

impl Operation for CreateOrUpdateTemporalEvolution<'_> {
    type Outcome = TemporalUpsertOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<TemporalUpsertOutcome, ResponseError> {
        TemporalUpsertOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for CreateOrUpdateTemporalEvolution<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::temporal_entities(), QueryString::default())
            .with_body(temporal_entity_object(self.entity, self.form))
            .into_request(endpoint, scope)
    }
}

/// `POST /temporal/entities` with the temporal evolution in simplified temporal representation (NGSI-LD clauses
/// 5.2.20 and 6.18.3.1): the same operation as [`CreateOrUpdateTemporalEvolution`], whose body clause 5.2.20 allows in
/// either representation. Table 6.18.3.1-1 has no parameter naming the representation; each attribute's member
/// (`values`, `languageMaps`, ..., clause 4.5.9) tells it.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{CreateOrUpdateSimplifiedTemporalEvolution, Operation},
///     },
///     model::{
///         Value,
///         evolution::{SimplifiedTemporalInput, TemporalValue, TemporalValues},
///     },
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let speed = TemporalValues::Property(vec![TemporalValue {
///     value: Value::from(120),
///     time: "2018-08-01T12:03:00Z".parse()?,
/// }]);
/// let vehicle = SimplifiedTemporalInput::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///     .entity_type("Vehicle")
///     .values("speed", speed)
///     .build()?;
/// let request =
///     CreateOrUpdateSimplifiedTemporalEvolution { entity: &vehicle }.to_request(&endpoint)?;
/// assert_eq!(
///     request.body().as_ref(),
///     concat!(
///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
///         r#""speed":{"type":"Property","values":[[120,"#,
///         r#""2018-08-01T12:03:00Z"]]}}"#
///     )
///     .as_bytes()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreateOrUpdateSimplifiedTemporalEvolution<'a> {
    /// The temporal evolution.
    pub entity: &'a SimplifiedTemporalInput,
}

impl Sealed for CreateOrUpdateSimplifiedTemporalEvolution<'_> {}

impl Operation for CreateOrUpdateSimplifiedTemporalEvolution<'_> {
    type Outcome = TemporalUpsertOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<TemporalUpsertOutcome, ResponseError> {
        TemporalUpsertOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for CreateOrUpdateSimplifiedTemporalEvolution<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::temporal_entities(), QueryString::default())
            .with_body(self.entity.to_object())
            .into_request(endpoint, scope)
    }
}
