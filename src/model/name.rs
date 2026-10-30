//! Names of entity types, attributes and vocabulary terms (NGSI-LD clause 4.6.2).
//!
//! Clause 4.6.2 restricts entity type, Property and Relationship names to `name = unicodeLetter *nameChar`, with
//! `nameChar = unicodeNumber / unicodeLetter / "_"`, optionally prefixed as `prefix:name`. Clauses 4.17 and 5.2.4
//! also allow a fully qualified name (an absolute IRI) wherever a short name is allowed.

use crate::model::{ReservedMember, null::NGSI_LD_NULL};
use iri_rs::IriBuf;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

#[cfg(feature = "linked-data")]
#[macro_use]
pub(crate) mod mapping;
pub(crate) mod draft;
pub(super) mod reserved;

/// Why a string is not a valid name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum NameError {
    /// The string is empty.
    #[error("a name cannot be empty")]
    Empty,
    /// The first character (or the first character after a prefix) is not a Unicode letter.
    #[error("a name must start with a letter, found `{found}` at byte {position}")]
    InvalidStart {
        /// The offending character.
        found: char,
        /// Its byte offset in the input.
        position: usize,
    },
    /// A character is neither a Unicode letter, a Unicode number nor `_`.
    #[error("`{found}` at byte {position} is not a name character")]
    InvalidCharacter {
        /// The offending character.
        found: char,
        /// Its byte offset in the input.
        position: usize,
    },
    /// The string names a member the core `@context` reserves, which cannot be an attribute.
    #[error("`{member}` is reserved by the core @context and cannot name an attribute")]
    Reserved {
        /// The reserved member.
        member: ReservedMember,
    },
    /// A vocabulary term contains whitespace or starts with `@`.
    #[error("`{found}` at byte {position} is not allowed in a vocabulary term")]
    InvalidTermCharacter {
        /// The offending character.
        found: char,
        /// Its byte offset in the input.
        position: usize,
    },
}

/// Whether `c` belongs to the Unicode general category Letter (`\p{L}`).
fn is_letter(c: char) -> bool {
    c.general_category_group() == GeneralCategoryGroup::Letter
}

/// Whether `c` is a `nameChar` of clause 4.6.2: a letter, a number (`\p{N}`) or `_`.
fn is_name_char(c: char) -> bool {
    is_letter(c) || c == '_' || c.general_category_group() == GeneralCategoryGroup::Number
}

/// Validates `unicodeLetter *nameChar` starting at byte `offset` of the original input.
fn validate_unprefixed(value: &str, offset: usize) -> Result<(), NameError> {
    let mut chars = value.char_indices();
    match chars.next() {
        None => Err(NameError::Empty),
        Some((position, found)) if !is_letter(found) => Err(NameError::InvalidStart {
            found,
            position: offset + position,
        }),
        Some(_) => chars
            .map(|(position, found)| (position + offset, found))
            .find(|&(_, found)| !is_name_char(found))
            .map_or(Ok(()), |(position, found)| Err(NameError::InvalidCharacter { found, position })),
    }
}

/// Validates the `name` production of clause 4.6.2, including the optional `prefix:` form.
fn validate_name(value: &str) -> Result<(), NameError> {
    match value.split_once(':') {
        Some((prefix, local)) => {
            validate_unprefixed(prefix, 0)?;
            validate_unprefixed(local, prefix.len() + 1)
        }
        None => validate_unprefixed(value, 0),
    }
}

/// Validates a name or, failing that, an absolute IRI; the name error is reported when neither applies.
fn validate_name_or_iri(value: &str) -> Result<(), NameError> {
    validate_name(value).or_else(|error| if IriBuf::new(value).is_ok() { Ok(()) } else { Err(error) })
}

/// Validates an attribute name: a name or an absolute IRI that the core `@context` does not reserve.
fn validate_attribute_name(value: &str) -> Result<(), NameError> {
    match ReservedMember::find(value) {
        Some(member) => Err(NameError::Reserved { member }),
        None => validate_name_or_iri(value),
    }
}

/// Validates a vocabulary term: non-empty, no whitespace, not a JSON-LD keyword.
fn validate_vocab_term(value: &str) -> Result<(), NameError> {
    if value.is_empty() {
        return Err(NameError::Empty);
    }
    value
        .char_indices()
        .find(|&(position, found)| found.is_whitespace() || (position == 0 && found == '@'))
        .map_or(Ok(()), |(position, found)| Err(NameError::InvalidTermCharacter { found, position }))
}

/// A short name as defined by NGSI-LD clause 4.6.2: `unicodeLetter *nameChar`, optionally prefixed as
/// `prefix:name`.
///
/// ```
/// use ngsi_ld::model::Name;
///
/// let name: Name = "ex:brandName".parse()?;
/// assert_eq!(name.to_string(), "ex:brandName");
/// assert!("brand-name".parse::<Name>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_name, error = NameError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom, Serialize, Deserialize)
)]
pub struct Name(String);

/// An unprefixed term: the `AttrName` and `EntityType` productions of the query language of NGSI-LD clause
/// 4.9 (`unicodeLetter *TermChar`).
///
/// The query, projection and type selection languages use the `:` and `,` characters as separators, so their
/// names cannot carry a prefix.
///
/// ```
/// use ngsi_ld::model::Term;
///
/// let term: Term = "temperature_2".parse()?;
/// assert_eq!(term.to_string(), "temperature_2");
/// assert!("ex:temperature".parse::<Term>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_term, error = NameError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom, Serialize, Deserialize)
)]
pub struct Term(String);

/// Validates the unprefixed `AttrName` production of clause 4.9.
fn validate_term(value: &str) -> Result<(), NameError> {
    validate_unprefixed(value, 0)
}

/// The name of an attribute (a Property or a Relationship of any kind).
///
/// It is a [`Name`] or an absolute IRI (NGSI-LD clauses 4.6.2 and 5.2.4), and never a member the core
/// `@context` defines as non-reified or as part of an entity or attribute structure (clause 4.5.1):
/// `id`, `type`, `scope`, `datasetId`, `observedAt` and the others listed by [`ReservedMember`].
///
/// ```
/// use ngsi_ld::model::AttributeName;
///
/// let name: AttributeName = "brandName".parse()?;
/// assert_eq!(name.to_string(), "brandName");
/// assert!(
///     "https://example.org/brandName"
///         .parse::<AttributeName>()
///         .is_ok()
/// );
/// assert!("datasetId".parse::<AttributeName>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_attribute_name, error = NameError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom, Serialize, Deserialize)
)]
pub struct AttributeName(String);

/// The name of an entity type: a [`Name`] or an absolute IRI (NGSI-LD clauses 4.6.2, 4.17 and 5.2.4).
///
/// ```
/// use ngsi_ld::model::EntityTypeName;
///
/// let short: EntityTypeName = "Vehicle".parse()?;
/// let long: EntityTypeName = "https://example.org/Vehicle".parse()?;
/// assert_ne!(short, long);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_name_or_iri, error = NameError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom, Serialize, Deserialize)
)]
pub struct EntityTypeName(String);

/// Generates `as_str` for the `nutype` names, whose derives offer only `AsRef<str>`, a call that needs a type
/// annotation wherever the target is not already a `&str`.
macro_rules! name_as_str {
    ($($name:ident),+) => {$(
        impl $name {
            /// Returns the name as written.
            #[must_use]
            pub fn as_str(&self) -> &str {
                AsRef::<str>::as_ref(self)
            }
        }
    )+};
}

name_as_str!(Name, Term, AttributeName, EntityTypeName);

/// A string a JSON-LD processor coerces into an IRI under the `@context`.
///
/// It is the `vocab` of a `VocabProperty` (NGSI-LD clause 4.5.20) and the `valueType` of a Property (clause 4.5.2.2).
///
/// It is a term, a compact IRI or an absolute IRI. Terms are free-form in JSON-LD (the specification's own
/// examples use `non-commercial`), so the only restrictions are the ones that make a string unusable as a
/// term: it is not empty, holds no whitespace and is not a JSON-LD keyword.
///
/// It is also the `objectType` of a Relationship (clause 5.2.6). Every [`EntityTypeName`] is a term, so a type name
/// converts into one without a check.
///
/// ```
/// use ngsi_ld::model::{EntityTypeName, VocabTerm};
///
/// let term: VocabTerm = "non-commercial".parse()?;
/// assert_eq!(term.to_string(), "non-commercial");
/// assert!("@id".parse::<VocabTerm>().is_err());
/// let parking: EntityTypeName = "OffStreetParking".parse()?;
/// assert_eq!(VocabTerm::from(&parking).to_string(), "OffStreetParking");
/// # Ok::<(), anyhow::Error>(())
/// ```
//
// Not a `nutype` like the other names: the NGSI-LD Null is a valid term that the crate writes as the `vocab` of a
// deleted `VocabProperty` (clause 4.5.0), and a `nutype` builds a statically valid value only through its fallible
// constructor or an `unsafe` one. The checks are the ones `validate_vocab_term` makes, in `try_new` and every
// conversion from text, so no unchecked value is ever built outside `ngsi_ld_null`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display, derive_more::AsRef, Serialize, Deserialize)]
#[as_ref(str)]
#[serde(try_from = "String")]
pub struct VocabTerm(String);

impl VocabTerm {
    /// Checks a term.
    ///
    /// # Errors
    ///
    /// Returns [`NameError`] when the term is empty, holds whitespace or starts with `@`.
    pub fn try_new(term: impl Into<String>) -> Result<Self, NameError> {
        let term = term.into();
        validate_vocab_term(&term).map(|()| Self(term))
    }

    /// The NGSI-LD Null as a term (clause 4.5.0): `urn:ngsi-ld:null`, which is neither empty nor holds whitespace or a
    /// leading `@`.
    pub(crate) fn ngsi_ld_null() -> Self {
        Self(NGSI_LD_NULL.as_str().to_owned())
    }

    /// Returns the term.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

// No derive validates the text; `derive_more::FromStr` would wrap it unchecked.
impl FromStr for VocabTerm {
    type Err = NameError;

    fn from_str(term: &str) -> Result<Self, NameError> {
        Self::try_new(term)
    }
}

// See the `FromStr` implementation for why this is not derived.
impl TryFrom<String> for VocabTerm {
    type Error = NameError;

    fn try_from(term: String) -> Result<Self, NameError> {
        Self::try_new(term)
    }
}

// See the `FromStr` implementation for why this is not derived.
impl TryFrom<&str> for VocabTerm {
    type Error = NameError;

    fn try_from(term: &str) -> Result<Self, NameError> {
        Self::try_new(term)
    }
}

/// The term naming an entity type, as the `objectType` of a Relationship names the types of its target (clause
/// 5.2.6).
///
/// No check is needed: a name of clause 4.6.2 starts with a letter and holds no whitespace, and an absolute IRI holds
/// no whitespace and starts with its scheme, so every entity type name is a term.
// No derive converts one checked name into another.
impl From<EntityTypeName> for VocabTerm {
    fn from(name: EntityTypeName) -> Self {
        Self(name.into_inner())
    }
}

/// The term naming an entity type the caller keeps, such as one it also creates the target entity with.
// No derive converts one checked name into another; the term owns its text, so the name is copied.
impl From<&EntityTypeName> for VocabTerm {
    fn from(name: &EntityTypeName) -> Self {
        Self(name.as_str().to_owned())
    }
}

#[cfg(test)]
mod tests;
