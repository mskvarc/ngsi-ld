//! Reading the attributes of a fragment (NGSI-LD clauses 5.4, 5.5.8 and 5.5.12): each instance is a change, which
//! writes an attribute or deletes one through the NGSI-LD Null.
//!
//! The Null deletes in the two shapes a fragment writes: the bare `"urn:ngsi-ld:null"` as the whole member value,
//! deleting the attribute (clauses 4.5.3.3 and 4.5.18.3, and clause 5.4 example 2), and an instance whose payload
//! is the Null in the shape of its kind (clause 4.5.0), with no member besides `type` and `datasetId`, deleting that
//! instance (clause 5.5.8). A Null nested in the value of a Property is how a merge removes a member of that value
//! (clause 5.5.12), so such an attribute is written as it is.

use crate::model::{
    Attribute,
    AttributeMeta,
    Change,
    Deletion,
    DocumentError,
    DocumentErrorKind,
    GeoProperty,
    InstanceDeletion,
    JsonProperty,
    LanguageProperty,
    ListProperty,
    ListRelationship,
    Lossless,
    MemberChange,
    NullMember,
    Patch,
    Property,
    Relationship,
    VocabProperty,
    member_change::read_change,
    null::{self, NGSI_LD_NULL},
    reader::unexpected,
    representation::{
        kind_output::{KindOutputMember, KindOutputRepresentation},
        read::{
            direction::{OutputMember, ReadDirection},
            read_unadmitted,
        },
    },
};
use jstrict::Value as Json;
use std::fmt::Debug;

impl ReadDirection for Patch {
    /// A member whose value is the NGSI-LD Null is deleted (clause 5.5.8).
    fn read_member<T: Debug + Clone + PartialEq + Eq>(
        json: Option<Json>,
        read: impl FnOnce(Json) -> Result<T, DocumentError>,
    ) -> Result<Option<MemberChange<T>>, DocumentError> {
        json.map(|json| read_change(json, read)).transpose()
    }

    fn read_attribute_output((): &mut Self::AttributeOutput, member: OutputMember, _: Json) -> Result<(), DocumentError> {
        Err(unexpected(<&str>::from(member)))
    }

    fn read_kind_output<K: KindOutputRepresentation>(members: Vec<(KindOutputMember, Json)>, _: Lossless) -> Result<(), DocumentError> {
        members.first().map_or(Ok(()), |&(member, _)| Err(unexpected(member.as_str())))
    }

    /// A fragment deletes through a [`Change::Delete`], so an attribute it writes holds the NGSI-LD Null only nested
    /// in a Property value, where a merge removes the member that holds it (clause 5.5.12).
    fn admit(attribute: &Attribute<Self>) -> Result<(), DocumentError> {
        match null::check_attribute(attribute) {
            Ok(()) | Err(NullMember::ValueMember) => Ok(()),
            Err(member) => Err(DocumentError::top(DocumentErrorKind::NgsiLdNull { member })),
        }
    }

    fn read_slot(json: Json, form: Lossless) -> Result<Self::Slot, DocumentError> {
        if matches!(&json, Json::String(text) if text.as_str() == NGSI_LD_NULL.as_str()) {
            return Ok(Change::Delete(Deletion::Member));
        }
        let mut attribute = read_unadmitted::<Self>(json, form)?;
        match null::check_attribute(&attribute) {
            Ok(()) | Err(NullMember::ValueMember) => Ok(Change::Set(attribute)),
            Err(_) if deletes_only(&attribute) => Ok(Change::Delete(Deletion::Instance(InstanceDeletion {
                kind: attribute.kind(),
                dataset_id: attribute.meta_mut().dataset_id.take(),
            }))),
            Err(member) => Err(DocumentError::top(DocumentErrorKind::NgsiLdNull { member })),
        }
    }
}

/// Whether an attribute whose payload is the NGSI-LD Null holds nothing a deletion does not write: no member besides
/// its kind, its payload and its dataset id.
fn deletes_only(attribute: &Attribute<Patch>) -> bool {
    let bare_meta = |meta: &AttributeMeta<Patch>| meta.observed_at.is_none() && meta.expires_at.is_none() && meta.proof.is_none() && meta.attributes.is_empty();
    match attribute {
        Attribute::Property(Property {
            unit_code, value_type, meta, ..
        })
        | Attribute::GeoProperty(GeoProperty {
            unit_code, value_type, meta, ..
        })
        | Attribute::ListProperty(ListProperty {
            unit_code, value_type, meta, ..
        }) => unit_code.is_none() && value_type.is_none() && bare_meta(meta),
        Attribute::LanguageProperty(LanguageProperty { value_type, meta, .. })
        | Attribute::VocabProperty(VocabProperty { value_type, meta, .. })
        | Attribute::JsonProperty(JsonProperty { value_type, meta, .. }) => value_type.is_none() && bare_meta(meta),
        Attribute::Relationship(Relationship { object_type, meta, .. }) | Attribute::ListRelationship(ListRelationship { object_type, meta, .. }) => {
            object_type.is_none() && bare_meta(meta)
        }
    }
}
