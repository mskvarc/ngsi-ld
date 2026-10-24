//! Reading and writing the `ngsildproof` of an attribute (NGSI-LD clauses 4.5.2.2 and 4.5.3.2, Annex C.11): a
//! Property with the two non-reified members `entityIdSealed` and `entityTypeSealed`, which no other Property may
//! carry.

use crate::model::{
    Attribute,
    DocumentError,
    DocumentErrorKind,
    EntityTypeName,
    Lossless,
    Proof,
    reader::{object, once, one_or_many, parsed, required},
    representation::{
        member::entity_id,
        read::{direction::ReadDirection, read_instance},
        write::{direction::WriteDirection, write_property_object},
    },
};
use jstrict::{Object, Value as Json, object::Entry};

/// `entityIdSealed`.
const ENTITY_ID_SEALED: &str = "entityIdSealed";
/// `entityTypeSealed`.
const ENTITY_TYPE_SEALED: &str = "entityTypeSealed";

/// Reads an `ngsildproof`: the two sealed members, then the rest of the object as one Property instance, read
/// and admitted like any attribute of the direction.
pub(crate) fn read_proof<D: ReadDirection>(json: Json, form: Lossless) -> Result<Proof<D>, DocumentError> {
    let object = object(json)?;
    let mut entity_id_sealed = None;
    let mut entity_type_sealed = None;
    let mut property = Object::new();
    for Entry { key, value } in object {
        if key.as_str() == ENTITY_ID_SEALED {
            once(&mut entity_id_sealed, ENTITY_ID_SEALED, entity_id(value, ENTITY_ID_SEALED)?)?;
        } else if key.as_str() == ENTITY_TYPE_SEALED {
            let types = one_or_many(value, |item| parsed::<EntityTypeName, _>(item, ENTITY_TYPE_SEALED))?;
            once(&mut entity_type_sealed, ENTITY_TYPE_SEALED, types)?;
        } else {
            property.push(key, value);
        }
    }
    let property = match read_instance::<D>(Json::Object(property), form)? {
        Attribute::Property(property) => property,
        other @ (Attribute::GeoProperty(_)
        | Attribute::LanguageProperty(_)
        | Attribute::VocabProperty(_)
        | Attribute::ListProperty(_)
        | Attribute::JsonProperty(_)
        | Attribute::Relationship(_)
        | Attribute::ListRelationship(_)) => return Err(DocumentError::top(DocumentErrorKind::ProofNotAProperty { found: other.kind() })),
    };
    if !property.value().is_object() {
        return Err(DocumentError::top(DocumentErrorKind::ProofNotAnObject));
    }
    Ok(Proof {
        entity_id_sealed: required(entity_id_sealed, ENTITY_ID_SEALED)?,
        entity_type_sealed: required(entity_type_sealed, ENTITY_TYPE_SEALED)?,
        property,
    })
}

/// Writes an `ngsildproof` as an object, the sealed members right after the `type`, as Annex C.11 places them.
pub(crate) fn write_proof<D: WriteDirection>(proof: &Proof<D>, form: Lossless) -> Json {
    let mut entries = write_property_object(&proof.property, form).into_iter().peekable();
    let mut object = Object::new();
    if let Some(entry) = entries.next_if(|entry| entry.key.as_str() == "type") {
        object.push_entry(entry);
    }
    object.push(ENTITY_ID_SEALED.into(), Json::from(proof.entity_id_sealed.as_str()));
    object.push(ENTITY_TYPE_SEALED.into(), proof.entity_type_sealed.to_json(|name| Json::from(name.to_string())));
    entries.for_each(|entry| {
        object.push_entry(entry);
    });
    Json::Object(object)
}
