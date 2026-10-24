//! The client: an [`Endpoint`](crate::http::Endpoint), which names the default tenant, a [`Transport`], and resource
//! handles whose operation builders finish with `send()`.
//!
//! # Clients and transports
//!
//! [`Client`], [`Transport`], [`ReqwestTransport`], [`ReqwestError`], [`ReqwestBuildError`], [`ClientError`],
//! [`Answering`], [`OutcomeOnly`], [`WithHead`], [`Answered`].
//!
//! # Resource handles
//!
//! [`Entities`], [`Subscriptions`], [`Registrations`], [`Temporal`], [`AvailableEntityTypes`], [`AvailableAttributes`],
//! [`StoredContexts`], [`EntityMaps`], [`Snapshots`], [`SnapshotView`], [`Pages`].
//!
//! # Contexts
//!
//! [`ContextLoader`], [`ContextSource`], [`MediaTypeCheck`], [`ContextLoadError`], [`ContextFileClientError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

#[cfg(feature = "linked-data")]
pub use crate::client::context_loader::{ContextLoadError, ContextLoader, ContextSource, MediaTypeCheck};
#[cfg(feature = "reqwest")]
pub use crate::client::reqwest::{ReqwestBuildError, ReqwestError, ReqwestTransport};
pub use crate::client::{
    answering::{Answered, Answering, OutcomeOnly, WithHead},
    api_client::Client,
    attributes::AvailableAttributes,
    context_file::ContextFileClientError,
    contexts::StoredContexts,
    entities::Entities,
    entity_maps::EntityMaps,
    entity_types::AvailableEntityTypes,
    error::ClientError,
    pages::Pages,
    registrations::Registrations,
    snapshot_view::SnapshotView,
    snapshots::Snapshots,
    subscriptions::Subscriptions,
    temporal::Temporal,
    transport::Transport,
};

mod answering;
mod api_client;
mod arguments;
mod attributes;
pub mod builders;
mod context_file;
#[cfg(feature = "linked-data")]
mod context_loader;
mod contexts;
mod entities;
mod entity_maps;
mod entity_types;
mod error;
mod overrides;
mod pages;
mod registrations;
#[cfg(feature = "reqwest")]
mod reqwest;
pub(crate) mod route;
mod snapshot_view;
mod snapshots;
mod source_identity;
mod subscriptions;
mod temporal;
mod transport;
