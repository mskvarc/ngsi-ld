//! Raw JSON: data the specification keeps outside JSON-LD interpretation.

use crate::model::{nesting::grow, null::NGSI_LD_NULL};
use jstrict::{Print, Value as Json};
use std::{
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    mem,
    str::FromStr,
};

/// The text is not valid JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("not valid JSON")]
pub struct InvalidJson;

/// A JSON value the crate carries verbatim, with its member order and number spelling: the `json` of a
/// `JsonProperty` (NGSI-LD clause 4.5.24), which is never expanded, the values of the foreign members of a geometry
/// (IETF RFC 7946 section 6.1), and inline `@context` objects.
///
/// ```
/// use ngsi_ld::model::RawJson;
///
/// let json: RawJson =
///     r#"{"id": "85a6cc52-0589-45f9", "value": "Overstay 60 minutes"}"#.parse()?;
/// assert_eq!(
///     json.to_string(),
///     r#"{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// Raw JSON nests as deep as the document that carries it, so it is cloned, compared, hashed and debug-printed on a
/// stack that grows with the depth.
// `Debug`, `Clone`, `PartialEq` and `Hash` are written out below: the JSON tree's own recurse once per level on the
// current stack.
#[derive(Eq)]
pub struct RawJson(Json);

impl RawJson {
    /// The JSON array of `items`, in order: the body of a query, whose items are documents rendered on their own.
    ///
    /// ```
    /// use ngsi_ld::model::RawJson;
    ///
    /// let items: [RawJson; 2] = [r#"{"id": "a"}"#.parse()?, "2".parse()?];
    /// assert_eq!(RawJson::array(items).to_string(), r#"[{"id":"a"},2]"#);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn array(items: impl IntoIterator<Item = RawJson>) -> RawJson {
        RawJson(Json::Array(items.into_iter().map(|item| item.0).collect()))
    }

    /// Whether the value is a JSON object.
    #[must_use]
    pub const fn is_object(&self) -> bool {
        self.0.is_object()
    }

    /// Whether the value is a JSON array whose items are all objects.
    #[must_use]
    pub fn is_array_of_objects(&self) -> bool {
        self.0.as_array().is_some_and(|items| items.iter().all(Json::is_object))
    }

    /// Whether the value is what a `JsonProperty` holds: an object or an array of objects (clause 4.5.24.2).
    pub(crate) fn is_json_property_payload(&self) -> bool {
        self.is_object() || self.is_array_of_objects()
    }

    /// Whether the value is the NGSI-LD Null, the `json` of a deleted `JsonProperty` in a notification (clauses
    /// 4.5.0 and 5.8.6).
    pub(crate) fn is_ngsi_ld_null(&self) -> bool {
        self.0.as_str() == Some(NGSI_LD_NULL.as_str())
    }

    /// Wraps a value read from a document.
    pub(crate) const fn from_json(json: Json) -> Self {
        Self(json)
    }

    /// Copies a value out of a document that keeps its own tree, on a stack that grows with the depth.
    pub(crate) fn copy_of(json: &Json) -> Self {
        Self(copy(json))
    }

    /// Returns a copy of the JSON tree, made on a stack that grows with the depth.
    pub(crate) fn to_json(&self) -> Json {
        copy(&self.0)
    }

    /// Takes the JSON tree.
    #[cfg(any(feature = "linked-data", feature = "http"))]
    pub(crate) fn into_json(self) -> Json {
        self.0
    }
}

/// Copies a JSON tree, each level on a stack that grows with the depth (see [`grow`]). Keys and scalars are cloned:
/// the copy owns them.
fn copy(json: &Json) -> Json {
    grow(|| match json {
        Json::Array(items) => Json::Array(items.iter().map(copy).collect()),
        Json::Object(object) => Json::Object(object.iter().map(|entry| (entry.key.clone(), copy(&entry.value))).collect()),
        scalar @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_)) => scalar.clone(),
    })
}

/// Compares two JSON trees as the tree's own `PartialEq` does, member order included, each level on a stack that
/// grows with the depth.
fn equal(left: &Json, right: &Json) -> bool {
    grow(|| match (left, right) {
        (Json::Array(left), Json::Array(right)) => left.len() == right.len() && left.iter().zip(right.iter()).all(|(left, right)| equal(left, right)),
        (Json::Object(left), Json::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| left.key == right.key && equal(&left.value, &right.value))
        }
        // A scalar compared with anything never descends.
        (scalar @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_)), other) => scalar == other,
        (Json::Array(_), Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_))
        | (Json::Object(_), Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => false,
    })
}

/// Feeds a JSON tree to `state`, consistently with [`equal`], each level on a stack that grows with the depth.
fn feed<H: Hasher>(json: &Json, state: &mut H) {
    grow(|| {
        mem::discriminant(json).hash(state);
        match json {
            Json::Null => {}
            Json::Boolean(boolean) => boolean.hash(state),
            Json::Number(number) => number.hash(state),
            Json::String(text) => text.hash(state),
            Json::Array(items) => {
                items.len().hash(state);
                items.iter().for_each(|item| feed(item, state));
            }
            Json::Object(object) => {
                object.len().hash(state);
                object.iter().for_each(|entry| {
                    entry.key.hash(state);
                    feed(&entry.value, state);
                });
            }
        }
    });
}

// No derive: the JSON tree's own `Clone` recurses once per level on the current stack.
impl Clone for RawJson {
    fn clone(&self) -> Self {
        Self(self.to_json())
    }
}

// See the `Clone` implementation for why this is not derived.
impl PartialEq for RawJson {
    fn eq(&self, other: &Self) -> bool {
        equal(&self.0, &other.0)
    }
}

// See the `Clone` implementation for why this is not derived.
impl Hash for RawJson {
    fn hash<H: Hasher>(&self, state: &mut H) {
        feed(&self.0, state);
    }
}

// See the `Clone` implementation for why this is not derived. The tree is shown as its compact JSON text, which
// printing writes without recursing on the current stack.
impl Debug for RawJson {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("RawJson").field(&format_args!("{self}")).finish()
    }
}

// No derive parses text into the wrapped JSON tree.
impl FromStr for RawJson {
    type Err = InvalidJson;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self).map_err(|_| InvalidJson)
    }
}

// The text form is the compact JSON form.
impl Display for RawJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0.compact_print(), f)
    }
}

#[cfg(test)]
mod tests;
