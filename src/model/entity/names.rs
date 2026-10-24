//! The names an entity holds: its types and its attributes (NGSI-LD clause 5.5.7). The `id` is a URI, never a
//! term, and is kept.

use crate::model::{Completeness, Entity, EntityDirection, direction::names::MapDirection, entity::completeness::MapCompleteness};

entity_names!(impl<D: EntityDirection + MapDirection, C: MapCompleteness> for Entity<D, C> => C);

carries_context!(impl<D: EntityDirection, C: Completeness> for Entity<D, C> => context);
