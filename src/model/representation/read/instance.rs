//! Reading one attribute instance of a known kind: its payload, the members only some kinds have, the members
//! every kind shares, and its sub-attributes in document order.

use crate::model::{
    Attribute,
    AttributeKind,
    AttributeMeta,
    DocumentError,
    DocumentErrorKind,
    EntityId,
    EntityTypeName,
    GeoProperty,
    JsonProperty,
    LanguageMap,
    LanguageProperty,
    ListProperty,
    ListRelationship,
    Lossless,
    MemberError,
    Property,
    RawJson,
    Relationship,
    ReservedMember,
    Value,
    VocabProperty,
    VocabTerm,
    null::NGSI_LD_NULL,
    reader::{array, date_time, identifier, identifier_text, once, one_or_many, parsed, required, string, unexpected, value},
    representation::{
        kind_output::KindOutputMember,
        member::{entity_id, geo_value},
        proof::read_proof,
        read::{
            direction::{OutputMember, ReadDirection},
            read_attribute_member,
        },
    },
};
use jstrict::{Object, Value as Json, object::Entry};

/// Reads a normalized attribute: an object whose `type` names the kind, or the bare NGSI-LD Null a notification
/// writes for a deleted attribute with nothing more to report (clause 5.8.6), read as a Property holding the Null,
/// the kind concise rule 6 gives a bare value. The direction then admits or refuses the Null.
pub(super) fn read_normalized<D: ReadDirection>(json: Json) -> Result<Attribute<D>, DocumentError> {
    let object = match json {
        Json::Object(object) => object,
        Json::String(text) if text.as_str() == NGSI_LD_NULL.as_str() => {
            return Ok(Attribute::Property(Property::plain(Value::Text(NGSI_LD_NULL.as_str().to_owned()))));
        }
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) => {
            return Err(DocumentError::top(MemberError::NotAnObject.into()));
        }
    };
    let kind = match object.get_unique("type").ok().flatten() {
        Some(Json::String(found)) => found
            .as_str()
            .parse::<AttributeKind>()
            .map_err(|_| DocumentError::top(DocumentErrorKind::UnknownAttributeType { found: found.to_string() }))?,
        Some(_) | None => return Err(DocumentError::top(DocumentErrorKind::MissingType)),
    };
    read_known_kind(kind, object, Lossless::Normalized)
}

/// The members of one attribute instance, sorted by role.
struct InstanceMembers<D: ReadDirection> {
    /// The member that carries the instance's content under the kind's name (`value`, `object`, `languageMap`, ...).
    payload: Option<Json>,
    /// `unitCode`, read only for a kind that allows it.
    unit_code: Option<Json>,
    /// `valueType`, read only for a property kind.
    value_type: Option<Json>,
    /// `objectType`, read only for a relationship kind.
    object_type: Option<Json>,
    /// `observedAt`.
    observed_at: Option<Json>,
    /// `expiresAt`.
    expires_at: Option<Json>,
    /// The output-only members of the kind (`previousValue` and the like), in document order.
    kind_output: Vec<(KindOutputMember, Json)>,
    /// The members every kind shares, and the sub-attributes.
    meta: AttributeMeta<D>,
}

/// Reads one attribute instance of a known kind.
pub(super) fn read_known_kind<D: ReadDirection>(kind: AttributeKind, object: Object, form: Lossless) -> Result<Attribute<D>, DocumentError> {
    let members = sort_members::<D>(kind, object, form)?;
    let payload = required(members.payload, kind.payload_member())?;
    let unit_code = D::read_member(members.unit_code, |json| {
        string(json, "unitCode")?
            .parse()
            .map_err(|source| DocumentError::top(DocumentErrorKind::UnitCode { source }))
    })?;
    let value_type = D::read_member(members.value_type, |json| parsed::<VocabTerm, _>(json, "valueType"))?;
    let object_type = D::read_member(members.object_type, |json| {
        one_or_many(json, |item| parsed::<EntityTypeName, _>(item, "objectType"))
    })?;
    let mut meta = members.meta;
    meta.observed_at = D::read_member(members.observed_at, |json| date_time(json, "observedAt"))?;
    meta.expires_at = D::read_member(members.expires_at, |json| date_time(json, "expiresAt"))?;
    let kind_output = members.kind_output;
    Ok(match kind {
        AttributeKind::Property => Attribute::Property(Property {
            value: value(payload)?,
            unit_code,
            value_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
        AttributeKind::GeoProperty => Attribute::GeoProperty(GeoProperty {
            value: geo_value(&payload)?,
            unit_code,
            value_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
        AttributeKind::LanguageProperty => Attribute::LanguageProperty(LanguageProperty {
            language_map: LanguageMap::from_json(payload).map_err(|source| DocumentError::top(DocumentErrorKind::LanguageMap { source }))?,
            value_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
        AttributeKind::VocabProperty => Attribute::VocabProperty(VocabProperty {
            vocab: one_or_many(payload, |item| parsed::<VocabTerm, _>(item, "vocab"))?,
            value_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
        AttributeKind::ListProperty => Attribute::ListProperty(ListProperty {
            value_list: array(payload)?.into_iter().map(value).collect::<Result<_, _>>()?,
            unit_code,
            value_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
        AttributeKind::JsonProperty => {
            let json = RawJson::from_json(payload);
            // A deleted `JsonProperty` in a notification holds the NGSI-LD Null (clause 5.8.6); the direction admits
            // or refuses it.
            if !json.is_json_property_payload() && !json.is_ngsi_ld_null() {
                return Err(DocumentError::top(DocumentErrorKind::JsonShape));
            }
            Attribute::JsonProperty(JsonProperty {
                json,
                value_type,
                meta,
                output: D::read_kind_output(kind_output, form)?,
            })
        }
        AttributeKind::Relationship => Attribute::Relationship(Relationship {
            object: one_or_many(payload, |item| entity_id(item, "object"))?,
            object_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
        AttributeKind::ListRelationship => Attribute::ListRelationship(ListRelationship {
            object_list: array(payload)?.into_iter().map(list_object).collect::<Result<_, _>>()?,
            object_type,
            meta,
            output: D::read_kind_output(kind_output, form)?,
        }),
    })
}

/// Sorts the members of an instance into its payload, kind-specific members, shared members and sub-attributes.
fn sort_members<D: ReadDirection>(kind: AttributeKind, object: Object, form: Lossless) -> Result<InstanceMembers<D>, DocumentError> {
    let mut members = InstanceMembers {
        payload: None,
        unit_code: None,
        value_type: None,
        object_type: None,
        observed_at: None,
        expires_at: None,
        kind_output: Vec::new(),
        meta: AttributeMeta::empty(),
    };
    let property_kind = !matches!(kind, AttributeKind::Relationship | AttributeKind::ListRelationship);
    let unit_kind = matches!(kind, AttributeKind::Property | AttributeKind::GeoProperty | AttributeKind::ListProperty);
    for Entry { key, value } in object {
        // Member names are free-form here, because every name the specification does not reserve is a
        // sub-attribute; the reserved ones are told apart first.
        match key.as_str() {
            "type" => {}
            payload if payload == kind.payload_member() => once(&mut members.payload, payload, value)?,
            "unitCode" if unit_kind => once(&mut members.unit_code, "unitCode", value)?,
            "valueType" if property_kind => once(&mut members.value_type, "valueType", value)?,
            "objectType" if !property_kind => once(&mut members.object_type, "objectType", value)?,
            "datasetId" => {
                once(&mut members.meta.dataset_id, "datasetId", identifier(value, "datasetId")?)?;
            }
            "observedAt" => once(&mut members.observed_at, "observedAt", value)?,
            "expiresAt" => once(&mut members.expires_at, "expiresAt", value)?,
            "ngsildproof" => {
                let proof = read_proof::<D>(value, form).map_err(|error| error.within("ngsildproof"))?;
                once(&mut members.meta.proof, "ngsildproof", Box::new(proof))?;
            }
            other => match other.parse::<OutputMember>() {
                Ok(output) => D::read_attribute_output(&mut members.meta.output, output, value)?,
                Err(_) if let Ok(member) = other.parse::<KindOutputMember>() => members.kind_output.push((member, value)),
                Err(_) if ReservedMember::find(other).is_some() => {
                    return Err(unexpected(other));
                }
                Err(_) => read_attribute_member(&mut members.meta.attributes, other, value, form)?,
            },
        }
    }
    Ok(members)
}

/// Reads one `objectList` entry: `{"object": id}` (clause 4.5.22.2) or, in concise form, the bare id
/// (clause 4.5.22.3).
pub(crate) fn list_object(json: Json) -> Result<EntityId, DocumentError> {
    match json {
        Json::Object(object) => match object.get_unique("object").ok().flatten() {
            Some(Json::String(id)) => identifier_text(id.as_str()),
            Some(_) | None => Err(DocumentError::top(MemberError::MissingMember { member: "object" }.into())),
        },
        other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => entity_id(other, "objectList"),
    }
}
