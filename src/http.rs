//! The HTTP binding of the API.
//!
//! It holds typed headers, the operation requests, the one request pipeline that turns them into HTTP requests, and the
//! decoding of HTTP responses into typed outcomes (NGSI-LD clause 6).
//!
//! Everything here is pure and synchronous; sending a request is the job of the `client` module.
//!
//! ```
//! use headers::Header;
//! use iri_rs::iri;
//! use ngsi_ld::{
//!     http::{
//!         Endpoint,
//!         headers::NgsiLdTenant,
//!         operation::{DeleteEntity, Operation},
//!     },
//!     model::{EntityId, Tenant},
//! };
//! use url::Url;
//!
//! let endpoint = Endpoint::builder()
//!     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
//!     .build()?;
//! let madrid: Tenant = "city-of-madrid".parse()?;
//! let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
//! let request = DeleteEntity::builder()
//!     .id(&id)
//!     .build()
//!     .to_request(&endpoint.with_tenant(madrid))?;
//! assert_eq!(
//!     request.uri(),
//!     "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"
//! );
//! assert_eq!(request.headers()[NgsiLdTenant::name()], "city-of-madrid");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Endpoints and requests
//!
//! [`Endpoint`], [`EndpointError`], [`ContextDelivery`], [`RequestError`], [`ContextDeliveryError`],
//! [`QueryParameter`].
//!
//! # Responses
//!
//! [`ResponseContext`], [`ResponseHead`], [`ResponseError`], [`BodyError`], [`ShapeError`], [`Retrieved`], [`Page`],
//! [`Coverage`].
//!
//! # Outcomes
//!
//! [`CreateOutcome`], [`ResourceCreated`], [`UpsertOutcome`], [`UpdateOutcome`], [`TemporalUpsertOutcome`],
//! [`BatchOutcome`], [`BatchCreateOutcome`], [`BatchOperationResult`], [`BatchEntityError`], [`UpdateResult`],
//! [`NotUpdatedDetails`].
//!
//! # Received notifications
//!
//! [`NotificationDelivery`], [`DeliveryError`].
//!
//! # Unchecked documents
//!
//! Deliberately invalid requests, for checking that a broker rejects what the specification says it must reject (clause
//! 5.5.4). Nothing in the strict API accepts these types, and they do not implement
//! [`Operation`](crate::http::operation::Operation): an unchecked request is built from a valid one and then broken on
//! purpose, by name.
//!
//! [`UncheckedDocument`], [`UncheckedRequest`], [`NotAnObject`].
//!
//! # Nested namespaces
//!
//! [`headers`], [`operation`], [`answer`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::http::{
    batch_operation_result::{BatchEntityError, BatchOperationResult},
    context_delivery::ContextDelivery,
    coverage::Coverage,
    delivery::{DeliveryError, NotificationDelivery},
    endpoint::{Endpoint, error::EndpointError},
    outcome::{BatchCreateOutcome, BatchOutcome, CreateOutcome, ResourceCreated, TemporalUpsertOutcome, UpdateOutcome, UpsertOutcome},
    page::Page,
    query_parameter::QueryParameter,
    request::error::{ContextDeliveryError, RequestError},
    response::{BodyError, ResponseError, ShapeError},
    response_context::ResponseContext,
    response_head::ResponseHead,
    retrieved::Retrieved,
    unchecked::{NotAnObject, UncheckedDocument, UncheckedRequest},
    update_result::{NotUpdatedDetails, UpdateResult},
};

pub mod answer;
mod batch_operation_result;
pub mod builders;
mod context_delivery;
mod coverage;
pub(crate) mod delivery;
mod endpoint;
pub mod headers;
pub mod operation;
mod outcome;
mod page;
pub(crate) mod path;
mod query_parameter;
pub(crate) mod query_string;
pub(crate) mod request;
pub(crate) mod response;
mod response_context;
mod response_head;
pub(crate) mod result_headers;
mod retrieved;
mod unchecked;
mod update_result;
