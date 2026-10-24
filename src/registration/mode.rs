//! The mode of distributed operation a context source supports (NGSI-LD clauses 4.3.6 and 5.2.9).

/// How a broker distributes operations to a registered context source (the `mode` of Table 5.2.9-1, clause
/// 4.3.6); `inclusive` when absent.
///
/// The enum is exhaustive: Table 5.2.9-1 allows exactly these four modes.
///
/// ```
/// use ngsi_ld::registration::RegistrationMode;
///
/// assert_eq!(RegistrationMode::Auxiliary.to_string(), "auxiliary");
/// assert_eq!(
///     "redirect".parse::<RegistrationMode>()?,
///     RegistrationMode::Redirect
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum RegistrationMode {
    /// `inclusive`: the broker treats the source as an equal of its own data and of every other source (clause
    /// 4.3.6.2).
    #[default]
    Inclusive,
    /// `exclusive`: the registered attributes of one entity are held by the source alone (clause 4.3.6.3).
    Exclusive,
    /// `redirect`: the registered data is held outside the broker, possibly by several sources (clause 4.3.6.3).
    Redirect,
    /// `auxiliary`: the source only supplements the broker's own data, for consumption operations (clause
    /// 4.3.6.2).
    Auxiliary,
}
