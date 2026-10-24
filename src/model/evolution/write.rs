//! Writing the temporal representation of an entity in normalized or concise form (NGSI-LD clauses 4.5.6 to
//! 4.5.8): every evolution an array of instances, each written as an entity's attribute is.

use crate::model::{
    Completeness,
    Lossless,
    evolution::{TemporalAttributes, TemporalEntity, scope::write_scope_instance},
    representation::write::{direction::WriteEntityDirection, push_display, write_identity},
};
use jstrict::{Object, Value as Json};

/// Appends every evolution, in order, each as an array of instance objects: the form of Annex C.5.5, and the one
/// Add Attributes and Modify Attribute Instance require ("contained by an Array", clauses 5.6.12.3 and 5.6.14.3).
fn write_evolutions<D: WriteEntityDirection>(object: &mut Object, attributes: &TemporalAttributes<D>, form: Lossless) {
    for (name, evolution) in attributes {
        let instances = evolution
            .identified()
            .map(|(instance, instance_id)| D::write_temporal_instance(instance, instance_id, form));
        object.push(name.as_ref().into(), Json::Array(instances.collect()));
    }
}

/// Writes the attributes alone, as one object.
pub(crate) fn temporal_attributes_object<D: WriteEntityDirection>(attributes: &TemporalAttributes<D>, form: Lossless) -> Object {
    let mut object = Object::new();
    write_evolutions(&mut object, attributes, form);
    object
}

/// Writes a temporal entity in canonical member order, without `@context`: `id`, `type`, `scope`, `expiresAt`, the
/// attributes, then `createdAt`, `modifiedAt` and `deletedAt`.
pub(crate) fn temporal_entity_object<D: WriteEntityDirection, C: Completeness>(entity: &TemporalEntity<D, C>, form: Lossless) -> Object {
    let mut object = Object::new();
    write_identity::<C>(&mut object, &entity.id, &entity.entity_type);
    if !entity.scope.is_empty() {
        let instances = entity.scope.iter().map(|instance| write_scope_instance(instance, form));
        object.push("scope".into(), Json::Array(instances.collect()));
    }
    push_display(&mut object, "expiresAt", entity.expires_at.as_ref());
    write_evolutions(&mut object, &entity.attributes, form);
    D::write_entity_output(&entity.output, &mut object);
    object
}
