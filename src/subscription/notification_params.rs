//! How a subscription notifies: the `NotificationParams` of NGSI-LD clause 5.2.14.

use crate::{
    model::{
        AttributeName,
        AttributeNameDraft,
        BuildError,
        Input,
        MemberProblem,
        NotPositive,
        Number,
        Output,
        Representation,
        name::draft::resolve_required_names,
    },
    query::{Join, projection::Projection},
    subscription::{Endpoint, NotificationMember, NotificationOutputMembers, Problem, SubscriptionDirection, SubscriptionField},
};
use bon::bon;
use std::num::NonZeroU32;

/// Whether a notification shows the previous value of what changed.
///
/// It is the `showChanges` member of NGSI-LD Table 5.2.14.1-1.
///
/// ```
/// use ngsi_ld::subscription::ChangeReporting;
///
/// assert_eq!(ChangeReporting::default(), ChangeReporting::Unspecified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ChangeReporting {
    /// No `showChanges` member: only the current values, the default.
    #[default]
    Unspecified,
    /// `true`: `previousValue`, `previousLanguageMap`, `previousObject` and their counterparts accompany the
    /// current values.
    WithPrevious,
    /// `false`: only the current values.
    CurrentOnly,
}

flag_choice!(ChangeReporting, WithPrevious, CurrentOnly);

/// Whether a notification includes the system attributes.
///
/// The system attributes are `createdAt`, `modifiedAt`, `expiresAt`, and `deletedAt` for a deletion (the `sysAttrs`
/// member of NGSI-LD Table 5.2.14.1-1).
///
/// ```
/// use ngsi_ld::subscription::SystemAttributeReporting;
///
/// assert_eq!(
///     SystemAttributeReporting::default(),
///     SystemAttributeReporting::Unspecified
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SystemAttributeReporting {
    /// No `sysAttrs` member: no system attributes, the default.
    #[default]
    Unspecified,
    /// `true`: the system attributes are included.
    Include,
    /// `false`: no system attributes.
    Omit,
}

flag_choice!(SystemAttributeReporting, Include, Omit);

/// Whether the last notification succeeded.
///
/// It is the `status` member of NGSI-LD Table 5.2.14.2-1.
///
/// The enum is exhaustive: the table allows exactly these two values.
///
/// ```
/// use ngsi_ld::subscription::NotificationStatus;
///
/// assert_eq!(
///     "failed".parse::<NotificationStatus>()?,
///     NotificationStatus::Failed
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum NotificationStatus {
    /// `ok`: the last attempt to notify succeeded.
    Ok,
    /// `failed`: the last attempt to notify failed.
    Failed,
}

/// How a subscription notifies.
///
/// It holds the endpoint, the representation and the members of the entities a notification carries, and, in a
/// response, the broker's record of past notifications (NGSI-LD clause 5.2.14).
///
/// Build one with [`NotificationParams::builder`]; a response's comes with the [`Subscription`] it belongs to.
///
/// [`Subscription`]: crate::subscription::Subscription
///
/// ```
/// use ngsi_ld::{
///     model::Representation,
///     subscription::{Endpoint, NotificationParams},
/// };
///
/// let notification =
///     NotificationParams::builder(Endpoint::builder("http://my.endpoint.org/notify").build()?)
///         .format(Representation::Concise)
///         .attributes(["speed"])
///         .build()?;
/// assert_eq!(notification.format(), Some(Representation::Concise));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationParams<D: SubscriptionDirection> {
    /// `attributes`: deprecated, a synonym of `pick` that cannot name `id`, `type` or `scope`.
    pub(crate) attributes: Vec<AttributeName>,
    /// `pick`.
    pub(crate) pick: Option<Projection>,
    /// `omit`.
    pub(crate) omit: Option<Projection>,
    /// `format`; `keyValues` is read as [`Representation::Simplified`].
    pub(crate) format: Option<Representation>,
    /// `showChanges`.
    pub(crate) show_changes: ChangeReporting,
    /// `sysAttrs`.
    pub(crate) system_attributes: SystemAttributeReporting,
    /// `join`.
    pub(crate) join: Option<Join>,
    /// `joinLevel`.
    pub(crate) join_level: Option<NonZeroU32>,
    /// `endpoint`.
    pub(crate) endpoint: Endpoint,
    /// The members only a broker writes; `()` in a request.
    pub(crate) output: D::NotificationOutput,
}

#[bon]
impl NotificationParams<Input> {
    /// Starts a builder from the endpoint the notifications go to.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing an invalid or empty `attributes`, a `joinLevel` of 0 or without a
    /// `flat` or `inline` join, and `showChanges` with the simplified format (Table 5.2.14.1-1).
    #[builder]
    pub fn new(
        #[builder(start_fn)] endpoint: Endpoint,
        #[builder(with = |names: impl IntoIterator<Item = impl Into<AttributeNameDraft>>| names.into_iter().map(Into::into).collect())] attributes: Option<
            Vec<AttributeNameDraft>,
        >,
        pick: Option<Projection>,
        omit: Option<Projection>,
        format: Option<Representation>,
        #[builder(default)] show_changes: ChangeReporting,
        #[builder(default)] system_attributes: SystemAttributeReporting,
        join: Option<Join>,
        join_level: Option<u32>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let attributes = resolve_required_names(SubscriptionField::from(NotificationMember::Attributes), attributes, &mut problems);
        let join_level = join_level.and_then(|level| {
            NonZeroU32::new(level).or_else(|| {
                problems.push(Problem::from(MemberProblem::NotPositive {
                    member: SubscriptionField::from(NotificationMember::JoinLevel),
                    source: NotPositive { number: Number::from(level) },
                }));
                None
            })
        });
        let params = Self {
            attributes,
            pick,
            omit,
            format,
            show_changes,
            system_attributes,
            join,
            join_level,
            endpoint,
            output: (),
        };
        problems.extend(params.rule_problems());
        BuildError::finish(problems, params)
    }
}

impl<D: SubscriptionDirection> NotificationParams<D> {
    /// Returns the attribute names a notification is limited to (`attributes`, deprecated).
    #[must_use]
    pub fn attributes(&self) -> &[AttributeName] {
        &self.attributes
    }

    /// Returns the members every notified entity is reduced to (`pick`).
    #[must_use]
    pub const fn pick(&self) -> Option<&Projection> {
        self.pick.as_ref()
    }

    /// Returns the members removed from every notified entity (`omit`).
    #[must_use]
    pub const fn omit(&self) -> Option<&Projection> {
        self.omit.as_ref()
    }

    /// Returns the representation of the notified entities (`format`); `None` means normalized.
    #[must_use]
    pub const fn format(&self) -> Option<Representation> {
        self.format
    }

    /// Returns whether previous values are shown (`showChanges`).
    #[must_use]
    pub const fn show_changes(&self) -> ChangeReporting {
        self.show_changes
    }

    /// Returns whether system attributes are included (`sysAttrs`).
    #[must_use]
    pub const fn system_attributes(&self) -> SystemAttributeReporting {
        self.system_attributes
    }

    /// Returns the linked entity retrieval (`join`).
    #[must_use]
    pub const fn join(&self) -> Option<Join> {
        self.join
    }

    /// Returns the depth of the linked entity retrieval (`joinLevel`).
    #[must_use]
    pub const fn join_level(&self) -> Option<NonZeroU32> {
        self.join_level
    }

    /// Returns the endpoint the notifications go to (`endpoint`).
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The rule of Table 5.2.14.1-1 that relates members: no `showChanges` with the simplified format. A `joinLevel`
    /// without a `flat` or `inline` join is "only applicable" to them, so the broker ignores it, as Tables 6.4.3.2-1
    /// and 6.5.3.1-1 word the same rule for queries.
    pub(crate) fn rule_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.show_changes == ChangeReporting::WithPrevious && self.format == Some(Representation::Simplified) {
            problems.push(Problem::ShowChangesWithSimplified);
        }
        problems
    }

    /// Converts the parameters into a request's, dropping the members only a broker writes.
    pub(crate) fn into_input(self) -> NotificationParams<Input> {
        NotificationParams {
            attributes: self.attributes,
            pick: self.pick,
            omit: self.omit,
            format: self.format,
            show_changes: self.show_changes,
            system_attributes: self.system_attributes,
            join: self.join,
            join_level: self.join_level,
            endpoint: self.endpoint,
            output: (),
        }
    }
}

impl NotificationParams<Output> {
    /// Returns the members only a broker writes: the record of past notifications.
    #[must_use]
    pub const fn output(&self) -> &NotificationOutputMembers {
        &self.output
    }
}
