//! Reading an attribute fragment that sets no payload (NGSI-LD clauses 5.4, 5.5.8 and 5.6.4.4).
//!
//! The body of Partial Attribute Update is an Attribute Fragment, an object whose members are the ones of the attribute
//! to change, each optional. The concise inference rules read an object as the value of a Property when nothing in it
//! names a kind (clause 4.5.2.3), which is right for an attribute inside an entity and wrong for a fragment: there,
//! `{"observedAt": "2022-03-14T12:51:02Z"}` changes the `observedAt` of the target and leaves its value alone.

use crate::model::{
    AttributeKind,
    AttributeMembers,
    DocumentError,
    Lossless,
    Patch,
    reader::unexpected,
    representation::{
        read::instance::{KindMembers, read_kind_members, sort_members},
        shape::KindMember,
    },
};
use jstrict::{Object, Value as Json};
use strum::IntoEnumIterator;

/// What a fragment object sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FragmentShape {
    /// A payload: the object is read as an attribute, by the rules of its representation.
    Payload,
    /// No payload, and the kind the fragment names with `type`, if it does.
    Members(Option<AttributeKind>),
}

impl FragmentShape {
    /// Tells whether the object holds the payload of a kind. What the object holds besides the payload, and whether it
    /// is a valid attribute, is for the reader of attributes to decide, so an object whose `type` is not a kind, or is
    /// given twice, has a payload as far as this is concerned.
    pub(crate) fn of(object: &Object) -> Self {
        match object.get_unique("type") {
            Ok(None) => {
                let names_payload = object.contains_key("value") || KindMember::iter().any(|member| object.contains_key(member.as_str()));
                if names_payload { Self::Payload } else { Self::Members(None) }
            }
            Ok(Some(Json::String(text))) => match text.as_str().parse::<AttributeKind>() {
                Ok(kind) if !object.contains_key(kind.payload_member()) => Self::Members(Some(kind)),
                Ok(_) | Err(_) => Self::Payload,
            },
            Ok(Some(Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_))) | Err(_) => Self::Payload,
        }
    }
}

/// Reads an object that sets no payload.
pub(crate) fn read_members(kind: Option<AttributeKind>, object: Object, form: Lossless) -> Result<AttributeMembers, DocumentError> {
    let members = sort_members::<Patch>(kind, object, form)?;
    if let Some(&(member, _)) = members.kind_output.first() {
        return Err(unexpected(member.as_str()));
    }
    let KindMembers {
        unit_code,
        value_type,
        object_type,
        meta,
    } = read_kind_members::<Patch>(
        members.unit_code,
        members.value_type,
        members.object_type,
        members.observed_at,
        members.expires_at,
        members.meta,
    )?;
    Ok(AttributeMembers {
        kind,
        unit_code,
        value_type,
        object_type,
        meta,
    })
}
