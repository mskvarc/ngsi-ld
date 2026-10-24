//! What each direction of a registration document carries: the identifier a client may leave out, and the members
//! only a broker writes (NGSI-LD Tables 5.2.9-2 and 5.2.2-1).

use crate::{
    model::{
        DateTime,
        DocumentError,
        Input,
        Output,
        reader::{count, date_time, once, token, within},
        representation::write::push_display,
    },
    registration::{RegistrationId, RegistrationOutputMember},
    subscription::{
        ReadErrorKind,
        direction::sealed::{IdMember, OutputBlock, Sealed, required_id},
    },
};
use jstrict::{Object, Value as Json};
use std::{error::Error, fmt::Debug};

/// A direction of a registration document: [`Input`] for what Register Context Source sends (clause 5.9.2.3),
/// [`Output`] for what Retrieve and Query Context Source Registrations return (clauses 5.10.1 and 5.10.2) and a
/// `CSourceNotification` carries.
///
/// The trait is sealed by the same seal as the subscription directions; its associated types decide what a
/// direction carries.
pub trait RegistrationDirection: Sealed + Debug + Clone + PartialEq + Eq {
    /// The registration's `id`: optional in a request, since the broker generates one when it is absent (clause
    /// 5.9.2.4), and always present in a response.
    type Id: IdMember<RegistrationId> + Debug + Clone + PartialEq + Eq;
    /// The members of the registration only a broker writes; `()` in a request.
    type RegistrationOutput: OutputBlock + Debug + Clone + PartialEq + Eq;
}

impl RegistrationDirection for Input {
    type Id = Option<RegistrationId>;
    type RegistrationOutput = ();
}

impl RegistrationDirection for Output {
    type Id = RegistrationId;
    type RegistrationOutput = RegistrationOutputMembers;
}

impl IdMember<RegistrationId> for RegistrationId {
    fn from_read<P: Error>(id: Option<RegistrationId>) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
        required_id(id)
    }

    fn get(&self) -> Option<&RegistrationId> {
        Some(self)
    }
}

/// Whether the last distributed operation a registration triggered succeeded (the `status` of Table 5.2.9-2).
///
/// The enum is exhaustive: the table allows exactly these two values.
///
/// ```
/// use ngsi_ld::registration::RegistrationStatus;
///
/// assert_eq!(
///     "failed".parse::<RegistrationStatus>()?,
///     RegistrationStatus::Failed
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum RegistrationStatus {
    /// `ok`: the last distributed operation succeeded.
    Ok,
    /// `failed`: the last distributed operation failed.
    Failed,
}

/// The members of a registration only a broker writes: `status`, `timesSent`, `timesFailed`, `lastSuccess` and
/// `lastFailure` (Table 5.2.9-2), `createdAt` and `modifiedAt` (Table 5.2.2-1).
///
/// A request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RegistrationOutputMembers {
    /// `status`: whether the last distributed operation succeeded.
    pub status: Option<RegistrationStatus>,
    /// `timesSent`: how many distributed operations the registration triggered, failed ones included.
    pub times_sent: Option<u64>,
    /// `timesFailed`: how many of them failed.
    pub times_failed: Option<u64>,
    /// `lastSuccess`: when the last successful distributed operation was sent.
    pub last_success: Option<DateTime>,
    /// `lastFailure`: when the last distributed operation failed.
    pub last_failure: Option<DateTime>,
    /// `createdAt`.
    pub created_at: Option<DateTime>,
    /// `modifiedAt`.
    pub modified_at: Option<DateTime>,
}

impl OutputBlock for RegistrationOutputMembers {
    fn read<P: Error>(&mut self, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        let Ok(output) = member.parse::<RegistrationOutputMember>() else {
            return Ok(Some(json));
        };
        let key = output.into();
        match output {
            RegistrationOutputMember::Status => once(&mut self.status, key, within(key, token(json, key))?)?,
            RegistrationOutputMember::TimesSent => once(&mut self.times_sent, key, within(key, count(json, key))?)?,
            RegistrationOutputMember::TimesFailed => once(&mut self.times_failed, key, within(key, count(json, key))?)?,
            RegistrationOutputMember::LastSuccess => once(&mut self.last_success, key, within(key, date_time(json, key))?)?,
            RegistrationOutputMember::LastFailure => once(&mut self.last_failure, key, within(key, date_time(json, key))?)?,
            RegistrationOutputMember::CreatedAt => once(&mut self.created_at, key, within(key, date_time(json, key))?)?,
            RegistrationOutputMember::ModifiedAt => once(&mut self.modified_at, key, within(key, date_time(json, key))?)?,
        }
        Ok(None)
    }

    fn write(&self, object: &mut Object) {
        if let Some(status) = self.status {
            object.push(<&str>::from(RegistrationOutputMember::Status).into(), Json::from(<&str>::from(status)));
        }
        if let Some(sent) = self.times_sent {
            object.push(<&str>::from(RegistrationOutputMember::TimesSent).into(), Json::from(sent));
        }
        if let Some(failed) = self.times_failed {
            object.push(<&str>::from(RegistrationOutputMember::TimesFailed).into(), Json::from(failed));
        }
        push_display(object, RegistrationOutputMember::LastSuccess.into(), self.last_success.as_ref());
        push_display(object, RegistrationOutputMember::LastFailure.into(), self.last_failure.as_ref());
        push_display(object, RegistrationOutputMember::CreatedAt.into(), self.created_at.as_ref());
        push_display(object, RegistrationOutputMember::ModifiedAt.into(), self.modified_at.as_ref());
    }
}
