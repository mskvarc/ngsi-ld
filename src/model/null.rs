//! The NGSI-LD Null (NGSI-LD clause 4.5.0), and the rule of clause 5.5.4 that keeps it out of requests.
//!
//! Clause 4.5.0 encodes a deletion as `"urn:ngsi-ld:null"` wherever a URI string is a valid JSON value, as an
//! array holding that single string where an array is required, and as `{"@none": "urn:ngsi-ld:null"}` for a
//! language map. Clause 5.5.4 makes a request that uses it anywhere but in a fragment a `BadRequestData`: as a
//! first-level member value, as the `value` of a Property, the `object` of a Relationship or the `languageMap` of
//! a `LanguageProperty`, and as the value of a member of a JSON object that is a Property value. Clauses 4.5.20.2,
//! 4.5.21.2 and 4.5.22.2 give the shapes of the other kinds.

use crate::model::{
    Attribute,
    BuildProblem,
    Direction,
    EntityId,
    GeoValue,
    LanguageKey,
    LanguageMap,
    LanguageText,
    OneOrMany,
    ProblemKind,
    SimplifiedValue,
    Value,
    VocabTerm,
    evolution::TemporalValues,
    nesting::grow,
};
use indexmap::IndexMap;
use iri_rs::{Iri, iri};

/// The NGSI-LD Null of clause 4.5.0.
pub(crate) const NGSI_LD_NULL: Iri<&str> = iri!("urn:ngsi-ld:null");

/// Where a request attribute holds the NGSI-LD Null, which clause 5.5.4 refuses outside fragments.
///
/// A request attribute (and every sub-attribute, which has the same representation) holds the NGSI-LD Null in none
/// of these places:
///
/// - the `value` of a Property, or the value of a member of an object at any depth inside that `value`;
/// - the `languageMap` of a `LanguageProperty`, when its `@none` text is the Null;
/// - the `vocab` of a `VocabProperty`, the `object` of a Relationship, the `valueList` of a `ListProperty` and the
///   `objectList` of a `ListRelationship`, when they consist of the Null alone.
/// - the `value` of a `GeoProperty`, when it is the Null rather than a geometry.
/// - the `json` of a `JsonProperty`, when it is the Null rather than an object or an array of objects.
///
/// Inside an object or an array, the `json` of a `JsonProperty` is raw JSON the specification never interprets
/// (clause 4.5.24.2), so it is not checked. A fragment deletes through
/// [`Change::Delete`](crate::model::Change::Delete), never through a value that happens to be the Null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display)]
pub enum NullMember {
    /// The `value` of a Property is the Null.
    #[strum(serialize = "`value`")]
    Value,
    /// A member of an object inside the `value` of a Property is the Null.
    #[strum(serialize = "a member of `value`")]
    ValueMember,
    /// The `@none` text of a `languageMap` is the Null.
    #[strum(serialize = "`languageMap`")]
    LanguageMap,
    /// The `vocab` is the Null alone.
    #[strum(serialize = "`vocab`")]
    Vocab,
    /// The `valueList` is the Null alone.
    #[strum(serialize = "`valueList`")]
    ValueList,
    /// The `object` is the Null alone.
    #[strum(serialize = "`object`")]
    Object,
    /// The `objectList` is the Null alone.
    #[strum(serialize = "`objectList`")]
    ObjectList,
    /// The `json` is the Null.
    #[strum(serialize = "`json`")]
    Json,
    /// An instance of the evolution of `scope` is the Null of a removed scope (clause 4.5.6).
    #[strum(serialize = "`scope`")]
    Scope,
}

/// Whether `text` is the NGSI-LD Null.
fn is_null(text: &str) -> bool {
    text == NGSI_LD_NULL.as_str()
}

/// Whether a value is the NGSI-LD Null.
pub(crate) fn is_null_value(value: &Value) -> bool {
    match value {
        Value::Text(text) => is_null(text),
        Value::Boolean(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) | Value::Typed(_) => false,
    }
}

/// Whether a member of `members`, or of an object nested anywhere inside them, is the NGSI-LD Null.
fn object_holds_null(members: &IndexMap<String, Value>) -> bool {
    members.values().any(|member| is_null_value(member) || nested_object_holds_null(member))
}

/// Whether an object nested in `value`, directly or through arrays, has a member that is the NGSI-LD Null. Each
/// level of nesting is searched on a stack that grows with the depth (see [`grow`]).
fn nested_object_holds_null(value: &Value) -> bool {
    grow(|| match value {
        Value::Object(members) => object_holds_null(members),
        Value::Array(items) => items.iter().any(nested_object_holds_null),
        Value::Boolean(_) | Value::Number(_) | Value::Text(_) | Value::Typed(_) => false,
    })
}

/// Fails with `member` when `items` is the NGSI-LD Null alone.
fn refuse_null_alone<T>(items: &[T], text: impl Fn(&T) -> bool, member: NullMember) -> Result<(), NullMember> {
    match items {
        [only] if text(only) => Err(member),
        [] | [_] | [_, _, ..] => Ok(()),
    }
}

/// Checks the `value` of a Property.
///
/// # Errors
///
/// Returns the place of the NGSI-LD Null when the value is the Null or holds it as a member value.
pub(crate) fn check_value(value: &Value) -> Result<(), NullMember> {
    if is_null_value(value) {
        return Err(NullMember::Value);
    }
    match value {
        Value::Object(members) if object_holds_null(members) => Err(NullMember::ValueMember),
        Value::Boolean(_) | Value::Number(_) | Value::Text(_) | Value::Array(_) | Value::Object(_) | Value::Typed(_) => Ok(()),
    }
}

/// Checks the `languageMap` of a `LanguageProperty`.
///
/// # Errors
///
/// Returns [`NullMember::LanguageMap`] when its `@none` text is the NGSI-LD Null.
pub(crate) fn check_language_map(map: &LanguageMap) -> Result<(), NullMember> {
    match map.get(&LanguageKey::Default) {
        Some(LanguageText::Single(text)) if is_null(text) => Err(NullMember::LanguageMap),
        Some(LanguageText::Single(_) | LanguageText::Multiple(_)) | None => Ok(()),
    }
}

/// Checks the `vocab` of a `VocabProperty`.
///
/// # Errors
///
/// Returns [`NullMember::Vocab`] when it is the NGSI-LD Null alone.
pub(crate) fn check_vocab(vocab: &OneOrMany<VocabTerm>) -> Result<(), NullMember> {
    refuse_null_alone(vocab.as_slice(), |term| is_null(term.as_ref()), NullMember::Vocab)
}

/// Checks the `valueList` of a `ListProperty`.
///
/// # Errors
///
/// Returns [`NullMember::ValueList`] when it is the NGSI-LD Null alone.
pub(crate) fn check_value_list(value_list: &[Value]) -> Result<(), NullMember> {
    refuse_null_alone(value_list, is_null_value, NullMember::ValueList)
}

/// Checks the `object` of a Relationship.
///
/// # Errors
///
/// Returns [`NullMember::Object`] when it is the NGSI-LD Null alone.
pub(crate) fn check_object(object: &OneOrMany<EntityId>) -> Result<(), NullMember> {
    refuse_null_alone(object.as_slice(), |id| is_null(id.as_str()), NullMember::Object)
}

/// Checks the `objectList` of a `ListRelationship`.
///
/// # Errors
///
/// Returns [`NullMember::ObjectList`] when it is the NGSI-LD Null alone.
pub(crate) fn check_object_list(object_list: &[EntityId]) -> Result<(), NullMember> {
    refuse_null_alone(object_list, |id| is_null(id.as_str()), NullMember::ObjectList)
}

/// Checks the `value` of a `GeoProperty`.
///
/// # Errors
///
/// Returns [`NullMember::Value`] when it is the NGSI-LD Null.
pub(crate) const fn check_geo_value(value: &GeoValue) -> Result<(), NullMember> {
    match value {
        GeoValue::Geometry(_) => Ok(()),
        GeoValue::Null => Err(NullMember::Value),
    }
}

/// Checks the payload of an attribute of any kind; its sub-attributes are checked where they are read or built.
///
/// # Errors
///
/// Returns the place of the NGSI-LD Null in the payload.
pub(crate) fn check_attribute<D: Direction>(attribute: &Attribute<D>) -> Result<(), NullMember> {
    match attribute {
        Attribute::Property(property) => check_value(property.value()),
        Attribute::LanguageProperty(property) => check_language_map(property.language_map()),
        Attribute::VocabProperty(property) => check_vocab(property.vocab()),
        Attribute::ListProperty(property) => check_value_list(property.value_list()),
        Attribute::Relationship(relationship) => check_object(relationship.object()),
        Attribute::ListRelationship(relationship) => check_object_list(relationship.object_list()),
        Attribute::GeoProperty(property) => check_geo_value(property.value()),
        Attribute::JsonProperty(property) => {
            if property.json().is_ngsi_ld_null() {
                Err(NullMember::Json)
            } else {
                Ok(())
            }
        }
    }
}

/// Whether the payload of an attribute is the NGSI-LD Null in the shape its kind needs, which is how a broker
/// records a deleted attribute (clauses 4.5.0 and 5.8.6). A Property whose value only holds the Null as a member is
/// not deleted.
pub(crate) fn is_null_payload<D: Direction>(attribute: &Attribute<D>) -> bool {
    match attribute {
        Attribute::Property(property) => is_null_value(property.value()),
        Attribute::JsonProperty(property) => property.json().is_ngsi_ld_null(),
        Attribute::GeoProperty(_)
        | Attribute::LanguageProperty(_)
        | Attribute::VocabProperty(_)
        | Attribute::ListProperty(_)
        | Attribute::Relationship(_)
        | Attribute::ListRelationship(_) => check_attribute(attribute).is_err(),
    }
}

/// Whether a simplified value is the NGSI-LD Null in a shape some kind needs: the bare Null, the Null alone in an
/// array, or a language map whose `@none` text is the Null (clause 4.5.0).
pub(crate) fn is_null_simplified(value: &SimplifiedValue) -> bool {
    match value {
        SimplifiedValue::Value(Value::Array(items)) => check_value_list(items).is_err(),
        SimplifiedValue::Value(value @ (Value::Boolean(_) | Value::Number(_) | Value::Text(_) | Value::Object(_) | Value::Typed(_))) => is_null_value(value),
        SimplifiedValue::LanguageMap(map) => check_language_map(map).is_err(),
        SimplifiedValue::Vocab(vocab) => check_vocab(vocab).is_err(),
        SimplifiedValue::Json(json) => json.is_ngsi_ld_null(),
        SimplifiedValue::Dataset(_) | SimplifiedValue::Geometry(_) => false,
    }
}

/// Checks the data points of a simplified temporal attribute a client sends (clauses 4.5.9 and 5.6.11.3), each
/// payload as the payload of an instance of its kind.
///
/// # Errors
///
/// Returns the place of the first NGSI-LD Null found.
pub(crate) fn check_temporal_values(values: &TemporalValues) -> Result<(), NullMember> {
    match values {
        TemporalValues::Property(points) => points.iter().try_for_each(|point| check_value(&point.value)),
        TemporalValues::GeoProperty(points) => points.iter().try_for_each(|point| check_geo_value(&point.value)),
        TemporalValues::LanguageProperty(points) => points.iter().try_for_each(|point| check_language_map(&point.value)),
        TemporalValues::VocabProperty(points) => points.iter().try_for_each(|point| check_vocab(&point.value)),
        TemporalValues::ListProperty(points) => points.iter().try_for_each(|point| check_value_list(&point.value)),
        TemporalValues::JsonProperty(points) => points
            .iter()
            .try_for_each(|point| if point.value.is_ngsi_ld_null() { Err(NullMember::Json) } else { Ok(()) }),
        TemporalValues::Relationship(points) => points.iter().try_for_each(|point| check_object(&point.value)),
        TemporalValues::ListRelationship(points) => points.iter().try_for_each(|point| check_object_list(&point.value)),
    }
}

/// Checks a value of a simplified fragment (clause 4.5.4), which deletes through its own deletion and never through a
/// value that happens to be the NGSI-LD Null. The simplified form does not say which kind a value belongs to, so a
/// value is checked as a Property value and, when it is an array, as a list (clause 4.5.0).
///
/// # Errors
///
/// Returns the place of the NGSI-LD Null in the value.
pub(crate) fn check_simplified(value: &SimplifiedValue) -> Result<(), NullMember> {
    match value {
        SimplifiedValue::Value(value @ Value::Array(items)) => check_value(value).and_then(|()| check_value_list(items)),
        SimplifiedValue::Value(value @ (Value::Boolean(_) | Value::Number(_) | Value::Text(_) | Value::Object(_) | Value::Typed(_))) => check_value(value),
        SimplifiedValue::LanguageMap(map) => check_language_map(map),
        SimplifiedValue::Vocab(vocab) => check_vocab(vocab),
        SimplifiedValue::Dataset(instances) => instances.values().try_for_each(check_simplified),
        SimplifiedValue::Geometry(_) | SimplifiedValue::Json(_) => Ok(()),
    }
}

/// Records, for a builder, the NGSI-LD Null a check found.
pub(crate) fn report(check: Result<(), NullMember>, problems: &mut Vec<BuildProblem>) {
    if let Err(member) = check {
        problems.push(BuildProblem::top(ProblemKind::NgsiLdNull { member }));
    }
}

#[cfg(test)]
mod tests;
