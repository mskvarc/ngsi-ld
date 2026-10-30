//! Reading entities and attributes in normalized and concise representation.
//!
//! The concise inference rules are applied here, in one place, in the order the `Lossless` documentation lists.

use crate::model::{
    Attribute,
    AttributeFragment,
    Attributes,
    Change,
    Completeness,
    Deletion,
    DocumentError,
    DocumentErrorKind,
    Entity,
    InstanceDeletion,
    Instances,
    Lossless,
    MemberError,
    NullMember,
    Patch,
    entity::member::{EntityMember, SystemMember},
    keyword_alias::{unalias_attribute, unalias_members},
    nesting::grow,
    reader::{attribute_name, context, date_time, duplicate, entity_types, object, once, scopes},
    representation::{
        member::{entity_id, identifying},
        read::{
            concise::read_concise,
            direction::{ReadDirection, ReadEntityDirection},
            instance::read_normalized,
            members::{FragmentShape, read_members},
        },
        shape::shapes_attribute,
    },
};
use jstrict::{Value as Json, object::Entry};

mod concise;
pub(crate) mod direction;
pub(crate) mod fragment;
pub(crate) mod instance;
pub(crate) mod members;
mod patch;

/// Reads an entity, keeping its `@context` and the order of its attributes; `C` decides whether `id` and `type` are
/// required.
pub(crate) fn read_entity<D: ReadEntityDirection, C: Completeness>(json: Json, form: Lossless) -> Result<Entity<D, C>, DocumentError> {
    let object = unalias_members(object(json)?)?;
    let mut id = None;
    let mut entity_type = None;
    let mut scope = None;
    let mut expires_at = None;
    let mut entity_context = None;
    let mut output = D::EntityOutput::default();
    let mut attributes = Attributes::<D>::new();
    for Entry { key, value } in object {
        let key = key.as_str();
        if let Ok(member) = key.parse::<EntityMember>() {
            let name = member.as_str();
            match member {
                EntityMember::Id => once(&mut id, name, entity_id(value, "id")?)?,
                EntityMember::Type => once(&mut entity_type, name, entity_types(value)?)?,
                EntityMember::Scope => once(&mut scope, name, scopes(value)?)?,
                EntityMember::ExpiresAt => once(&mut expires_at, name, date_time(value, "expiresAt")?)?,
                EntityMember::Context => once(&mut entity_context, name, context(value)?)?,
            }
        } else if let Ok(member) = key.parse::<SystemMember>() {
            D::read_entity_output(&mut output, member, value)?;
        } else {
            read_attribute_member(&mut attributes, key, value, form)?;
        }
    }
    Ok(Entity {
        id: identifying::<C, _>(id, "id")?,
        entity_type: identifying::<C, _>(entity_type, "type")?,
        scope,
        expires_at,
        attributes,
        output,
        context: entity_context,
    })
}

/// Reads the member `member` as an attribute and adds it after the ones read before, refusing a name given twice.
/// Every level of nesting passes through here, so the attribute is read on a stack that grows with the depth (see
/// [`grow`]).
pub(crate) fn read_attribute_member<D: ReadDirection>(attributes: &mut Attributes<D>, member: &str, value: Json, form: Lossless) -> Result<(), DocumentError> {
    let name = attribute_name(member)?;
    if attributes.get(&name).is_some() {
        return Err(duplicate(member));
    }
    let instances = grow(|| read_instances::<D>(value, form)).map_err(|error| error.within(member))?;
    attributes
        .set(name, instances)
        .map_err(|source| DocumentError::top(DocumentErrorKind::Attributes { source }).within(member))
}

/// Reads the value of an attribute member: one instance, or an array of them.
pub(crate) fn read_instances<D: ReadDirection>(json: Json, form: Lossless) -> Result<Instances<D::Slot>, DocumentError> {
    Instances::try_from_vec(read_instance_list::<D>(json, form)?).map_err(|source| DocumentError::top(DocumentErrorKind::Instances { source }))
}

/// Reads one instance, or an array of them, in document order, before any rule ties them together: an entity's
/// instances are then checked by dataset id (clause 4.5.5), a temporal evolution's by kind (clause 4.5.7). Each
/// instance is the direction's slot: an attribute, or in a fragment a change to one.
pub(crate) fn read_instance_list<D: ReadDirection>(json: Json, form: Lossless) -> Result<Vec<D::Slot>, DocumentError> {
    instance_items(json, form).into_iter().map(|item| D::read_slot(item, form)).collect()
}

/// Splits the value of an attribute member into its instances: the elements of an array of them, or the value
/// itself. In concise form an array is a list of instances only when every element is shaped like an attribute;
/// otherwise it is the bare array value of a Property (clause 4.5.2.3).
pub(crate) fn instance_items(json: Json, form: Lossless) -> Vec<Json> {
    match (form, json) {
        (Lossless::Concise, Json::Array(items)) if !items.is_empty() && items.iter().all(shapes_attribute) => items.into(),
        (Lossless::Normalized, Json::Array(items)) => items.into(),
        (Lossless::Concise | Lossless::Normalized, json) => vec![json],
    }
}

/// Reads a document of one attribute instance, the body of an operation on one attribute.
pub(crate) fn read_lone_instance<D: ReadDirection>(json: Json, form: Lossless) -> Result<D::Slot, DocumentError> {
    single(read_instances::<D>(json, form)?.into_vec())
}

/// Reads the body of Partial Attribute Update (clauses 5.6.4 and 6.7.3.1): an attribute fragment, or the deletion of
/// the instance.
pub(crate) fn read_attribute_body(json: Json, form: Lossless) -> Result<Change<AttributeFragment, InstanceDeletion>, DocumentError> {
    let json = match json {
        Json::Object(object) => {
            let object = unalias_attribute(object)?;
            match FragmentShape::of(&object) {
                FragmentShape::Members(kind) => return read_members(kind, object, form).map(|members| Change::Set(members.into())),
                FragmentShape::Payload => Json::Object(object),
            }
        }
        other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => other,
    };
    match single(read_instance_list::<Patch>(json, form)?)? {
        Change::Set(attribute) => Ok(Change::Set(attribute.into())),
        Change::Delete(Deletion::Instance(deletion)) => Ok(Change::Delete(deletion)),
        Change::Delete(Deletion::Member) => Err(DocumentError::top(DocumentErrorKind::NgsiLdNull { member: NullMember::Value })),
    }
}

/// The only instance of a one-instance document.
fn single<T>(instances: Vec<T>) -> Result<T, DocumentError> {
    <[T; 1]>::try_from(instances)
        .map(|[one]| one)
        .map_err(|_| DocumentError::top(MemberError::NotAnObject.into()))
}

/// Reads one instance in `form`, and lets the direction admit it.
pub(crate) fn read_instance<D: ReadDirection>(json: Json, form: Lossless) -> Result<Attribute<D>, DocumentError> {
    let attribute = read_unadmitted::<D>(json, form)?;
    D::admit(&attribute)?;
    Ok(attribute)
}

/// Reads one instance in `form`, before the direction admits it.
pub(crate) fn read_unadmitted<D: ReadDirection>(json: Json, form: Lossless) -> Result<Attribute<D>, DocumentError> {
    match form {
        Lossless::Concise => read_concise::<D>(json),
        Lossless::Normalized => read_normalized::<D>(json),
    }
}
