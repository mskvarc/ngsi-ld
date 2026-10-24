//! The NGSI-LD specification version a payload conforms to (NGSI-LD clause 4.3.6.8), as the `ngsi-ld` preference
//! names it (NGSI-LD clause 6.3.6).

use std::{fmt, num::ParseIntError, str::FromStr};

/// An NGSI-LD specification version, `major.minor` with an optional `.patch` (NGSI-LD clause 4.3.6.8).
///
/// Clause 4.3.6.8 names versions as `major.minor`; a `Preference-Applied` value may also carry a patch level, as in
/// `ngsi-ld=1.8.1`. A version displays exactly as it was written, so `1.8` and `1.8.0` are different values.
///
/// ```
/// use ngsi_ld::http::headers::SpecVersion;
///
/// let version: SpecVersion = "1.8.1".parse()?;
/// assert_eq!(version, SpecVersion::with_patch(1, 8, 1));
/// assert_eq!(SpecVersion::new(1, 9).to_string(), "1.9");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpecVersion {
    /// The major version.
    major: u16,
    /// The minor version.
    minor: u16,
    /// The patch level, when written.
    patch: Option<u16>,
}

impl SpecVersion {
    /// The version `major.minor`.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> SpecVersion {
        SpecVersion { major, minor, patch: None }
    }

    /// The version `major.minor.patch`.
    #[must_use]
    pub const fn with_patch(major: u16, minor: u16, patch: u16) -> SpecVersion {
        SpecVersion {
            major,
            minor,
            patch: Some(patch),
        }
    }

    /// The major version.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// The minor version.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }

    /// The patch level, when written.
    #[must_use]
    pub const fn patch(self) -> Option<u16> {
        self.patch
    }
}

/// Why a string is not an NGSI-LD version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SpecVersionError {
    /// The version has fewer than two or more than three dot-separated components.
    #[error("a version has two or three dot-separated components, found {found}")]
    ComponentCount {
        /// How many components the string has.
        found: usize,
    },
    /// A component is empty, holds a character other than an ASCII digit, or has a leading zero.
    #[error("a version component must be a decimal number without a leading zero")]
    InvalidComponent,
    /// A component does not fit the range of a version number.
    #[error("a version component is out of range")]
    OutOfRange(#[source] ParseIntError),
}

/// Parses one version component: ASCII digits without a leading zero, so that displaying the version reproduces
/// the input.
fn parse_component(raw: &str) -> Result<u16, SpecVersionError> {
    let canonical = !raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()) && (raw == "0" || !raw.starts_with('0'));
    if !canonical {
        return Err(SpecVersionError::InvalidComponent);
    }
    raw.parse().map_err(SpecVersionError::OutOfRange)
}

// Two or three `.`-separated components, the patch level optional, which a derive cannot express.
impl FromStr for SpecVersion {
    type Err = SpecVersionError;

    fn from_str(raw: &str) -> Result<SpecVersion, SpecVersionError> {
        let mut components = raw.split('.');
        let (Some(major), Some(minor), patch, None) = (components.next(), components.next(), components.next(), components.next()) else {
            return Err(SpecVersionError::ComponentCount { found: raw.split('.').count() });
        };
        Ok(SpecVersion {
            major: parse_component(major)?,
            minor: parse_component(minor)?,
            patch: patch.map(parse_component).transpose()?,
        })
    }
}

// The patch level is written only when present, which no derive can express.
impl fmt::Display for SpecVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)?;
        match self.patch {
            Some(patch) => write!(f, ".{patch}"),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{SpecVersion, SpecVersionError};
    use pretty_assertions::assert_eq;

    #[test]
    fn a_version_round_trips_as_written() {
        for raw in ["1.8", "1.8.1", "1.9", "0.0", "10.20.30"] {
            assert_eq!(raw.parse::<SpecVersion>().map(|version| version.to_string()), Ok(raw.to_owned()));
        }
    }

    #[test]
    fn a_version_exposes_its_components() {
        let version = "1.8.1".parse::<SpecVersion>().unwrap();
        assert_eq!((version.major(), version.minor(), version.patch()), (1, 8, Some(1)));
        assert_eq!(SpecVersion::new(1, 9).patch(), None);
    }

    #[test]
    fn a_version_with_the_wrong_number_of_components_is_rejected() {
        assert_eq!("1".parse::<SpecVersion>(), Err(SpecVersionError::ComponentCount { found: 1 }));
        assert_eq!("latest".parse::<SpecVersion>(), Err(SpecVersionError::ComponentCount { found: 1 }));
        assert_eq!("1.2.3.4".parse::<SpecVersion>(), Err(SpecVersionError::ComponentCount { found: 4 }));
    }

    #[test]
    fn a_version_with_a_malformed_component_is_rejected() {
        for raw in ["1.", ".1", "1.x", "+1.2", "01.2", "1.02", "1 .2", "1.2."] {
            assert_eq!(raw.parse::<SpecVersion>(), Err(SpecVersionError::InvalidComponent), "{raw}");
        }
        assert!(matches!("1.70000".parse::<SpecVersion>(), Err(SpecVersionError::OutOfRange(_))));
    }
}
