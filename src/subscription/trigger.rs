//! What kind of change makes a subscription notify (the `notificationTrigger` member of NGSI-LD Table 5.2.12-1).

use std::{
    collections::{BTreeSet, btree_set},
    iter::Copied,
};

/// A kind of change that triggers a notification (Table 5.2.12-1).
///
/// The enum is exhaustive: the table lists exactly these six triggers. They are ordered as the table lists them,
/// which is the order a set of triggers is written in.
///
/// ```
/// use ngsi_ld::subscription::NotificationTrigger;
///
/// assert_eq!(
///     NotificationTrigger::AttributeUpdated.to_string(),
///     "attributeUpdated"
/// );
/// assert_eq!(
///     "entityDeleted".parse::<NotificationTrigger>()?,
///     NotificationTrigger::EntityDeleted
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
pub enum NotificationTrigger {
    /// `entityCreated`.
    EntityCreated,
    /// `entityUpdated`: the combination of `attributeCreated`, `attributeUpdated` and `attributeDeleted`.
    EntityUpdated,
    /// `entityDeleted`.
    EntityDeleted,
    /// `attributeCreated`.
    AttributeCreated,
    /// `attributeUpdated`.
    AttributeUpdated,
    /// `attributeDeleted`.
    AttributeDeleted,
}

/// A trigger list is empty, or names a trigger twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum NotificationTriggersError {
    /// The list names no trigger, so it names none of the valid ones Table 5.2.12-1 lists.
    #[error("a notification trigger list cannot be empty")]
    Empty,
    /// A trigger is listed twice.
    #[error("the notification trigger `{trigger}` is listed twice")]
    Duplicate {
        /// The repeated trigger.
        trigger: NotificationTrigger,
    },
}

/// The non-empty set of triggers a subscription notifies on (Table 5.2.12-1), written in the order the table lists
/// them; a subscription without one notifies on `attributeCreated` and `attributeUpdated`.
///
/// ```
/// use ngsi_ld::subscription::{NotificationTrigger, NotificationTriggers};
///
/// let triggers = NotificationTriggers::try_from_iter([
///     NotificationTrigger::AttributeUpdated,
///     NotificationTrigger::EntityCreated,
/// ])?;
/// assert!(triggers.contains(NotificationTrigger::EntityCreated));
/// assert_eq!(
///     triggers.iter().next(),
///     Some(NotificationTrigger::EntityCreated)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotificationTriggers(BTreeSet<NotificationTrigger>);

impl NotificationTriggers {
    /// Collects triggers into a set, refusing an empty list and a trigger listed twice.
    ///
    /// # Errors
    ///
    /// Returns [`NotificationTriggersError`] for an empty list or a repeated trigger.
    pub fn try_from_iter(triggers: impl IntoIterator<Item = NotificationTrigger>) -> Result<Self, NotificationTriggersError> {
        let mut set = BTreeSet::new();
        for trigger in triggers {
            if !set.insert(trigger) {
                return Err(NotificationTriggersError::Duplicate { trigger });
            }
        }
        if set.is_empty() {
            Err(NotificationTriggersError::Empty)
        } else {
            Ok(Self(set))
        }
    }

    /// Whether the set holds `trigger`.
    #[must_use]
    pub fn contains(&self, trigger: NotificationTrigger) -> bool {
        self.0.contains(&trigger)
    }

    /// Iterates over the triggers in the order Table 5.2.12-1 lists them.
    pub fn iter(&self) -> impl Iterator<Item = NotificationTrigger> + '_ {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a NotificationTriggers {
    type IntoIter = Copied<btree_set::Iter<'a, NotificationTrigger>>;
    type Item = NotificationTrigger;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().copied()
    }
}
