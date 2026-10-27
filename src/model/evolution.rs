//! The temporal evolution of an entity.
//!
//! It comes in three representations (NGSI-LD clauses 4.5.6 to 4.5.9, 4.5.19 and 5.2.20): normalized or concise
//! ([`TemporalEntity`]), simplified ([`SimplifiedTemporalEntity`], `format=temporalValues`) and aggregated
//! ([`AggregatedTemporalEntity`], `format=aggregatedValues`).
//!
//! The representation is the caller's choice, made by the request's `format` (clause 6.3.12) and the parse method
//! called; it is never inferred from the shape of a document.
//!
//! # Normalized and concise
//!
//! [`TemporalEntity`], [`TemporalAttributes`], [`Evolution`], [`TemporalAttributeName`],
//! [`TemporalAttributeNameDraft`], [`ScopeInstance`], [`ScopeValue`], [`InstanceSpelling`].
//!
//! # Simplified
//!
//! [`SimplifiedTemporalEntity`], [`SimplifiedTemporalInput`], [`TemporalValues`], [`TemporalValue`].
//!
//! # Aggregated
//!
//! [`AggregatedTemporalEntity`], [`AggregatedAttribute`], [`AggregatedValue`], [`AggregatedKind`],
//! [`AggregationMethod`], [`AggregationMethodIter`], [`NotAggregatable`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::model::evolution::{
    aggregated::{AggregatedAttribute, AggregatedKind, AggregatedTemporalEntity, AggregatedValue, AggregationMethod, AggregationMethodIter, NotAggregatable},
    attribute_name::{TemporalAttributeName, TemporalAttributeNameDraft},
    attributes::{Evolution, TemporalAttributes},
    scope::{ScopeInstance, ScopeValue},
    simplified::{
        SimplifiedTemporalEntity,
        input::SimplifiedTemporalInput,
        values::{TemporalValue, TemporalValues},
    },
    spelling::InstanceSpelling,
    temporal_entity::TemporalEntity,
};

mod aggregated;
mod attribute_name;
mod attributes;
pub mod builders;
mod checked_names;
#[cfg(feature = "linked-data")]
mod names;
mod output;
pub(crate) mod read;
mod scope;
mod simplified;
mod spelling;
mod temporal_entity;
pub(crate) mod write;

#[cfg(test)]
mod tests;
