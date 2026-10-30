//! Identifiers of entities and attribute instances (NGSI-LD clauses 4.5.5, 4.5.7 and 5.2.4).

use crate::model::{Uri, UriError, keyword::NONE};
use iri_rs::Iri;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// The identifier of an entity: a URI.
///
/// NGSI-LD clause 5.2.4.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::EntityId;
///
/// let from_literal = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let parsed: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse()?;
/// assert_eq!(from_literal, parsed);
/// assert!("Vehicle A4567".parse::<EntityId>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
    derive_more::FromStr,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct EntityId(Uri);

impl EntityId {
    /// Parses `text` like [`FromStr`], and writes it back as given (see [`Uri::verbatim`]).
    ///
    /// ```
    /// use ngsi_ld::model::EntityId;
    ///
    /// let id = EntityId::verbatim("URN:NGSI-LD:Vehicle:B9211")?;
    /// assert_eq!(id.to_string(), "URN:NGSI-LD:Vehicle:B9211");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`UriError`] when `text` is not a URI or IRI.
    pub fn verbatim(text: &str) -> Result<Self, UriError> {
        Uri::verbatim(text).map(Self)
    }

    /// Returns the identifier as a URI.
    #[must_use]
    pub const fn as_uri(&self) -> &Uri {
        &self.0
    }

    /// Returns the identifier as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// A literal checked at compile time by `iri_rs::iri!`. No derive converts through the wrapped [`Uri`].
impl From<Iri<&str>> for EntityId {
    fn from(iri: Iri<&str>) -> Self {
        Self(Uri::from(iri))
    }
}

// No derive parses a string into the wrapped URI.
impl TryFrom<&str> for EntityId {
    type Error = UriError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

// No derive parses a string into the wrapped URI.
impl TryFrom<String> for EntityId {
    type Error = UriError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// The identifier of one instance of a multi-attribute.
///
/// NGSI-LD clause 4.5.5.
///
/// The default instance has no dataset id, or, written explicitly, the JSON-LD keyword `@none`. Both mean the
/// same instance; the explicit spelling is kept as [`DatasetId::DefaultInstance`] so a document that used it is
/// written back the way it was read.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::DatasetId;
///
/// let gps = DatasetId::from(iri!("urn:ngsi-ld:dataset:gps"));
/// assert_eq!(gps.to_string(), "urn:ngsi-ld:dataset:gps");
/// assert_eq!("@none".parse::<DatasetId>()?, DatasetId::DefaultInstance);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::IsVariant, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum DatasetId {
    /// The default instance, spelled `@none`.
    #[display("@none")]
    DefaultInstance,
    /// A named instance.
    #[display("{_0}")]
    Named(Uri),
}

// `derive_more::From` would also generate a conversion for the unit variant; only the named one converts.
impl From<Uri> for DatasetId {
    fn from(uri: Uri) -> Self {
        Self::Named(uri)
    }
}

impl DatasetId {
    /// Parses `text` like [`FromStr`], and writes a named instance back as given (see [`Uri::verbatim`]).
    ///
    /// # Errors
    ///
    /// [`UriError`] when `text` is neither `@none` nor a URI or IRI.
    pub fn verbatim(text: &str) -> Result<Self, UriError> {
        if text == NONE {
            Ok(Self::DefaultInstance)
        } else {
            Uri::verbatim(text).map(Self::Named)
        }
    }
}

/// A literal checked at compile time by `iri_rs::iri!`. No derive converts through the wrapped [`Uri`].
impl From<Iri<&str>> for DatasetId {
    fn from(iri: Iri<&str>) -> Self {
        Self::Named(Uri::from(iri))
    }
}

// No derive parses the `@none` keyword into the unit variant.
impl FromStr for DatasetId {
    type Err = UriError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == NONE {
            Ok(Self::DefaultInstance)
        } else {
            value.parse().map(Self::Named)
        }
    }
}

// No derive parses a string through `FromStr`.
impl TryFrom<String> for DatasetId {
    type Error = UriError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

// The wire form is the display form; serde needs an owned `String` to serialise through `into`, and no derive
// converts through `Display`.
impl From<DatasetId> for String {
    fn from(id: DatasetId) -> Self {
        id.to_string()
    }
}

/// The identifier of one instance in a temporal evolution.
///
/// NGSI-LD clause 4.5.7.
///
/// A broker assigns it, and returns it in the temporal representation; Tables 5.2.5-2 and 5.2.6-2 do not make it
/// system generated, so a client may also give it in the temporal representation it sends (clauses 5.6.11.3 and
/// 5.6.12.3), never in a Core API entity.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
    derive_more::FromStr,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct InstanceId(Uri);

impl InstanceId {
    /// Parses `text` like [`FromStr`], and writes it back as given (see [`Uri::verbatim`]).
    ///
    /// # Errors
    ///
    /// [`UriError`] when `text` is not a URI or IRI.
    pub fn verbatim(text: &str) -> Result<Self, UriError> {
        Uri::verbatim(text).map(Self)
    }
}

/// A literal checked at compile time by `iri_rs::iri!`. No derive converts through the wrapped [`Uri`].
impl From<Iri<&str>> for InstanceId {
    fn from(iri: Iri<&str>) -> Self {
        Self(Uri::from(iri))
    }
}

#[cfg(test)]
mod tests;
