//! Writing the NGSI-LD Null (clause 4.5.0): the deletions a fragment sends, and the bare Null a notification writes
//! for a deleted attribute with nothing more to report (clause 5.8.6).

use crate::{
    model::{
        Attribute,
        AttributeKind,
        Deletion,
        Direction,
        EntityId,
        InstanceDeletion,
        LanguageMap,
        Lossless,
        keyword::NONE,
        null::{NGSI_LD_NULL, is_null_value},
        representation::write::{is_plain, push_display},
    },
    seal::Seal,
};
use jstrict::{Object, Value as Json};

/// The NGSI-LD Null in the shape `kind` needs for its payload member (clause 4.5.0): a string, an array with
/// one null for the list kinds, `{"@none": null}` for a language map.
fn null_payload(kind: AttributeKind) -> Json {
    match kind {
        AttributeKind::Property | AttributeKind::GeoProperty | AttributeKind::VocabProperty | AttributeKind::JsonProperty | AttributeKind::Relationship => {
            Json::from(NGSI_LD_NULL.as_str())
        }
        AttributeKind::ListProperty | AttributeKind::ListRelationship => Json::Array(vec![Json::from(NGSI_LD_NULL.as_str())].into()),
        AttributeKind::LanguageProperty => {
            let mut map = Object::new();
            map.push(NONE.into(), Json::from(NGSI_LD_NULL.as_str()));
            Json::Object(map)
        }
    }
}

/// Writes a deletion. A deletion of the whole member is the bare NGSI-LD Null; [`Instances`] keeps it from standing
/// among other instances, where it would match no attribute shape.
///
/// [`Instances`]: crate::model::Instances
pub(crate) fn write_deletion(deletion: &Deletion, form: Lossless) -> Json {
    match deletion {
        Deletion::Member => Json::from(NGSI_LD_NULL.as_str()),
        Deletion::Instance(instance) => Json::Object(write_instance_deletion(instance, form)),
    }
}

/// Writes the deletion of one instance as an attribute object whose payload is the NGSI-LD Null, the form an entity
/// fragment and the body of Partial Attribute Update take (clauses 4.5.2.2, 4.5.3.2 and 5.5.8).
pub(crate) fn write_instance_deletion(deletion: &InstanceDeletion, form: Lossless) -> Object {
    let mut object = Object::new();
    // In concise form a `GeoProperty` keeps its `type`: a null `value` alone reads as a Property.
    if form == Lossless::Normalized || deletion.kind == AttributeKind::GeoProperty {
        object.push("type".into(), Json::from(deletion.kind.as_str()));
    }
    object.push(deletion.kind.payload_member().into(), null_payload(deletion.kind));
    push_display(&mut object, "datasetId", deletion.dataset_id.as_ref());
    object
}

/// Whether `ids` is the NGSI-LD Null alone, the `object` or `objectList` of a deleted Relationship or
/// `ListRelationship` (clause 4.5.0).
fn is_null_id_alone(ids: &[EntityId]) -> bool {
    matches!(ids, [only] if only.as_str() == NGSI_LD_NULL.as_str())
}

/// The bare NGSI-LD Null a notification writes for a deleted attribute of any kind when it has nothing more to
/// report (clause 5.8.6): "only the name of the attribute as key and the URI "urn:ngsi-ld:null" as value", unless a
/// `datasetId`, a system attribute or a previous value must be given. The attribute qualifies when its payload is
/// the Null in the shape of its kind (clause 4.5.0) and it has no other member. A reader takes the bare Null for a
/// Property holding it, so the kind is not written back; the specification does not carry it either.
pub(crate) fn bare_deletion<D: Direction>(attribute: &Attribute<D>) -> Option<Json> {
    let deleted = match attribute {
        Attribute::Property(property) => {
            is_null_value(&property.value)
                && D::lacks(Seal::new(), &property.unit_code)
                && D::lacks(Seal::new(), &property.value_type)
                && is_plain(&property.meta, &property.output)
        }
        Attribute::GeoProperty(property) => {
            property.value.is_null()
                && D::lacks(Seal::new(), &property.unit_code)
                && D::lacks(Seal::new(), &property.value_type)
                && is_plain(&property.meta, &property.output)
        }
        Attribute::LanguageProperty(property) => {
            property.language_map == LanguageMap::ngsi_ld_null() && D::lacks(Seal::new(), &property.value_type) && is_plain(&property.meta, &property.output)
        }
        Attribute::VocabProperty(property) => {
            matches!(property.vocab.as_slice(), [only] if only.as_ref() == NGSI_LD_NULL.as_str())
                && D::lacks(Seal::new(), &property.value_type)
                && is_plain(&property.meta, &property.output)
        }
        Attribute::ListProperty(property) => {
            matches!(property.value_list.as_slice(), [only] if is_null_value(only))
                && D::lacks(Seal::new(), &property.unit_code)
                && D::lacks(Seal::new(), &property.value_type)
                && is_plain(&property.meta, &property.output)
        }
        Attribute::JsonProperty(property) => {
            property.json.is_ngsi_ld_null() && D::lacks(Seal::new(), &property.value_type) && is_plain(&property.meta, &property.output)
        }
        Attribute::Relationship(relationship) => {
            is_null_id_alone(relationship.object.as_slice())
                && D::lacks(Seal::new(), &relationship.object_type)
                && is_plain(&relationship.meta, &relationship.output)
        }
        Attribute::ListRelationship(relationship) => {
            is_null_id_alone(&relationship.object_list)
                && D::lacks(Seal::new(), &relationship.object_type)
                && is_plain(&relationship.meta, &relationship.output)
        }
    };
    deleted.then(|| Json::from(NGSI_LD_NULL.as_str()))
}
