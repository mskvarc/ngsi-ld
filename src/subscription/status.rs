//! Whether a subscription operates: the `isActive` a subscriber sets and the `status` a broker reports (NGSI-LD
//! clauses 5.2.12 and 5.8.1.4).

/// Whether a subscriber runs or pauses a subscription.
///
/// It is the `isActive` member of NGSI-LD Table 5.2.12-1.
///
/// ```
/// use ngsi_ld::subscription::Activation;
///
/// assert_eq!(Activation::default(), Activation::Unspecified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Activation {
    /// No `isActive` member: the subscription is active (Table 5.2.12-1).
    #[default]
    Unspecified,
    /// `true`: the subscription is under operation.
    Active,
    /// `false`: the subscription is paused, and no notification is delivered.
    Paused,
}

flag_choice!(Activation, Active, Paused);

/// The state a broker reports for a subscription.
///
/// It is the `status` member of NGSI-LD Table 5.2.12-2.
///
/// The enum is exhaustive: the table allows exactly these three values.
///
/// ```
/// use ngsi_ld::subscription::SubscriptionStatus;
///
/// assert_eq!(
///     "expired".parse::<SubscriptionStatus>()?,
///     SubscriptionStatus::Expired
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum SubscriptionStatus {
    /// `active`: notifications are sent.
    Active,
    /// `paused`: the subscriber set `isActive` to false.
    Paused,
    /// `expired`: the subscription's `expiresAt` has passed (clause 5.8.1.4).
    Expired,
}
