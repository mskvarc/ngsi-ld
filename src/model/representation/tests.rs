//! Attributes, the concise parsing rules, entities and fragments, one module per concern, and the timestamp they
//! share.

use crate::model::DateTime;

/// The `observedAt` the attributes and fragments of these tests carry.
fn observed() -> DateTime {
    "2022-03-14T12:51:02.000Z".parse().unwrap()
}

mod attribute_forms;
mod concise_reading;
mod deletions;
mod entities;
mod fragments_for_entity;
mod instances;
mod output_members;
mod projected;
mod proof;
mod requests;
