//! The members of a snapshot a client chooses besides its queries (NGSI-LD Table 5.2.41-1): `snapshotLifetime`,
//! `snapshotPriority`, `endpoint` and `receiverInfo`, which a snapshot, the body of Clone Snapshot and the fragment
//! of Update Snapshot Status share.

use crate::{
    model::{
        BuildError,
        DocumentError,
        Duration,
        JsonType,
        KeyValuePair,
        MemberError,
        UrlDraft,
        reader::{duration, number, once, within, written_url},
        written_url::WrittenUrl,
    },
    snapshot::{Problem, SnapshotField, SnapshotMember, SnapshotPriority},
    subscription::{
        ReadErrorKind,
        endpoint::push_pairs,
        reader::{error, notification_params::key_value_pairs},
        writer::{printed, push},
    },
};
use jstrict::{Object, Value as Json};

/// The settings of a snapshot, in the order Table 5.2.41-1 lists them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Settings {
    /// `snapshotLifetime`.
    pub(crate) lifetime: Option<Duration>,
    /// `snapshotPriority`.
    pub(crate) priority: Option<SnapshotPriority>,
    /// `endpoint`.
    pub(crate) endpoint: Option<WrittenUrl>,
    /// `receiverInfo`; `Some` of an empty list when the document wrote an empty array.
    pub(crate) receiver_info: Option<Vec<KeyValuePair>>,
}

impl Settings {
    /// The settings a builder was given, recording a priority out of range and an endpoint that is not a URL.
    pub(crate) fn from_builder(
        lifetime: Option<Duration>,
        priority: Option<u8>,
        endpoint: Option<UrlDraft>,
        receiver_info: Option<Vec<KeyValuePair>>,
        problems: &mut Vec<Problem>,
    ) -> Settings {
        Settings {
            lifetime,
            priority: priority.and_then(|priority| priority_from_builder(priority, problems)),
            endpoint: endpoint.and_then(|url| url.resolve(SnapshotField::from(SnapshotMember::Endpoint), problems)),
            receiver_info,
        }
    }

    /// Reads `member` into the settings when it is one of them, or gives the value back.
    pub(crate) fn read(&mut self, member: &str, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<Problem>>> {
        let Ok(member) = member.parse::<SnapshotMember>() else {
            return Ok(Some(json));
        };
        let key = member.into();
        match member {
            SnapshotMember::SnapshotLifetime => once(&mut self.lifetime, key, within(key, duration(json, key))?)?,
            SnapshotMember::SnapshotPriority => once(&mut self.priority, key, within(key, read_priority(json))?)?,
            SnapshotMember::Endpoint => once(&mut self.endpoint, key, within(key, written_url(json, key))?)?,
            SnapshotMember::ReceiverInfo => once(&mut self.receiver_info, key, within(key, key_value_pairs(json))?)?,
        }
        Ok(None)
    }

    /// Writes the settings that are set, in the order of Table 5.2.41-1.
    pub(crate) fn write(&self, object: &mut Object) {
        push(object, SnapshotMember::SnapshotLifetime.into(), self.lifetime.as_ref().map(printed));
        push(
            object,
            SnapshotMember::SnapshotPriority.into(),
            self.priority.map(|priority| priority.to_number().into_json()),
        );
        push(
            object,
            SnapshotMember::Endpoint.into(),
            self.endpoint.as_ref().map(|endpoint| Json::from(endpoint.as_str())),
        );
        if let Some(pairs) = &self.receiver_info {
            if pairs.is_empty() {
                object.push(<&str>::from(SnapshotMember::ReceiverInfo).into(), Json::Array(jstrict::Array::new()));
            } else {
                push_pairs(object, SnapshotMember::ReceiverInfo.into(), pairs);
            }
        }
    }
}

/// The priority a builder was given, recording one out of range.
pub(crate) fn priority_from_builder(priority: u8, problems: &mut Vec<Problem>) -> Option<SnapshotPriority> {
    SnapshotPriority::try_new(priority)
        .map_err(|source| problems.push(Problem::Priority { source }))
        .ok()
}

/// Reads `snapshotPriority`: an integer from 1 to 10 (Table 5.2.41-1).
pub(crate) fn read_priority(json: Json) -> Result<SnapshotPriority, DocumentError<ReadErrorKind<Problem>>> {
    let member = SnapshotMember::SnapshotPriority.into();
    let integer = number(json, member)?.as_i64().ok_or_else(|| {
        error(
            MemberError::WrongShape {
                member,
                expected: JsonType::Integer,
            }
            .into(),
        )
    })?;
    SnapshotPriority::from_integer(integer).map_err(|source| {
        error(ReadErrorKind::Invalid {
            source: BuildError::single(Problem::Priority { source }),
        })
    })
}
