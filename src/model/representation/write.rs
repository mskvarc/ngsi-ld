//! Writing entities, attributes and fragments in normalized and concise representation (NGSI-LD clauses
//! 4.5.2.2, 4.5.2.3, 4.5.3.2, 4.5.3.3, 4.7.3 and their counterparts for the other kinds).
//!
//! The member order is the one the `Lossless` documentation lists; `@context` is written only by
//! [`write_context`].
//!
//! Writing builds an owned JSON tree from borrowed data, so names and strings are copied into it.

use crate::{
    model::{
        Attribute,
        AttributeFragment,
        AttributeKind,
        AttributeMeta,
        Attributes,
        Change,
        Completeness,
        Context,
        Direction,
        Entity,
        EntityFragment,
        EntityId,
        EntityTypeName,
        InstanceDeletion,
        Instances,
        Lossless,
        OneOrMany,
        Property,
        ScopeChange,
        Value,
        geometry::Geometry,
        keyword::CONTEXT,
        nesting::grow,
        null::NGSI_LD_NULL,
        representation::{
            proof::write_proof,
            shape::shapes_attribute,
            write::{
                deletion::{bare_deletion, write_instance_deletion},
                direction::{WriteDirection, WriteEntityDirection, push_member},
                members::write_members_object,
            },
        },
    },
    seal::Seal,
};
use jstrict::{Object, Value as Json};

pub(crate) mod deletion;
pub(crate) mod direction;
mod members;

/// Where an attribute instance is written: alone as its member's value, or as one element of an instance array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// The only instance, the whole member value: the concise form may write it bare.
    Alone,
    /// One of several instances in an array, which a reader recognises only when every element is an attribute
    /// object (clauses 4.5.2.3 and 4.5.5), so the instance is always an object.
    AmongInstances,
}

/// Appends `value` as a string member when present.
pub(crate) fn push_display(object: &mut Object, key: &str, value: Option<&impl ToString>) {
    if let Some(value) = value {
        object.push(key.into(), Json::from(value.to_string()));
    }
}

/// Writes one or more instances: one as the member value, several as an array of attribute objects (clauses
/// 4.5.2.2, 4.5.3.2 and 4.5.5).
pub(crate) fn write_instances<D: WriteDirection>(instances: &Instances<D::Slot>, form: Lossless) -> Json {
    match instances.iter().as_slice() {
        [only] => D::write_slot(only, form, Placement::Alone),
        several => Json::Array(
            several
                .iter()
                .map(|instance| D::write_slot(instance, form, Placement::AmongInstances))
                .collect(),
        ),
    }
}

/// Appends every attribute, in order. Every level of nesting passes through here, so the attributes are written on
/// a stack that grows with the depth (see [`grow`]).
pub(crate) fn write_attributes<D: WriteDirection>(object: &mut Object, attributes: &Attributes<D>, form: Lossless) {
    for (name, instances) in attributes {
        let instances = grow(|| write_instances::<D>(instances, form));
        object.push(name.as_ref().into(), instances);
    }
}

/// Writes an attribute of any kind. An attribute that stands alone is bare when it can be: a deleted attribute with
/// nothing to report is the bare NGSI-LD Null in either form (clause 5.8.6), and in concise form an attribute may be
/// its bare value.
pub(crate) fn write_attribute<D: WriteDirection>(attribute: &Attribute<D>, form: Lossless, placement: Placement) -> Json {
    let bare = match placement {
        Placement::Alone => bare_deletion(attribute).or_else(|| match form {
            Lossless::Concise => bare_concise(attribute),
            Lossless::Normalized => None,
        }),
        Placement::AmongInstances => None,
    };
    bare.unwrap_or_else(|| Json::Object(write_attribute_object(attribute, form)))
}

/// Writes an attribute as an object, never as a bare concise value: the form a request body takes, which may
/// have to carry an `@context` member (clause 6.3.5).
pub(crate) fn write_attribute_object<D: WriteDirection>(attribute: &Attribute<D>, form: Lossless) -> Object {
    let mut object = Object::new();
    if form == Lossless::Normalized || needs_type_in_concise(attribute) {
        object.push("type".into(), Json::from(attribute.kind().as_str()));
    }
    write_payload(&mut object, attribute, form);
    write_meta_head(&mut object, attribute.meta());
    write_kind_output(&mut object, attribute, form);
    write_meta_tail(&mut object, attribute.meta(), form);
    object
}

/// Writes the body of Partial Attribute Update (clause 6.7.3.1): the attribute fragment as an object, or the
/// deletion of the instance as an object whose payload is the NGSI-LD Null.
pub(crate) fn write_attribute_body(body: &Change<AttributeFragment, InstanceDeletion>, form: Lossless) -> Object {
    match body {
        Change::Set(fragment) => write_fragment_object(fragment, form),
        Change::Delete(deletion) => write_instance_deletion(deletion, form),
    }
}

/// Writes an attribute fragment as an object, never as a bare concise value.
pub(crate) fn write_fragment_object(fragment: &AttributeFragment, form: Lossless) -> Object {
    match fragment {
        AttributeFragment::WithPayload(attribute) => write_attribute_object(attribute, form),
        AttributeFragment::WithoutPayload(members) => write_members_object(members, form),
    }
}

/// Writes a Property as an object, never as a bare concise value: the form an `ngsildproof` takes, which carries
/// members besides the Property's own.
pub(crate) fn write_property_object<D: WriteDirection>(property: &Property<D>, form: Lossless) -> Object {
    let mut object = Object::new();
    if form == Lossless::Normalized || property_needs_type_in_concise(property) {
        object.push("type".into(), Json::from(AttributeKind::Property.as_str()));
    }
    write_property_payload(&mut object, property);
    write_meta_head(&mut object, &property.meta);
    D::write_kind_output(&property.output, &mut object, form);
    write_meta_tail(&mut object, &property.meta, form);
    object
}

/// Whether the members besides the payload are all absent, the output-only members of the kind included.
pub(crate) fn is_plain<D: Direction, K: PartialEq + Default>(meta: &AttributeMeta<D>, kind_output: &K) -> bool {
    meta.dataset_id.is_none()
        && D::lacks(Seal::new(), &meta.observed_at)
        && D::lacks(Seal::new(), &meta.expires_at)
        && meta.proof.is_none()
        && meta.attributes.is_empty()
        && meta.output == D::AttributeOutput::default()
        && *kind_output == K::default()
}

/// The bare concise form, when the attribute has one: a Property with nothing but a value that cannot be
/// misread (clause 4.5.2.3), or a `GeoProperty` with nothing but its geometry (clause 4.7.3).
fn bare_concise<D: Direction>(attribute: &Attribute<D>) -> Option<Json> {
    match attribute {
        Attribute::Property(property)
            if is_plain(&property.meta, &property.output) && D::lacks(Seal::new(), &property.unit_code) && D::lacks(Seal::new(), &property.value_type) =>
        {
            let value = property.value.to_json();
            is_unambiguous_bare_value(&value).then_some(value)
        }
        Attribute::GeoProperty(property)
            if property.value.is_geometry()
                && is_plain(&property.meta, &property.output)
                && D::lacks(Seal::new(), &property.unit_code)
                && D::lacks(Seal::new(), &property.value_type) =>
        {
            Some(property.value.to_json())
        }
        Attribute::Property(_)
        | Attribute::GeoProperty(_)
        | Attribute::LanguageProperty(_)
        | Attribute::VocabProperty(_)
        | Attribute::ListProperty(_)
        | Attribute::JsonProperty(_)
        | Attribute::Relationship(_)
        | Attribute::ListRelationship(_) => None,
    }
}

/// Whether a reader of the concise form reads `value`, written bare, back as the value of a Property: neither an
/// object a reader takes for an attribute, nor an array of such objects, which it takes for instances.
fn is_unambiguous_bare_value(value: &Json) -> bool {
    match value {
        Json::Object(_) => !shapes_attribute(value),
        Json::Array(items) => items.is_empty() || !items.iter().all(shapes_attribute),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) => true,
    }
}

/// Whether a concise attribute keeps its `type`: a Property whose value is shaped like a geometry would
/// otherwise be read as a `GeoProperty` (clause 5.2.5), and a `GeoProperty` holding the NGSI-LD Null as a Property
/// (clauses 4.5.2.3 and 4.7.3).
fn needs_type_in_concise<D: Direction>(attribute: &Attribute<D>) -> bool {
    match attribute {
        Attribute::Property(property) => property_needs_type_in_concise(property),
        Attribute::GeoProperty(property) => property.value.is_null(),
        Attribute::LanguageProperty(_)
        | Attribute::VocabProperty(_)
        | Attribute::ListProperty(_)
        | Attribute::JsonProperty(_)
        | Attribute::Relationship(_)
        | Attribute::ListRelationship(_) => false,
    }
}

/// Whether a concise Property keeps its `type`: one whose value is shaped like a geometry would otherwise be read
/// as a `GeoProperty` (clause 5.2.5).
fn property_needs_type_in_concise<D: Direction>(property: &Property<D>) -> bool {
    Geometry::from_json(&property.value.to_json()).is_ok()
}

/// Writes the `value`, `valueType` and `unitCode` of a Property.
fn write_property_payload<D: WriteDirection>(object: &mut Object, property: &Property<D>) {
    object.push("value".into(), property.value.to_json());
    push_member::<D, _>(object, "valueType", &property.value_type);
    push_member::<D, _>(object, "unitCode", &property.unit_code);
}

/// Writes the payload member and the members only some kinds have.
fn write_payload<D: WriteDirection>(object: &mut Object, attribute: &Attribute<D>, form: Lossless) {
    match attribute {
        Attribute::Property(property) => write_property_payload(object, property),
        Attribute::GeoProperty(property) => {
            object.push("value".into(), property.value.to_json());
            push_member::<D, _>(object, "valueType", &property.value_type);
            push_member::<D, _>(object, "unitCode", &property.unit_code);
        }
        Attribute::LanguageProperty(property) => {
            object.push("languageMap".into(), property.language_map.to_json());
            push_member::<D, _>(object, "valueType", &property.value_type);
        }
        Attribute::VocabProperty(property) => {
            object.push("vocab".into(), property.vocab.to_json(|term| Json::from(term.to_string())));
            push_member::<D, _>(object, "valueType", &property.value_type);
        }
        Attribute::ListProperty(property) => {
            object.push("valueList".into(), Json::Array(property.value_list.iter().map(Value::to_json).collect()));
            push_member::<D, _>(object, "valueType", &property.value_type);
            push_member::<D, _>(object, "unitCode", &property.unit_code);
        }
        Attribute::JsonProperty(property) => {
            // The raw JSON is carried verbatim, so the tree is copied as it is.
            object.push("json".into(), property.json.to_json());
            push_member::<D, _>(object, "valueType", &property.value_type);
        }
        Attribute::Relationship(relationship) => {
            write_object_type::<D>(object, &relationship.object_type);
            object.push("object".into(), relationship.object.to_json(|id| Json::from(id.as_str())));
        }
        Attribute::ListRelationship(relationship) => {
            write_object_type::<D>(object, &relationship.object_type);
            // Normalized form wraps each target as `{"object": id}` (clause 4.5.22.2); concise form writes the bare
            // URIs (clause 4.5.22.3, and the concise example of clause C.2.2).
            let entries = relationship.object_list.iter().map(|id| match form {
                Lossless::Normalized => object_entry(id),
                Lossless::Concise => Json::from(id.as_str()),
            });
            object.push("objectList".into(), Json::Array(entries.collect()));
        }
    }
}

/// Writes `objectType` when present, or the NGSI-LD Null a fragment deletes it with.
fn write_object_type<D: WriteDirection>(object: &mut Object, object_type: &D::Member<OneOrMany<EntityTypeName>>) {
    if let Some(json) = D::write_member(object_type, |types| types.to_json(|name| Json::from(name.to_string()))) {
        object.push("objectType".into(), json);
    }
}

/// One `objectList` entry in the object form clause 4.5.22.2 requires: `{"object": id}`.
pub(crate) fn object_entry(id: &EntityId) -> Json {
    let mut entry = Object::new();
    entry.push("object".into(), Json::from(id.as_str()));
    Json::Object(entry)
}

/// Writes the shared members that come before the output-only ones: `datasetId`, `observedAt`, `expiresAt`.
fn write_meta_head<D: WriteDirection>(object: &mut Object, meta: &AttributeMeta<D>) {
    push_display(object, "datasetId", meta.dataset_id.as_ref());
    push_member::<D, _>(object, "observedAt", &meta.observed_at);
    push_member::<D, _>(object, "expiresAt", &meta.expires_at);
}

/// Writes the output-only members of the attribute's kind.
fn write_kind_output<D: WriteDirection>(object: &mut Object, attribute: &Attribute<D>, form: Lossless) {
    match attribute {
        Attribute::Property(property) => D::write_kind_output(&property.output, object, form),
        Attribute::GeoProperty(property) => D::write_kind_output(&property.output, object, form),
        Attribute::LanguageProperty(property) => D::write_kind_output(&property.output, object, form),
        Attribute::VocabProperty(property) => D::write_kind_output(&property.output, object, form),
        Attribute::ListProperty(property) => D::write_kind_output(&property.output, object, form),
        Attribute::JsonProperty(property) => D::write_kind_output(&property.output, object, form),
        Attribute::Relationship(relationship) => D::write_kind_output(&relationship.output, object, form),
        Attribute::ListRelationship(relationship) => D::write_kind_output(&relationship.output, object, form),
    }
}

/// Writes the shared members that come after the output-only members of the kind: the output-only members every
/// kind has, `ngsildproof`, and the sub-attributes.
fn write_meta_tail<D: WriteDirection>(object: &mut Object, meta: &AttributeMeta<D>, form: Lossless) {
    D::write_attribute_output(&meta.output, object);
    if let Some(proof) = &meta.proof {
        object.push("ngsildproof".into(), write_proof(proof, form));
    }
    write_attributes(object, &meta.attributes, form);
}

/// Writes an entity in canonical member order, without `@context`: the body of a request, whose context the
/// request pipeline delivers (clause 6.3.5).
pub(crate) fn write_entity<D: WriteEntityDirection, C: Completeness>(entity: &Entity<D, C>, form: Lossless) -> Object {
    let mut object = Object::new();
    write_identity::<C>(&mut object, &entity.id, &entity.entity_type);
    if let Some(scope) = &entity.scope {
        object.push("scope".into(), scope.to_json(|scope| Json::from(scope.to_string())));
    }
    push_display(&mut object, "expiresAt", entity.expires_at.as_ref());
    write_attributes(&mut object, &entity.attributes, form);
    D::write_entity_output(&entity.output, &mut object);
    object
}

/// Writes the identifying members of an entity, `id` then `type`, each one only if the entity holds it (a
/// projection may have removed them, clause 4.21).
pub(crate) fn write_identity<C: Completeness>(object: &mut Object, id: &C::Member<EntityId>, entity_type: &C::Member<OneOrMany<EntityTypeName>>) {
    if let Some(id) = C::get(Seal::new(), id) {
        object.push("id".into(), Json::from(id.as_str()));
    }
    if let Some(entity_type) = C::get(Seal::new(), entity_type) {
        object.push("type".into(), entity_type.to_json(|name| Json::from(name.to_string())));
    }
}

/// Appends the `@context`, last, as every example of Annex C places it.
pub(crate) fn write_context(object: &mut Object, context: Option<&Context>) {
    if let Some(context) = context {
        object.push(CONTEXT.into(), context.to_json());
    }
}

/// Writes a fragment, without `@context`: the body of a request, whose context the request pipeline delivers
/// (clause 6.3.5).
pub(crate) fn write_fragment<C: Completeness>(fragment: &EntityFragment<C>, form: Lossless) -> Object {
    let mut object = Object::new();
    write_fragment_head(
        &mut object,
        C::get(Seal::new(), &fragment.id),
        C::get(Seal::new(), &fragment.entity_type),
        fragment.scope.as_ref(),
    );
    push_display(&mut object, "expiresAt", fragment.expires_at.as_ref());
    write_attributes(&mut object, &fragment.attributes, form);
    object
}

/// Writes the entity members a fragment of any representation may hold, in canonical order: `id`, `type` and
/// `scope`, each only if the fragment sets it (clause 5.4).
pub(crate) fn write_fragment_head(object: &mut Object, id: Option<&EntityId>, entity_type: Option<&OneOrMany<EntityTypeName>>, scope: Option<&ScopeChange>) {
    if let Some(id) = id {
        object.push("id".into(), Json::from(id.as_str()));
    }
    if let Some(entity_type) = entity_type {
        object.push("type".into(), entity_type.to_json(|name| Json::from(name.to_string())));
    }
    if let Some(scope) = scope {
        let value = match scope {
            ScopeChange::Set(scope) => scope.to_json(|scope| Json::from(scope.to_string())),
            ScopeChange::Delete => Json::from(NGSI_LD_NULL.as_str()),
        };
        object.push("scope".into(), value);
    }
}
