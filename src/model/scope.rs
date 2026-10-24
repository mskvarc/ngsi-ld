//! Scopes: a hierarchical, user-defined classification of entities (NGSI-LD clause 4.18).

use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

/// Why a string is not a valid scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ScopeError {
    /// The scope, or one of its levels, is empty.
    #[error("scope level at byte {position} is empty")]
    EmptyLevel {
        /// The byte offset where the empty level starts.
        position: usize,
    },
    /// A level does not start with a letter.
    #[error("a scope level must start with a letter, found `{found}` at byte {position}")]
    InvalidStart {
        /// The offending character.
        found: char,
        /// Its byte offset in the input.
        position: usize,
    },
    /// A level contains a character that is neither a letter, a number nor `_`.
    #[error("`{found}` at byte {position} is not a scope level character")]
    InvalidCharacter {
        /// The offending character.
        found: char,
        /// Its byte offset in the input.
        position: usize,
    },
}

impl ScopeError {
    /// The same error in a text that holds the checked one at byte `offset`.
    #[must_use]
    pub(crate) const fn shifted(self, offset: usize) -> Self {
        match self {
            Self::EmptyLevel { position } => Self::EmptyLevel { position: position + offset },
            Self::InvalidStart { found, position } => Self::InvalidStart {
                found,
                position: position + offset,
            },
            Self::InvalidCharacter { found, position } => Self::InvalidCharacter {
                found,
                position: position + offset,
            },
        }
    }
}

/// Validates one `ScopeLevel` (`unicodeLetter *ScopeLevelChar`) starting at byte `offset`.
fn validate_level(level: &str, offset: usize) -> Result<(), ScopeError> {
    let mut chars = level.char_indices().map(|(position, found)| (position + offset, found));
    match chars.next() {
        None => Err(ScopeError::EmptyLevel { position: offset }),
        Some((position, found)) if found.general_category_group() != GeneralCategoryGroup::Letter => Err(ScopeError::InvalidStart { found, position }),
        Some(_) => chars
            .find(|&(_, found)| !matches!(found.general_category_group(), GeneralCategoryGroup::Letter | GeneralCategoryGroup::Number) && found != '_')
            .map_or(Ok(()), |(position, found)| Err(ScopeError::InvalidCharacter { found, position })),
    }
}

/// Validates `Scope = [/] ScopeLevel *(/ ScopeLevel)`.
fn validate_scope(value: &str) -> Result<(), ScopeError> {
    let (body, offset) = value.strip_prefix('/').map_or((value, 0), |rest| (rest, 1));
    body.split('/')
        .scan(offset, |start, level| {
            let level_offset = *start;
            *start += level.len() + 1;
            Some(validate_level(level, level_offset))
        })
        .collect()
}

/// A scope: `[/] ScopeLevel *(/ ScopeLevel)` (NGSI-LD clause 4.18).
///
/// The NGSI-LD Null the grammar also admits only marks a deleted scope, which a client expresses through a
/// fragment instead (see [`Change`](crate::model::Change)).
///
/// ```
/// use ngsi_ld::model::Scope;
///
/// let scope: Scope = "/Madrid/Gardens/ParqueNorte".parse()?;
/// assert_eq!(scope.to_string(), "/Madrid/Gardens/ParqueNorte");
/// assert!("/Madrid//Gardens".parse::<Scope>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_scope, error = ScopeError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom, Serialize, Deserialize)
)]
pub struct Scope(String);

/// Validates a level on its own, so its errors count from its first byte.
fn validate_single_level(level: &str) -> Result<(), ScopeError> {
    validate_level(level, 0)
}

/// One level of a scope: `unicodeLetter *ScopeLevelChar` (the `ScopeLevel` production of NGSI-LD clause 4.18),
/// as the scope query language of clause 4.19 names it between its `/` separators.
///
/// ```
/// use ngsi_ld::model::ScopeLevel;
///
/// let level: ScopeLevel = "ParqueNorte".parse()?;
/// assert_eq!(level.to_string(), "ParqueNorte");
/// assert!("1Madrid".parse::<ScopeLevel>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_single_level, error = ScopeError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ScopeLevel(String);

#[cfg(test)]
mod tests;
