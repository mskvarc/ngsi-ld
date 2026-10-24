//! Reading one concise attribute: inferring its kind by the rules listed in the documentation of the parent
//! module.

use crate::model::{
    Attribute,
    AttributeKind,
    AttributeMeta,
    DocumentError,
    DocumentErrorKind,
    GeoProperty,
    GeoValue,
    Lossless,
    MemberError,
    Property,
    ValueError,
    geometry::Geometry,
    reader::value,
    representation::{
        read::{direction::ReadDirection, instance::read_known_kind},
        shape::KindMember,
    },
};
use jstrict::Value as Json;
use strum::IntoEnumIterator;

/// Reads a concise attribute, inferring its kind.
pub(super) fn read_concise<D: ReadDirection>(json: Json) -> Result<Attribute<D>, DocumentError> {
    let object = match json {
        Json::Object(object) => object,
        Json::Null => return Err(DocumentError::top(MemberError::Value { source: ValueError::Null }.into())),
        other @ (Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => return bare_property(other),
    };
    // Rule 1.
    if let Some(Json::String(found)) = object.get_unique("type").ok().flatten()
        && let Ok(kind) = found.as_str().parse::<AttributeKind>()
    {
        return read_known_kind(kind, object, Lossless::Concise);
    }
    // Rule 2.
    let as_json = Json::Object(object);
    if let Ok(geometry) = Geometry::from_json(&as_json) {
        return Ok(Attribute::GeoProperty(GeoProperty {
            value: GeoValue::Geometry(geometry),
            unit_code: Default::default(),
            value_type: Default::default(),
            meta: AttributeMeta::empty(),
            output: Default::default(),
        }));
    }
    // The JSON was built from an object just above, so it is one.
    let Json::Object(object) = as_json else {
        return Err(DocumentError::top(MemberError::NotAnObject.into()));
    };
    // Rule 3.
    if let Some(found) = object.get_unique("type").ok().flatten() {
        return Err(DocumentError::top(DocumentErrorKind::UnknownAttributeType { found: found.to_string() }));
    }
    // Rule 4.
    let mut kinds = KindMember::iter().filter(|member| object.contains_key(member.as_str()));
    match (kinds.next(), kinds.next()) {
        (Some(first), Some(second)) => Err(DocumentError::top(DocumentErrorKind::AmbiguousKind {
            first: first.as_str(),
            second: second.as_str(),
        })),
        (Some(member), None) => read_known_kind(member.kind(), object, Lossless::Concise),
        // Rule 5.
        (None, None | Some(_)) if object.contains_key("value") => {
            let geometric = object
                .get_unique("value")
                .ok()
                .flatten()
                .is_some_and(|value| Geometry::from_json(value).is_ok());
            let kind = if geometric { AttributeKind::GeoProperty } else { AttributeKind::Property };
            read_known_kind(kind, object, Lossless::Concise)
        }
        // Rule 6.
        (None, None | Some(_)) => bare_property(Json::Object(object)),
    }
}

/// A Property that is nothing but its value (clause 4.5.2.3).
fn bare_property<D: ReadDirection>(json: Json) -> Result<Attribute<D>, DocumentError> {
    Ok(Attribute::Property(Property::plain(value(json)?)))
}
