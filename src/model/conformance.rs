//! The version of the specification a subscriber asks its notifications to conform to (the `ngsildConformance`
//! member of NGSI-LD Table 5.2.12-1, applied by clause 4.3.6.8).

use std::{num::ParseIntError, str::FromStr};

/// Why a string is not a `major.minor` version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConformanceVersionError {
    /// The string is not two numbers separated by one `.`.
    #[error("a conformance version is written major.minor")]
    Shape,
    /// A part is not a decimal number.
    #[error("invalid version number")]
    Number {
        /// Why the number is invalid.
        #[source]
        source: ParseIntError,
    },
}

/// A semantic version of the specification in the form `major.minor` (Table 5.2.12-1).
///
/// ```
/// use ngsi_ld::model::ConformanceVersion;
///
/// let version: ConformanceVersion = "1.8".parse()?;
/// assert_eq!((version.major(), version.minor()), (1, 8));
/// assert!("1.8.1".parse::<ConformanceVersion>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
#[display("{major}.{minor}")]
pub struct ConformanceVersion {
    /// The major version.
    major: u32,
    /// The minor version.
    minor: u32,
}

impl ConformanceVersion {
    /// The version `major.minor`.
    #[must_use]
    pub const fn new(major: u32, minor: u32) -> ConformanceVersion {
        ConformanceVersion { major, minor }
    }

    /// Returns the major version.
    #[must_use]
    pub const fn major(&self) -> u32 {
        self.major
    }

    /// Returns the minor version.
    #[must_use]
    pub const fn minor(&self) -> u32 {
        self.minor
    }
}

// Two numbers around one `.`, digits only, which no derive parses.
impl FromStr for ConformanceVersion {
    type Err = ConformanceVersionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (major, minor) = text.split_once('.').ok_or(ConformanceVersionError::Shape)?;
        let number = |part: &str| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(ConformanceVersionError::Shape);
            }
            part.parse().map_err(|source| ConformanceVersionError::Number { source })
        };
        Ok(Self::new(number(major)?, number(minor)?))
    }
}
