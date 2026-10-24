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
    reader::{attribute_name, context, date_time, duplicate, empty, entity_types, object, once},
    representation::{
        member::{entity_id, identifying},
        read::{direction::ReadEntityDirection, instance_items, read_instance},
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
        .map(|mut item| {
            let instance_id = D::take_temporal_instance_id(&mut item)?;
            Ok((read_instance::<D>(item, form)?, instance_id))
        })
        .collect::<Result<Vec<_>, DocumentError>>()
        .map_err(|error| error.within(member))?;
    if instances.is_empty() {
        return Err(empty().within(member));
    }
    instances.into_iter().try_for_each(|(instance, instance_id)| {
        // The name is kept for the next instance and for the error, so the map gets a copy.
        attributes
            .insert_instance(name.clone(), instance, instance_id)
            .map_err(|source| DocumentError::top(DocumentErrorKind::Attributes { source }).within(member))
    })
}
