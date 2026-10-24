//! What each direction writes that only it has: its attribute slot, and its output-only members.

use crate::model::{
    Attribute,
    Change,
    Direction,
    EntityDirection,
    EntityOutputMembers,
    Input,
    InstanceId,
    Lossless,
    MemberChange,
    Output,
    Patch,
    representation::{
        kind_output::KindOutputRepresentation,
        write::{Placement, deletion::write_deletion, push_display, write_attribute, write_attribute_object},
    },
};
use jstrict::{Object, Value as Json, object::Entry};
use std::fmt::{Debug, Display};

/// Appends an optional member of an attribute as a string, or as the NGSI-LD Null a fragment deletes it with.
pub(crate) fn push_member<D: WriteDirection, T: Debug + Clone + PartialEq + Eq + Display>(object: &mut Object, key: &str, member: &D::Member<T>) {
    if let Some(json) = D::write_member(member, |value| Json::from(value.to_string())) {
        object.push(key.into(), json);
    }
}

/// How each direction writes what only it has: its attribute slot, its output-only members, and the changes a
/// fragment makes to the optional members of an attribute.
pub(crate) trait WriteDirection: Direction {
    /// Writes an optional member of an attribute with `write`, or, in a fragment that deletes it, as the NGSI-LD Null
    /// (clause 5.5.8); `None` when the member is absent.
    fn write_member<T: Debug + Clone + PartialEq + Eq>(member: &Self::Member<T>, write: impl FnOnce(&T) -> Json) -> Option<Json>;

    /// Writes one slot: an attribute, or in a fragment a change to one.
    fn write_slot(slot: &Self::Slot, form: Lossless, placement: Placement) -> Json;
    /// Appends the output-only members every attribute kind has.
    fn write_attribute_output(output: &Self::AttributeOutput, object: &mut Object);
    /// Appends the output-only members of one attribute kind, `K`.
    fn write_kind_output<K: KindOutputRepresentation>(output: &Self::KindOutput<K>, object: &mut Object, form: Lossless);
}

impl WriteDirection for Input {
    fn write_member<T: Debug + Clone + PartialEq + Eq>(member: &Option<T>, write: impl FnOnce(&T) -> Json) -> Option<Json> {
        member.as_ref().map(write)
    }

    fn write_slot(slot: &Self::Slot, form: Lossless, placement: Placement) -> Json {
        write_attribute(slot, form, placement)
    }

    fn write_attribute_output((): &Self::AttributeOutput, _: &mut Object) {}

    fn write_kind_output<K: KindOutputRepresentation>((): &(), _: &mut Object, _: Lossless) {}
}

impl WriteDirection for Output {
    fn write_member<T: Debug + Clone + PartialEq + Eq>(member: &Option<T>, write: impl FnOnce(&T) -> Json) -> Option<Json> {
        member.as_ref().map(write)
    }

    fn write_slot(slot: &Self::Slot, form: Lossless, placement: Placement) -> Json {
        write_attribute(slot, form, placement)
    }

    fn write_attribute_output(output: &Self::AttributeOutput, object: &mut Object) {
        push_display(object, "createdAt", output.created_at.as_ref());
        push_display(object, "modifiedAt", output.modified_at.as_ref());
        push_display(object, "deletedAt", output.deleted_at.as_ref());
        push_display(object, "instanceId", output.instance_id.as_ref());
    }

    fn write_kind_output<K: KindOutputRepresentation>(output: &K, object: &mut Object, form: Lossless) {
        output.write(object, form);
    }
}

impl WriteDirection for Patch {
    fn write_member<T: Debug + Clone + PartialEq + Eq>(member: &Option<MemberChange<T>>, write: impl FnOnce(&T) -> Json) -> Option<Json> {
        member.as_ref().and_then(|change| change.write(|value| Some(write(value))))
    }

    fn write_slot(slot: &Self::Slot, form: Lossless, placement: Placement) -> Json {
        match slot {
            Change::Set(attribute) => write_attribute(attribute, form, placement),
            Change::Delete(deletion) => write_deletion(deletion, form),
        }
    }

    fn write_attribute_output((): &Self::AttributeOutput, _: &mut Object) {}

    fn write_kind_output<K: KindOutputRepresentation>((): &(), _: &mut Object, _: Lossless) {}
}

/// How an entity direction writes its output-only members.
pub(crate) trait WriteEntityDirection: EntityDirection + WriteDirection {
    /// Appends the system members.
    fn write_entity_output(output: &Self::EntityOutput, object: &mut Object);

    /// Writes one instance of a temporal evolution, always as an object (clauses 4.5.7 and 4.5.8), with the
    /// `instanceId` a request names for it.
    fn write_temporal_instance(attribute: &Attribute<Self>, instance_id: &Self::TemporalInstanceId, form: Lossless) -> Json;
}

impl WriteEntityDirection for Input {
    fn write_entity_output((): &Self::EntityOutput, _: &mut Object) {}

    /// `instanceId` takes the place a response gives it: after the members of the instance itself, before its
    /// `ngsildproof` and its sub-attributes, which close the object (see the canonical order of [`write_attribute`]).
    fn write_temporal_instance(attribute: &Attribute<Self>, instance_id: &Option<InstanceId>, form: Lossless) -> Json {
        let object = write_attribute_object(attribute, form);
        let Some(instance_id) = instance_id else {
            return Json::Object(object);
        };
        let meta = attribute.meta();
        let mut head: Vec<Entry> = object.into_iter().collect();
        let tail = head.split_off(head.len().saturating_sub(usize::from(meta.proof.is_some()) + meta.attributes.len()));
        let mut object = Object::new();
        for entry in head {
            object.push_entry(entry);
        }
        object.push("instanceId".into(), Json::from(instance_id.to_string()));
        for entry in tail {
            object.push_entry(entry);
        }
        Json::Object(object)
    }
}

impl WriteEntityDirection for Output {
    fn write_entity_output(output: &EntityOutputMembers, object: &mut Object) {
        push_display(object, "createdAt", output.created_at.as_ref());
        push_display(object, "modifiedAt", output.modified_at.as_ref());
        push_display(object, "deletedAt", output.deleted_at.as_ref());
    }

    fn write_temporal_instance(attribute: &Attribute<Self>, (): &(), form: Lossless) -> Json {
        write_attribute(attribute, form, Placement::AmongInstances)
    }
}
