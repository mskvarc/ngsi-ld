//! Opt-in tests against a running broker: every operation of the client and its error types, asserting what the
//! specification says the broker answers.
//!
//! Every test is `#[ignore]`d, so the default `cargo test` never needs a broker. Each test works on identifiers and
//! entity types of its own, so the tests run in parallel and can be rerun against the same broker. A test whose
//! assertion a broker is known not to meet keeps the assertion and says so in a comment naming the clause and the
//! observed behaviour.
#![cfg(feature = "reqwest")]

/// The client, unique names, and the documents the tests share.
#[cfg(test)]
#[path = "live/broker.rs"]
mod broker;

/// A receiver of notifications.
#[cfg(test)]
#[path = "live/receiver.rs"]
mod receiver;

/// Create, Retrieve, Merge and Replace Entity.
#[cfg(test)]
#[path = "live/entities.rs"]
mod entities;

/// Query Entities through GET and POST, and paging.
#[cfg(test)]
#[path = "live/query.rs"]
mod query;

/// Append and Update Attributes, Partial Attribute Update, Replace and Delete Attribute.
#[cfg(test)]
#[path = "live/attributes.rs"]
mod attributes;

/// Delete Entity and Purge Entities.
#[cfg(test)]
#[path = "live/removal.rs"]
mod removal;

/// Batch Entity Creation, Upsert, Update, Merge and Delete.
#[cfg(test)]
#[path = "live/batch.rs"]
mod batch;

/// Query Entities and Purge Entities with a geoquery and a scope query.
#[cfg(test)]
#[path = "live/geo_scope.rs"]
mod geo_scope;

/// Subscriptions and their notifications.
#[cfg(test)]
#[path = "live/subscriptions.rs"]
mod subscriptions;

/// Context source registrations, registration subscriptions and their notifications.
#[cfg(test)]
#[path = "live/registrations.rs"]
mod registrations;

/// The Temporal API.
#[cfg(test)]
#[path = "live/temporal.rs"]
mod temporal;

/// Entity type and attribute discovery, and the stored `@context`s.
#[cfg(test)]
#[path = "live/discovery.rs"]
mod discovery;

/// Entity maps and the identity of the broker.
#[cfg(test)]
#[path = "live/entity_maps.rs"]
mod entity_maps;

/// The deprecated `attrs`, the `Prefer` header and `local` on Retrieve Temporal Evolution.
#[cfg(test)]
#[path = "live/request_options.rs"]
mod request_options;

/// Snapshots, and operations aimed at one.
#[cfg(test)]
#[path = "live/snapshots.rs"]
mod snapshots;

/// Requests a broker must reject, sent through the unchecked path.
#[cfg(test)]
#[path = "live/rejections.rs"]
mod rejections;

/// Tenants.
#[cfg(test)]
#[path = "live/tenants.rs"]
mod tenants;
