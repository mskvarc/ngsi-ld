//! Typed, ergonomic NGSI-LD for Rust.
//!
//! The crate models the NGSI-LD API defined by ETSI GS CIM 009 v1.9.1: the documents a client sends and receives
//! ([`model`], the subscriptions and notifications of [`subscription`], the context source registrations of
//! [`registration`], and the snapshots of [`snapshot`]), the query languages it writes ([`query`]), and the operations
//! it calls on a context broker.
//!
//! # Quick start
//!
//! Without features, the crate is the data model and the query languages. An entity is built with its builder,
//! which checks every rule the specification gives it, and rendered in the representation asked for:
//!
//! ```
//! use iri_rs::iri;
//! use ngsi_ld::model::{Entity, Representation};
//!
//! let vehicle = Entity::builder()
//!     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
//!     .entity_type("Vehicle")
//!     .property("speed", 80)
//!     .build()?;
//! assert_eq!(
//!     vehicle.render(Representation::Normalized).to_string(),
//!     r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80}}"#
//! );
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! With the `http` feature, a `Client` sends the operations of the API over any `Transport`; the `reqwest` feature
//! provides one. Here the transport is a stand-in that answers every request as a broker answers a created entity:
//!
//! ```
//! # #[cfg(feature = "http")]
//! # mod offline {
//! #     use bytes::Bytes;
//! #     use http::{HeaderValue, Request, Response, StatusCode, header::LOCATION};
//! #     use std::convert::Infallible;
//! #     pub struct Offline;
//! #     impl ngsi_ld::client::Transport for Offline {
//! #         type Error = Infallible;
//! #         async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
//! #             let mut response = Response::new(Bytes::new());
//! #             *response.status_mut() = StatusCode::CREATED;
//! #             response.headers_mut().insert(LOCATION, HeaderValue::from_static("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"));
//! #             Ok(response)
//! #         }
//! #     }
//! # }
//! # #[cfg(not(feature = "http"))]
//! # fn main() {}
//! # #[cfg(feature = "http")]
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> anyhow::Result<()> {
//! # use offline::Offline;
//! use iri_rs::iri;
//! use ngsi_ld::{client::Client, http::CreateOutcome, model::Entity};
//! use url::Url;
//!
//! let client = Client::builder().base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?).transport(Offline).build()?;
//! let vehicle = Entity::builder().id(iri!("urn:ngsi-ld:Vehicle:A4567")).entity_type("Vehicle").property("speed", 80).build()?;
//! let outcome = client.entities().create(&vehicle).send().await?;
//! assert!(matches!(outcome, CreateOutcome::Created { .. }));
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! The defaults give the client over `reqwest`, with TLS through rustls. With `default-features = false`, the crate is
//! the data model and the query languages, with no HTTP client or async runtime.
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `http` | yes | The HTTP binding and the client, with no transport |
//! | `reqwest` | yes | A transport over `reqwest`, with no TLS |
//! | `reqwest-rustls` | yes | TLS through rustls |
//! | `reqwest-http2` | yes | HTTP/2 |
//! | `reqwest-system-proxy` | yes | The system proxy settings |
//! | `reqwest-native-tls` | no | TLS through the platform's native library |
//! | `linked-data` | no | JSON-LD processing of the `@context` |
//! | `chrono` | no | Conversions to and from `chrono` types |
//! | `geo-types` | no | Conversions to and from `geo-types` geometries |
//! | `geojson` | no | Conversions to and from `geojson` geometries |
//!
//! # Broker compatibility
//!
//! The crate implements NGSI-LD as the specification defines it and expects the broker to do the same. It works
//! around no broker's deviations: an operation that runs into one fails with an error. Only conformant brokers are
//! supported for now; handling the known quirks of a named broker is planned.

#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
mod flag_choice;
#[macro_use]
mod uri_identifier;

#[cfg(feature = "http")]
pub mod client;
pub mod context_file;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "linked-data")]
pub mod linked_data;
#[macro_use]
pub mod model;
pub mod query;
pub mod registration;
pub mod snapshot;
pub mod subscription;
