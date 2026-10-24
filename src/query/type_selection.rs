//! The NGSI-LD Entity Type Selection Language (NGSI-LD clause 4.17), written in the `type` parameter.
//!
//! A selection is a disjunction of alternatives, each one entity type or a parenthesised conjunction of entity
//! types: `(Home;Vehicle)|Motorhome`. Clause 4.17 accepts `,` as an alternative spelling of the OR operator;
//! the parser reads it and the printer always writes `|`.
//!
//! ```
//! use ngsi_ld::query::type_selection;
//!
//! let selection = type_selection::all_of(["Home", "Vehicle"])
//!     .or("Motorhome")
//!     .build()?;
//! assert_eq!(selection.to_string(), "(Home;Vehicle)|Motorhome");
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Type selections
//!
//! [`TypeSelection`], [`SelectableType`], [`SelectableTypeError`].
//!
//! # Start functions
//!
//! [`entity_type`], [`all_of`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, each_in_text},
};
use crate::{
    model::{EntityTypeName, NameError},
    query::{
        Expected,
        ParseError,
        Problem,
        build_error::Deferred,
        grammar::{self, write_separated},
        type_selection::builders::TypeSelectionBuilder,
    },
};
use std::{fmt, str::FromStr};

pub mod builders;
mod parser;

/// Why a string or a name is not an entity type a type selection can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum SelectableTypeError {
    /// The string is neither a name nor a URI.
    #[error("not a valid entity type name")]
    Name {
        /// Why the name is invalid.
        #[from]
        source: NameError,
    },
    /// The name holds a character the language uses as an operator or a parenthesis.
    #[error("`{found}` at byte {position} is an operator of the type selection language")]
    Separator {
        /// The offending character.
        found: char,
        /// Its byte offset in the name.
        position: usize,
    },
}

/// Whether `c` is an operator or a parenthesis of the type selection language.
pub(crate) const fn is_separator(c: char) -> bool {
    matches!(c, ',' | ';' | '|' | '(' | ')')
}

/// An entity type a type selection can hold (the `EntityType` production of NGSI-LD clause 4.17): a name of
/// clause 4.6.2 or a URI.
///
/// The note of clause 4.17 warns that `,`, `;`, `(` and `)` are allowed in URIs but are also the operators of
/// the language, and recommends short names. A URI holding one of them (or `|`) cannot be written in a
/// selection without changing its meaning, so it is refused.
///
/// ```
/// use ngsi_ld::query::type_selection::SelectableType;
///
/// let short: SelectableType = "Building".parse()?;
/// assert_eq!(short.to_string(), "Building");
/// assert!(
///     "https://example.org/Building"
///         .parse::<SelectableType>()
///         .is_ok()
/// );
/// assert!(
///     "https://example.org/types;v=2"
///         .parse::<SelectableType>()
///         .is_err()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display, derive_more::AsRef, derive_more::Into)]
#[display("{_0}")]
pub struct SelectableType(EntityTypeName);

impl TryFrom<EntityTypeName> for SelectableType {
    type Error = SelectableTypeError;

    fn try_from(name: EntityTypeName) -> Result<Self, Self::Error> {
        match name.as_ref().char_indices().find(|&(_, c)| is_separator(c)) {
            Some((position, found)) => Err(SelectableTypeError::Separator { found, position }),
            None => Ok(Self(name)),
        }
    }
}

// A derived parser would skip the checks of `TryFrom<EntityTypeName>`.
impl FromStr for SelectableType {
    type Err = SelectableTypeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(text.parse::<EntityTypeName>()?)
    }
}

/// A selection of entities by type (the `EntityTypes` production of NGSI-LD clause 4.17): the entities having
/// all the types of at least one alternative.
///
/// ```
/// use ngsi_ld::query::type_selection::TypeSelection;
///
/// let selection: TypeSelection = "Building,House".parse()?;
/// assert_eq!(selection.to_string(), "Building|House");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeSelection {
    /// The alternatives; never empty, and none of them empty.
    alternatives: Vec<Vec<SelectableType>>,
}

impl TypeSelection {
    /// Selects the entities of the one entity type `selected`.
    pub(crate) fn single(selected: SelectableType) -> TypeSelection {
        TypeSelection {
            alternatives: vec![vec![selected]],
        }
    }

    /// Iterates over the alternatives, each the entity types an entity must all have.
    pub fn alternatives(&self) -> impl Iterator<Item = &[SelectableType]> {
        self.alternatives.iter().map(Vec::as_slice)
    }

    /// Selects the entities this selection or `other` selects.
    #[must_use]
    pub fn or(mut self, other: Self) -> Self {
        self.alternatives.extend(other.alternatives);
        self
    }
}

// Alternatives are joined with `|` and conjunctions parenthesised, which a derive cannot express.
impl fmt::Display for TypeSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.alternatives.iter().enumerate().try_for_each(|(index, alternative)| {
            if index > 0 {
                f.write_str("|")?;
            }
            match alternative.as_slice() {
                [single] => single.fmt(f),
                conjunction => {
                    f.write_str("(")?;
                    write_separated(f, conjunction, ';')?;
                    f.write_str(")")
                }
            }
        })
    }
}

// No derive parses the entity type selection grammar.
impl FromStr for TypeSelection {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        grammar::run(text, parser::selection, Expected::EntityType, Expected::Separator).map(|alternatives| Self { alternatives })
    }
}

/// Validates one entity type.
fn parse_type(name: &str) -> Deferred<SelectableType> {
    Deferred::from_result(name.parse().map_err(|source| Problem::InvalidEntityType { source }))
}

/// Validates a conjunction of entity types.
fn parse_conjunction(names: impl IntoIterator<Item = impl AsRef<str>>) -> Deferred<Vec<SelectableType>> {
    names
        .into_iter()
        .map(|name| parse_type(name.as_ref()))
        .collect::<Deferred<Vec<_>>>()
        .and_then(|types| if types.is_empty() { Err(Problem::EmptyConjunction) } else { Ok(types) })
}

/// Starts a selection with the entity type `name`.
///
/// ```
/// use ngsi_ld::query::type_selection;
///
/// let selection = type_selection::entity_type("Building")
///     .or("House")
///     .build()?;
/// assert_eq!(selection.to_string(), "Building|House");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[must_use]
pub fn entity_type(name: impl AsRef<str>) -> TypeSelectionBuilder {
    TypeSelectionBuilder(parse_type(name.as_ref()).map(|selected| vec![vec![selected]]))
}

/// Starts a selection with the entities having all the types `names`.
#[must_use]
pub fn all_of(names: impl IntoIterator<Item = impl AsRef<str>>) -> TypeSelectionBuilder {
    TypeSelectionBuilder(parse_conjunction(names).map(|conjunction| vec![conjunction]))
}

#[cfg(feature = "linked-data")]
impl MapNames for TypeSelection {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        let alternatives = self
            .alternatives
            .into_iter()
            .map(|alternative| each_in_text(alternative, mapping))
            .collect::<Result<_, _>>()?;
        Ok(Self { alternatives })
    }
}

/// The mapped entity type must still be one a selection can hold.
#[cfg(feature = "linked-data")]
impl MapNames for SelectableType {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        let mapped = self.0.map_names(mapping)?;
        Self::try_from(mapped).map_err(|source| DocumentError::top(M::Error::from(source)))
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// A type selection carries no `@context`: it is written under the context of the request that carries it.
    impl for TypeSelection
);

#[cfg(test)]
mod tests;
