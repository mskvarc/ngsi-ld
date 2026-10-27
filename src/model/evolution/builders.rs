//! The builders of the types of [`evolution`](crate::model::evolution).
//!
//! [`AggregatedTemporalEntityBuilder`], [`DeletedInstanceBuilder`], [`ScopeInstanceOutputBuilder`],
//! [`SimplifiedTemporalEntityBuilder`], [`SimplifiedTemporalInputBuilder`], [`TemporalEntityBuilder`],
//! [`TemporalEntityOutputBuilder`].

pub use crate::model::evolution::{
    aggregated::output::AggregatedTemporalEntityBuilder,
    deleted_instance::DeletedInstanceBuilder,
    output::TemporalEntityOutputBuilder,
    scope::ScopeInstanceOutputBuilder,
    simplified::{input::SimplifiedTemporalInputBuilder, output::SimplifiedTemporalEntityBuilder},
    temporal_entity::TemporalEntityBuilder,
};
