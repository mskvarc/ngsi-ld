//! Reading the members every document shares: JSON objects, arrays, strings, booleans, numbers and counts,
//! tokens, timestamps, names, identifiers, URLs, scopes, durations, context source aliases, NGSI-LD Values,
//! geometries, members that hold one value or several, and `@context`s.
//!
//! Every reader of the crate goes through these. Each is generic over the error kind `K` of the document being
//! read, which holds a [`MemberError`].

use crate::model::{
    AttributeName,
    Context,
    ContextSourceAlias,
    DateTime,
    DocumentError,
    Duration,
    EntityTypeName,
    JsonType,
    MemberError,
    NameError,
    Number,
    OneOrMany,
    Scope,
    UriError,
    Value,
    geometry::Geometry,
    one_or_many::one_or_many_from_json,
    written_url::WrittenUrl,
};
use jstrict::{Object, Value as Json};
use std::str::FromStr;

/// An error of `kind` at the top of the member being read.
fn error<K: From<MemberError>>(kind: MemberError) -> DocumentError<K> {
    DocumentError::top(K::from(kind))
}

/// The error of a member whose value is not of the JSON type `expected`.
fn wrong_shape<K: From<MemberError>>(member: &'static str, expected: JsonType) -> DocumentError<K> {
    error(MemberError::WrongShape { member, expected })
}

/// Parses JSON text.
pub(crate) fn parse_json<K: From<MemberError>>(text: &str) -> Result<Json, DocumentError<K>> {
    text.parse().map_err(|_| error(MemberError::NotJson))
}

/// Takes the members of an object.
pub(crate) fn object<K: From<MemberError>>(json: Json) -> Result<Object, DocumentError<K>> {
    match json {
        Json::Object(object) => Ok(object),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) => Err(error(MemberError::NotAnObject)),
    }
}

/// Takes the elements of an array.
pub(crate) fn array<K: From<MemberError>>(json: Json) -> Result<Vec<Json>, DocumentError<K>> {
    match json {
        Json::Array(items) => Ok(items.into_vec()),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_) => Err(error(MemberError::NotAnArray)),
    }
}

/// Reads a string member.
pub(crate) fn string<K: From<MemberError>>(json: Json, member: &'static str) -> Result<String, DocumentError<K>> {
    match json {
        Json::String(text) => Ok(text.into_string()),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_) => Err(wrong_shape(member, JsonType::String)),
    }
}

/// Reads a boolean member.
pub(crate) fn boolean<K: From<MemberError>>(json: &Json, member: &'static str) -> Result<bool, DocumentError<K>> {
    match json {
        Json::Boolean(flag) => Ok(*flag),
        Json::Null | Json::Number(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => Err(wrong_shape(member, JsonType::Boolean)),
    }
}

/// Reads a number member.
pub(crate) fn number<K: From<MemberError>>(json: Json, member: &'static str) -> Result<Number, DocumentError<K>> {
    match json {
        Json::Number(number) => Ok(Number::from_json(number)),
        Json::Null | Json::Boolean(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => Err(wrong_shape(member, JsonType::Number)),
    }
}

/// Reads a count: a JSON number that is a whole number, zero or more.
pub(crate) fn count<K: From<MemberError>>(json: Json, member: &'static str) -> Result<u64, DocumentError<K>> {
    match json {
        Json::Number(number) => Number::from_json(number)
            .as_i64()
            .and_then(|count| u64::try_from(count).ok())
            .ok_or_else(|| wrong_shape(member, JsonType::Count)),
        Json::Null | Json::Boolean(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => Err(wrong_shape(member, JsonType::Count)),
    }
}

/// Reads a member whose value is one of the tokens the specification lists.
pub(crate) fn token<T: FromStr, K: From<MemberError>>(json: Json, member: &'static str) -> Result<T, DocumentError<K>> {
    let found = string(json, member)?;
    found.parse().map_err(|_| error(MemberError::InvalidToken { member, found }))
}

/// Reads a member whose value is the one token the specification fixes, such as the `type` of a document that has
/// a single type.
pub(crate) fn fixed_token<K: From<MemberError>>(json: Json, member: &'static str, expected: &str) -> Result<(), DocumentError<K>> {
    let found = string(json, member)?;
    if found == expected {
        Ok(())
    } else {
        Err(error(MemberError::InvalidToken { member, found }))
    }
}

/// Reads a timestamp member.
pub(crate) fn date_time<K: From<MemberError>>(json: Json, member: &'static str) -> Result<DateTime, DocumentError<K>> {
    string(json, member)?
        .parse()
        .map_err(|source| error(MemberError::Temporal { source: Box::new(source) }))
}

/// Reads one name or term.
pub(crate) fn parsed<T: FromStr<Err = NameError>, K: From<MemberError>>(json: Json, member: &'static str) -> Result<T, DocumentError<K>> {
    string(json, member)?.parse().map_err(|source| error(MemberError::Name { source }))
}

/// Reads a URI or an identifier wrapping one.
pub(crate) fn identifier<T: FromStr<Err = UriError>, K: From<MemberError>>(json: Json, member: &'static str) -> Result<T, DocumentError<K>> {
    string(json, member)?.parse().map_err(|source| error(MemberError::Identifier { source }))
}

/// Reads a URI or an identifier wrapping one from text already taken out of the document: a member name, or a
/// string read in place.
pub(crate) fn identifier_text<T: FromStr<Err = UriError>, K: From<MemberError>>(text: &str) -> Result<T, DocumentError<K>> {
    text.parse().map_err(|source| error(MemberError::Identifier { source }))
}

/// Reads the name of the attribute written as the member `key`, placing an error under that member.
pub(crate) fn attribute_name<K: From<MemberError>>(key: &str) -> Result<AttributeName, DocumentError<K>> {
    key.parse().map_err(|source| error(MemberError::Name { source }).within(key))
}

/// Reads a URL member.
pub(crate) fn url<K: From<MemberError>>(json: Json, member: &'static str) -> Result<url::Url, DocumentError<K>> {
    string(json, member)?.parse().map_err(|source| error(MemberError::Url { source }))
}

/// Reads a URL member that is written back as read.
pub(crate) fn written_url<K: From<MemberError>>(json: Json, member: &'static str) -> Result<WrittenUrl, DocumentError<K>> {
    url(json, member).map(WrittenUrl::from)
}

/// Reads one scope.
pub(crate) fn scope<K: From<MemberError>>(json: Json, member: &'static str) -> Result<Scope, DocumentError<K>> {
    string(json, member)?.parse().map_err(|source| error(MemberError::Scope { source }))
}

/// Reads a member that is one value or a non-empty array of them, each read with `item`.
pub(crate) fn one_or_many<T, K: From<MemberError>>(
    json: Json,
    item: impl FnMut(Json) -> Result<T, DocumentError<K>>,
) -> Result<OneOrMany<T>, DocumentError<K>> {
    one_or_many_from_json(json, item, empty())
}

/// Reads an entity `type`: one entity type name or a non-empty array of them.
pub(crate) fn entity_types<K: From<MemberError>>(json: Json) -> Result<OneOrMany<EntityTypeName>, DocumentError<K>> {
    one_or_many(json, |item| parsed(item, "type"))
}

/// Reads a `scope`: one scope or a non-empty array of them (clause 4.18).
pub(crate) fn scopes<K: From<MemberError>>(json: Json) -> Result<OneOrMany<Scope>, DocumentError<K>> {
    one_or_many(json, |item| scope(item, "scope"))
}

/// Reads an NGSI-LD Value (clause 3.1).
pub(crate) fn value<K: From<MemberError>>(json: Json) -> Result<Value, DocumentError<K>> {
    Value::from_json(json).map_err(|source| error(MemberError::Value { source }))
}

/// Reads a `GeoJSON` geometry (clause 4.7.1).
pub(crate) fn geometry<K: From<MemberError>>(json: &Json) -> Result<Geometry, DocumentError<K>> {
    Geometry::from_json(json).map_err(|source| error(MemberError::Geometry { source }))
}

/// Reads an ISO 8601 duration (clause 4.5.19.1).
pub(crate) fn duration<K: From<MemberError>>(json: Json, member: &'static str) -> Result<Duration, DocumentError<K>> {
    string(json, member)?.parse().map_err(|source| error(MemberError::Duration { source }))
}

/// Reads a `contextSourceAlias`, an IETF RFC 7230 token (Tables 5.2.9-1 and 5.2.40-1).
pub(crate) fn context_source_alias<K: From<MemberError>>(json: Json) -> Result<ContextSourceAlias, DocumentError<K>> {
    string(json, "contextSourceAlias")?
        .parse()
        .map_err(|source| error(MemberError::ContextSourceAlias { source }))
}

/// Reads an `@context`.
pub(crate) fn context<K: From<MemberError>>(json: Json) -> Result<Context, DocumentError<K>> {
    Context::from_json(json).map_err(|source| error(MemberError::Context { source }))
}

/// Stores `value` in an optional slot, refusing a second occurrence of the member.
pub(crate) fn once<T, K: From<MemberError>>(slot: &mut Option<T>, member: &str, value: T) -> Result<(), DocumentError<K>> {
    if slot.is_some() {
        return Err(duplicate(member));
    }
    *slot = Some(value);
    Ok(())
}

/// Takes a required member, which the document must have carried.
pub(crate) fn required<T, K: From<MemberError>>(slot: Option<T>, member: &'static str) -> Result<T, DocumentError<K>> {
    slot.ok_or_else(|| error(MemberError::MissingMember { member }))
}

/// Places the error of reading the value of member `key` under that member.
pub(crate) fn within<T, K>(key: &str, read: Result<T, DocumentError<K>>) -> Result<T, DocumentError<K>> {
    read.map_err(|error| error.within(key))
}

/// The error for a list or text the specification forbids to be empty.
pub(crate) fn empty<K: From<MemberError>>() -> DocumentError<K> {
    error(MemberError::Empty)
}

/// The error for a member that appears twice, such as an attribute name given twice.
pub(crate) fn duplicate<K: From<MemberError>>(member: &str) -> DocumentError<K> {
    error(MemberError::DuplicateMember { member: member.to_owned() })
}

/// The error for a member the document does not define.
pub(crate) fn unexpected<K: From<MemberError>>(member: &str) -> DocumentError<K> {
    error(MemberError::UnexpectedMember { member: member.to_owned() })
}

#[cfg(test)]
mod tests;
