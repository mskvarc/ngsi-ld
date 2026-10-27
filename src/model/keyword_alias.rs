//! The JSON-LD keywords `@id` and `@type` read as the members `id` and `type` they stand for (NGSI-LD clause 4.4).
//!
//! The core `@context` maps the term `id` to `@id` and the term `type` to `@type`. Clause 4.4 says that, since the
//! keywords are what JSON-LD typically uses, "they may also be used in NGSI-LD requests instead of `id` and `type`
//! respectively, wherever this is applicable", while a response only uses the terms. A reader therefore takes the
//! keyword for the term before it reads the members of an object, so that one reading serves both spellings and a
//! document is written back with the terms. A member given in both spellings is a member given twice.

use crate::model::{AttributeKind, DocumentError, MemberError, reader::duplicate};
use jstrict::{Object, Value as Json, object::Entry};

/// Reads `@id` and `@type` of an object that has the members `id` and `type`, such as an entity, an entity selector,
/// a subscription or a registration, as those members.
///
/// # Errors
///
/// Returns the error of a member given twice when the object holds a term and the keyword that stands for it.
pub(crate) fn unalias_members<K: From<MemberError>>(object: Object) -> Result<Object, DocumentError<K>> {
    rename(object, |key, _| match key {
        "@id" => Some("id"),
        "@type" => Some("type"),
        _ => None,
    })
}

/// Reads `@type` of an attribute as its `type`, when it names a kind of attribute.
///
/// An attribute has no `id`, and an object that holds `@type` beside `@value` is a JSON-LD value object (clause 4.6.3),
/// the value of a Property in concise representation, so `@type` is the keyword of an attribute only when the datatype
/// it holds is a kind of attribute.
///
/// # Errors
///
/// Returns the error of a member given twice when the object holds both `type` and `@type`.
pub(crate) fn unalias_attribute<K: From<MemberError>>(object: Object) -> Result<Object, DocumentError<K>> {
    rename(object, |key, value| (key == "@type" && names_a_kind(value)).then_some("type"))
}

/// Whether `value` is the name of a kind of attribute.
fn names_a_kind(value: &Json) -> bool {
    matches!(value, Json::String(text) if text.as_str().parse::<AttributeKind>().is_ok())
}

/// Renames the members `term` gives a name for, keeping the order of the members.
fn rename<K: From<MemberError>>(object: Object, term: impl Fn(&str, &Json) -> Option<&'static str>) -> Result<Object, DocumentError<K>> {
    if !object.iter().any(|entry| term(entry.key.as_str(), &entry.value).is_some()) {
        return Ok(object);
    }
    let mut renamed = Object::new();
    let mut terms = Vec::new();
    for Entry { key, value } in object {
        match term(key.as_str(), &value) {
            Some(name) => {
                terms.push(name);
                renamed.push(name.into(), value);
            }
            None => {
                renamed.push(key, value);
            }
        }
    }
    for name in terms {
        renamed.get_unique(name).map_err(|_| duplicate(name))?;
    }
    Ok(renamed)
}

#[cfg(test)]
mod tests;
