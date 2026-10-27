//! Reading the temporal representation of an entity in normalized or concise form (NGSI-LD clauses 4.5.6 to
//! 4.5.8): each evolution an array of instances, or the single instance clauses 4.5.7 and 4.5.8 allow when only
//! one is present, each read as an entity's attribute is.

use crate::model::{
    Completeness,
    DocumentError,
    DocumentErrorKind,
    Lossless,
    entity::member::{EntityMember, SystemMember},
    evolution::{TemporalAttributes, TemporalEntity, scope::read_scope_evolution},
    keyword::CONTEXT,
    reader::{attribute_name, context, date_time, duplicate, empty, entity_types, object, once},
    representation::{
        member::{entity_id, identifying},
        read::{direction::ReadEntityDirection, instance_items},
    },
};
use jstrict::{Value as Json, object::Entry};

/// Reads a temporal entity, keeping its `@context` and the order of its attributes and instances; `C` decides
/// whether `id` and `type` are required.
pub(crate) fn read_temporal_entity<D: ReadEntityDirection, C: Completeness>(json: Json, form: Lossless) -> Result<TemporalEntity<D, C>, DocumentError> {
    let object = object(json)?;
    let mut id = None;
    let mut entity_type = None;
    let mut scope = None;
    let mut expires_at = None;
    let mut entity_context = None;
    let mut output = D::EntityOutput::default();
    let mut attributes = TemporalAttributes::<D>::new();
    for Entry { key, value } in object {
        let key = key.as_str();
        if let Ok(member) = key.parse::<EntityMember>() {
            let name = member.as_str();
            match member {
                EntityMember::Id => once(&mut id, name, entity_id(value, "id")?)?,
                EntityMember::Type => once(&mut entity_type, name, entity_types(value)?)?,
                EntityMember::Scope => once(&mut scope, name, read_scope_evolution::<D>(value, form).map_err(|error| error.within(name))?)?,
                EntityMember::ExpiresAt => once(&mut expires_at, name, date_time(value, "expiresAt")?)?,
                EntityMember::Context => once(&mut entity_context, name, context(value)?)?,
            }
        } else if let Ok(member) = key.parse::<SystemMember>() {
            D::read_entity_output(&mut output, member, value)?;
        } else {
            read_evolution_member(&mut attributes, key, value, form)?;
        }
    }
    Ok(TemporalEntity {
        id: identifying::<C, _>(id, "id")?,
        entity_type: identifying::<C, _>(entity_type, "type")?,
        scope: scope.unwrap_or_default(),
        expires_at,
        attributes,
        output,
        context: entity_context,
    })
}

/// Reads the attributes of a temporal evolution alone, the body of Add Attributes: each member an evolution, in the
/// order written. An `@context` is checked and not kept, since the binding delivers it (clause 6.3.5).
pub(crate) fn read_temporal_attributes<D: ReadEntityDirection>(json: Json, form: Lossless) -> Result<TemporalAttributes<D>, DocumentError> {
    let mut attributes = TemporalAttributes::<D>::new();
    let mut attributes_context = None;
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        if key == CONTEXT {
            once(&mut attributes_context, key, context(value)?)?;
        } else {
            read_evolution_member(&mut attributes, key, value, form)?;
        }
    }
    Ok(attributes)
}

/// Reads the member `member` as the evolution of an attribute and adds it after the ones read before, refusing a
/// name given twice.
fn read_evolution_member<D: ReadEntityDirection>(
    attributes: &mut TemporalAttributes<D>,
    member: &str,
    value: Json,
    form: Lossless,
) -> Result<(), DocumentError> {
    let name = attribute_name(member)?;
    if attributes.get(&name).is_some() {
        return Err(duplicate(member));
    }
    let instances = instance_items(value, form)
        .into_iter()
        .map(|item| D::read_temporal_instance(item, form))
        .collect::<Result<Vec<_>, DocumentError>>()
        .map_err(|error| error.within(member))?;
    if instances.is_empty() {
        return Err(empty().within(member));
    }
    instances.into_iter().try_for_each(|(instance, members)| {
        // The name is kept for the next instance and for the error, so the map gets a copy.
        attributes
            .insert_instance(name.clone(), instance, members)
            .map_err(|source| DocumentError::top(DocumentErrorKind::Attributes { source }).within(member))
    })
}
