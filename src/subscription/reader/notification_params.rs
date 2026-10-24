//! Reading the notification parameters of a subscription (Table 5.2.14.1-1) and their endpoint (Table 5.2.15-1).

use crate::{
    model::{
        DocumentError,
        KeyValuePair,
        NotPositive,
        Number,
        Representation,
        reader::{boolean, count, object, once, required, string, token, unexpected, url, within},
    },
    query::{Join, projection::Projection},
    seal::Seal,
    subscription::{
        ChangeReporting,
        Endpoint,
        EndpointMember,
        NotificationMember,
        NotificationParams,
        Problem,
        ReadErrorKind,
        SubscriptionDirection,
        SystemAttributeReporting,
        direction::sealed::OutputBlock,
        reader::{Emptiness, error, list, names, positive},
    },
};
use jstrict::{Value as Json, object::Entry};
use std::{error::Error, num::NonZeroU32};

/// Reads a projection written as an array of projection terms (`pick`, `omit`, Table 5.2.14.1-1): the terms
/// joined by `,` are one projection of the language of clause 4.21.
pub(crate) fn projection<P: Error>(json: Json, member: &'static str) -> Result<Projection, DocumentError<ReadErrorKind<P>>> {
    let terms = list(json, Emptiness::Refused)?
        .into_iter()
        .map(|term| string(term, member))
        .collect::<Result<Vec<_>, _>>()?;
    terms
        .join(",")
        .parse()
        .map_err(|source| error(ReadErrorKind::Query { source: Box::new(source) }))
}

/// Reads the `notification` member (Table 5.2.14.1-1), with the members only a broker writes when `D` has them.
pub(super) fn read_notification_params<D: SubscriptionDirection>(json: Json) -> Result<NotificationParams<D>, DocumentError<ReadErrorKind<Problem>>> {
    let mut attributes = None;
    let mut pick = None;
    let mut omit = None;
    let mut format = None;
    let mut show_changes = None;
    let mut system_attributes = None;
    let mut join = None;
    let mut join_level = None;
    let mut endpoint = None;
    let mut output = D::NotificationOutput::default();
    for Entry { key, value } in object(json)? {
        let Ok(member) = key.as_str().parse::<NotificationMember>() else {
            return Err(unexpected(key.as_str()));
        };
        let key = member.into();
        match member {
            NotificationMember::Attributes => once(&mut attributes, key, within(key, names(value, key, Emptiness::Refused))?)?,
            NotificationMember::Pick => once(&mut pick, key, within(key, projection(value, key))?)?,
            NotificationMember::Omit => once(&mut omit, key, within(key, projection(value, key))?)?,
            NotificationMember::Format => once(&mut format, key, within(key, token::<Representation, _>(value, key))?)?,
            NotificationMember::ShowChanges => once(&mut show_changes, key, within(key, boolean(&value, key)).map(ChangeReporting::from_flag)?)?,
            NotificationMember::SysAttrs => once(
                &mut system_attributes,
                key,
                within(key, boolean(&value, key)).map(SystemAttributeReporting::from_flag)?,
            )?,
            NotificationMember::Join => once(&mut join, key, within(key, token::<Join, _>(value, key))?)?,
            NotificationMember::JoinLevel => once(&mut join_level, key, within(key, level(value))?)?,
            NotificationMember::Endpoint => once(&mut endpoint, key, within(key, read_endpoint(value))?)?,
            NotificationMember::Status
            | NotificationMember::TimesSent
            | NotificationMember::TimesFailed
            | NotificationMember::LastNotification
            | NotificationMember::LastFailure
            | NotificationMember::LastSuccess => {
                if output.read(Seal::new(), key, value)?.is_some() {
                    return Err(unexpected(key));
                }
            }
        }
    }
    Ok(NotificationParams {
        attributes: attributes.unwrap_or_default(),
        pick,
        omit,
        format,
        show_changes: show_changes.unwrap_or_default(),
        system_attributes: system_attributes.unwrap_or_default(),
        join,
        join_level,
        endpoint: required(endpoint, NotificationMember::Endpoint.into())?,
        output,
    })
}

/// Reads `joinLevel`: a positive integer.
pub(crate) fn level<P: Error>(json: Json) -> Result<NonZeroU32, DocumentError<ReadErrorKind<P>>> {
    let level = count(json, NotificationMember::JoinLevel.into())?;
    u32::try_from(level).ok().and_then(NonZeroU32::new).ok_or_else(|| {
        error(ReadErrorKind::NotPositive {
            source: NotPositive { number: Number::from(level) },
        })
    })
}

/// Reads the `endpoint` member (Table 5.2.15-1).
fn read_endpoint<P: Error>(json: Json) -> Result<Endpoint, DocumentError<ReadErrorKind<P>>> {
    let mut uri = None;
    let mut accept = None;
    let mut timeout = None;
    let mut cooldown = None;
    let mut receiver_info = None;
    let mut notifier_info = None;
    for Entry { key, value } in object(json)? {
        let Ok(member) = key.as_str().parse::<EndpointMember>() else {
            return Err(unexpected(key.as_str()));
        };
        let key = member.into();
        match member {
            EndpointMember::Uri => once(&mut uri, key, within(key, url(value, key))?)?,
            EndpointMember::Accept => once(&mut accept, key, within(key, token(value, key))?)?,
            EndpointMember::Timeout => once(&mut timeout, key, within(key, positive(value, key))?)?,
            EndpointMember::Cooldown => once(&mut cooldown, key, within(key, positive(value, key))?)?,
            EndpointMember::ReceiverInfo => once(&mut receiver_info, key, within(key, key_value_pairs(value))?)?,
            EndpointMember::NotifierInfo => once(&mut notifier_info, key, within(key, key_value_pairs(value))?)?,
        }
    }
    Ok(Endpoint {
        uri: required(uri, EndpointMember::Uri.into())?,
        accept,
        timeout,
        cooldown,
        receiver_info: receiver_info.unwrap_or_default(),
        notifier_info: notifier_info.unwrap_or_default(),
    })
}

/// Reads an array of key-value pairs (clause 5.2.22).
pub(crate) fn key_value_pairs<P: Error>(json: Json) -> Result<Vec<KeyValuePair>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Allowed)?.into_iter().map(key_value_pair).collect()
}

/// Reads one key-value pair: `key` and `value`, both strings (Table 5.2.22-1).
fn key_value_pair<P: Error>(json: Json) -> Result<KeyValuePair, DocumentError<ReadErrorKind<P>>> {
    let mut key = None;
    let mut value = None;
    for Entry { key: member, value: json } in object(json)? {
        match member.as_str() {
            "key" => once(&mut key, "key", within("key", string(json, "key"))?)?,
            "value" => once(&mut value, "value", within("value", string(json, "value"))?)?,
            other => return Err(unexpected(other)),
        }
    }
    Ok(KeyValuePair::new(required(key, "key")?, required(value, "value")?))
}
