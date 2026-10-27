//! Writing an attribute fragment that sets no payload (NGSI-LD clauses 5.4 and 5.5.8): the members it changes, in the
//! order the other attributes write them.

use crate::model::{
    AttributeMembers,
    Lossless,
    Patch,
    representation::write::{direction::push_member, write_meta_head, write_meta_tail, write_object_type},
};
use jstrict::{Object, Value as Json};

/// Writes the fragment as an object, never as a bare value: its `type` when it names one, then `valueType` and
/// `unitCode` or `objectType`, `datasetId`, `observedAt`, `expiresAt`, `ngsildproof` and the sub-attributes.
pub(crate) fn write_members_object(members: &AttributeMembers, form: Lossless) -> Object {
    let mut object = Object::new();
    if let Some(kind) = members.kind {
        object.push("type".into(), Json::from(kind.as_str()));
    }
    push_member::<Patch, _>(&mut object, "valueType", &members.value_type);
    push_member::<Patch, _>(&mut object, "unitCode", &members.unit_code);
    write_object_type::<Patch>(&mut object, &members.object_type);
    write_meta_head(&mut object, &members.meta);
    write_meta_tail(&mut object, &members.meta, form);
    object
}
