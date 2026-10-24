//! Lists of attribute names a subscription holds (`watchedAttributes`, `expandValues`, `jsonKeys`, and the
//! notification's `attributes`): the comma-separated form two of them take, and the array form of the others.

use crate::model::AttributeName;
use jstrict::Value as Json;
use std::fmt::Display;

/// Writes names as one comma-separated string, the form Table 5.2.12-1 gives `expandValues` and `jsonKeys`.
pub(crate) fn comma_separated(names: &[AttributeName]) -> Json {
    Json::from(names.iter().map(AsRef::as_ref).collect::<Vec<&str>>().join(","))
}

/// Writes names as an array of strings: attribute names, or the names the `attrs` of a temporal query takes.
pub(crate) fn array<T: Display>(names: &[T]) -> Json {
    Json::Array(names.iter().map(|name| Json::from(name.to_string())).collect())
}
