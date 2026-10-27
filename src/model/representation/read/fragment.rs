//! Reading an entity fragment (NGSI-LD clause 5.4) in normalized or concise representation: the members it sets,
//! and the attributes it writes or deletes in document order.

use crate::model::{
    Attributes,
    Completeness,
    DocumentError,
    EntityFragment,
    Lossless,
    Patch,
    ScopeChange,
    entity::member::{EntityMember, SystemMember},
    keyword_alias::unalias_members,
    null::NGSI_LD_NULL,
    reader::{context, date_time, entity_types, object, once, scopes, unexpected},
    representation::{
        member::{entity_id, identifying},
        read::read_attribute_member,
    },
};
use jstrict::{Value as Json, object::Entry};

/// Reads a fragment, keeping its `@context` and the order of its attributes; `C` decides whether `id` and `type` are
/// required. A system member is refused: a fragment is what a client sends (clause 5.2.2).
pub(crate) fn read_fragment<C: Completeness>(json: Json, form: Lossless) -> Result<EntityFragment<C>, DocumentError> {
    let object = unalias_members(object(json)?)?;
    let mut id = None;
    let mut entity_type = None;
    let mut scope = None;
    let mut expires_at = None;
    let mut fragment_context = None;
    let mut attributes = Attributes::<Patch>::new();
    for Entry { key, value } in object {
        let key = key.as_str();
        if let Ok(member) = key.parse::<EntityMember>() {
            let name = member.as_str();
            match member {
                EntityMember::Id => once(&mut id, name, entity_id(value, "id")?)?,
                EntityMember::Type => once(&mut entity_type, name, entity_types(value)?)?,
                EntityMember::Scope => once(&mut scope, name, scope_change(value)?)?,
                EntityMember::ExpiresAt => once(&mut expires_at, name, date_time(value, "expiresAt")?)?,
                EntityMember::Context => once(&mut fragment_context, name, context(value)?)?,
            }
        } else if key.parse::<SystemMember>().is_ok() {
            return Err(unexpected(key));
        } else {
            read_attribute_member(&mut attributes, key, value, form)?;
        }
    }
    Ok(EntityFragment {
        id: identifying::<C, _>(id, "id")?,
        entity_type: identifying::<C, _>(entity_type, "type")?,
        scope,
        expires_at,
        attributes,
        context: fragment_context,
    })
}

/// Reads what a fragment does to `scope`: the NGSI-LD Null deletes every scope (clause 4.18), anything else sets them.
fn scope_change(json: Json) -> Result<ScopeChange, DocumentError> {
    match json {
        Json::String(text) if text.as_str() == NGSI_LD_NULL.as_str() => Ok(ScopeChange::Delete),
        other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) | Json::Object(_)) => scopes(other).map(ScopeChange::Set),
    }
}
