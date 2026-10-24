//! The registration management information of NGSI-LD clause 5.2.34: how a broker should pace and bound the
//! distributed operations it sends a context source.

use crate::{
    model::{BuildError, Duration, Number, PositiveNumber, value::positive_number::resolve_positive},
    query::Locality,
    registration::{ManagementMember, Problem, RegistrationField},
};
use bon::bon;

/// How a broker should send distributed operations to a registered context source (NGSI-LD clause 5.2.34); a
/// broker may override it.
///
/// ```
/// use ngsi_ld::{model::Duration, query::Locality, registration::RegistrationManagementInfo};
///
/// let management = RegistrationManagementInfo::builder()
///     .locality(Locality::Local)
///     .cache_duration("PT30S".parse::<Duration>()?)
///     .timeout(2000)
///     .build()?;
/// assert_eq!(management.locality(), Locality::Local);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegistrationManagementInfo {
    /// `localOnly`.
    pub(crate) locality: Locality,
    /// `cacheDuration`.
    pub(crate) cache_duration: Option<Duration>,
    /// `timeout`, in milliseconds.
    pub(crate) timeout: Option<PositiveNumber>,
    /// `cooldown`, in milliseconds.
    pub(crate) cooldown: Option<PositiveNumber>,
}

#[bon]
impl RegistrationManagementInfo {
    /// Starts a builder; every member is optional.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing a `timeout` or `cooldown` that is not greater than 0 (Table
    /// 5.2.34-1).
    #[builder]
    pub fn new(
        #[builder(default)] locality: Locality,
        cache_duration: Option<Duration>,
        #[builder(into)] timeout: Option<Number>,
        #[builder(into)] cooldown: Option<Number>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let management = Self {
            locality,
            cache_duration,
            timeout: resolve_positive(RegistrationField::from(ManagementMember::Timeout), timeout, &mut problems),
            cooldown: resolve_positive(RegistrationField::from(ManagementMember::Cooldown), cooldown, &mut problems),
        };
        BuildError::finish(problems, management)
    }
}

impl RegistrationManagementInfo {
    /// Returns whether distributed operations act only on the data the source itself holds (`localOnly`, clause
    /// 4.3.6.4).
    #[must_use]
    pub const fn locality(&self) -> Locality {
        self.locality
    }

    /// Returns the least time between two consumption operations on the same data, before which a cached value is
    /// returned (`cacheDuration`).
    #[must_use]
    pub const fn cache_duration(&self) -> Option<&Duration> {
        self.cache_duration.as_ref()
    }

    /// Returns the time after which a forwarded request counts as failed, in milliseconds (`timeout`).
    #[must_use]
    pub const fn timeout(&self) -> Option<&PositiveNumber> {
        self.timeout.as_ref()
    }

    /// Returns the time to wait after a failure before forwarding again, in milliseconds (`cooldown`).
    #[must_use]
    pub const fn cooldown(&self) -> Option<&PositiveNumber> {
        self.cooldown.as_ref()
    }
}
