//! The names an entity fragment holds: the types it adds and the attributes it changes (NGSI-LD clause 5.5.7).

use crate::model::{Completeness, EntityFragment, entity::completeness::MapCompleteness};

entity_names!(impl<C: MapCompleteness> for EntityFragment<C> => C);

carries_context!(
    impl<C: Completeness> for EntityFragment<C> => context
);
