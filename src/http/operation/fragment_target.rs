//! The entity a fragment sent to one entity names, held to the entity its request path names (NGSI-LD clause 5.4).

use crate::{http::operation::ArgumentError, model::EntityId};

/// Refuses a fragment whose `id` names an entity other than `target`, the entity the request path names: clause 5.4
/// says the `id` of a fragment "shall be equal to the id of the target". A fragment without `id` names no other
/// entity.
pub(super) fn check_fragment_target(target: &EntityId, fragment_id: Option<&EntityId>) -> Result<(), ArgumentError> {
    match fragment_id {
        Some(id) if id != target => Err(ArgumentError::FragmentNamesAnotherEntity),
        Some(_) | None => Ok(()),
    }
}
