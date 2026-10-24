//! Reading the members of entities that every representation shares: entity ids, the values of `GeoProperty` attributes
//! and the identifying members a projection may leave out. The normalized and concise readers and the simplified reader
//! all go through these, and through the readers of [`model::reader`](crate::model::reader).

use crate::{
    model::{
        Completeness,
        DocumentError,
        EntityId,
        GeoValue,
        MemberError,
        reader::{identifier, required},
    },
    seal::Seal,
};
use jstrict::Value as Json;
use std::fmt::Debug;

/// Reads an entity id.
pub(crate) fn entity_id(json: Json, member: &'static str) -> Result<EntityId, DocumentError> {
    identifier(json, member)
}

/// Reads the value of a `GeoProperty`.
pub(crate) fn geo_value(json: &Json) -> Result<GeoValue, DocumentError> {
    GeoValue::from_json(json).map_err(|source| DocumentError::top(MemberError::Geometry { source }.into()))
}

/// Takes an identifying member of an entity (`id` or `type`), which a complete entity requires and a projected
/// one may lack (clause 4.21).
pub(crate) fn identifying<C: Completeness, T: Debug + Clone + PartialEq + Eq + Send + Sync>(
    slot: Option<T>,
    member: &'static str,
) -> Result<C::Member<T>, DocumentError> {
    required(C::from_read(Seal::new(), slot), member)
}
