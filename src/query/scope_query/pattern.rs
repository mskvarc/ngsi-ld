//! The scopes a scope query names: scope levels, each a name or the single-level wildcard `+` (the `ScopeQ`
//! production of NGSI-LD clause 4.19).

use crate::model::{ScopeError, ScopeLevel};
use std::{fmt, str::FromStr};

/// One level of a [`ScopePattern`] (the `ScopeQLevel` production of NGSI-LD clause 4.19).
///
/// The enum is exhaustive: clause 4.19 defines a level as a name or `+`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum LevelPattern {
    /// A scope level, matched as written.
    #[display("{_0}")]
    Level(ScopeLevel),
    /// `+`: any single scope level.
    #[display("+")]
    AnyLevel,
}

/// A scope whose levels may be the wildcard `+` (the `ScopeQ` production of NGSI-LD clause 4.19):
/// `/Madrid/+/ParqueNorte`.
///
/// The production starts with `/`, but clause 4.19 example 5 writes the scope `/CompanyB` as `CompanyB`; a pattern
/// is read with or without its leading `/`, and always printed with it.
///
/// ```
/// use ngsi_ld::query::scope_query::{LevelPattern, ScopePattern};
///
/// let pattern: ScopePattern = "/Madrid/+/ParqueNorte".parse()?;
/// assert_eq!(pattern.levels()[1], LevelPattern::AnyLevel);
/// assert_eq!("CompanyB".parse::<ScopePattern>()?.to_string(), "/CompanyB");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopePattern(Vec<LevelPattern>);

impl ScopePattern {
    /// Returns the levels, from the root; there is at least one.
    #[must_use]
    pub fn levels(&self) -> &[LevelPattern] {
        &self.0
    }
}

// Every level is introduced by `/`, which a derive cannot express.
impl fmt::Display for ScopePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|level| write!(f, "/{level}"))
    }
}

/// Reads the levels between `/` separators; the error positions count from the start of `text`.
impl FromStr for ScopePattern {
    type Err = ScopeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (body, offset) = text.strip_prefix('/').map_or((text, 0), |rest| (rest, 1));
        body.split('/')
            .scan(offset, |start, level| {
                let level_offset = *start;
                *start += level.len() + 1;
                Some((level, level_offset))
            })
            .map(|(level, level_offset)| match level {
                "+" => Ok(LevelPattern::AnyLevel),
                name => ScopeLevel::try_new(name).map(LevelPattern::Level).map_err(|error| error.shifted(level_offset)),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}
