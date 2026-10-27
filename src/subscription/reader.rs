//! Reading subscription documents: a subscription (Table 5.2.12-1), its notification parameters (Table
//! 5.2.14.1-1) and endpoint (Table 5.2.15-1), with the members only a broker writes when the direction has them.

use crate::{
    model::{
        AttributeName,
        BuildError,
        ConformanceVersion,
        DatasetId,
        DocumentError,
        MemberError,
        NameError,
        PositiveNumber,
        keyword::CONTEXT,
        keyword_alias::unalias_members,
        reader::{
            array,
            boolean,
            context,
            date_time,
            empty,
            identifier,
            number,
            object,
            once,
            parse_json,
            parsed,
            required,
            string,
            token,
            unexpected,
            url,
            within,
        },
    },
    query::{EntitySelector, EntitySplitting, Locality, ParseError, geoquery::GeoQuery, temporal_query::TemporalQuery},
    seal::Seal,
    subscription::{
        Activation,
        DocumentType,
        NotificationTriggers,
        Problem,
        ReadErrorKind,
        Subscription,
        SubscriptionDirection,
        SubscriptionKind,
        SubscriptionMember,
        direction::sealed::{IdMember, OutputBlock},
        kind::sealed::KindMembers,
        member::SubscriptionMandatoryMember,
        reader::notification_params::read_notification_params,
        rules,
    },
};
use jstrict::{Object, Value as Json, object::Entry};
use std::{error::Error, str::FromStr};

pub(crate) mod fragment;
pub(crate) mod notification_params;

/// An error of `kind` at the top of the member being read.
pub(crate) const fn error<P: Error>(kind: ReadErrorKind<P>) -> DocumentError<ReadErrorKind<P>> {
    DocumentError::top(kind)
}

/// Reads a number Table 5.2.12-1 or 5.2.15-1 requires to be greater than 0.
pub(crate) fn positive<P: Error>(json: Json, member: &'static str) -> Result<PositiveNumber, DocumentError<ReadErrorKind<P>>> {
    PositiveNumber::try_from(number(json, member)?).map_err(|source| error(ReadErrorKind::NotPositive { source }))
}

/// Reads a member written in a query language.
pub(crate) fn language<T: FromStr<Err = ParseError>, P: Error>(json: Json, member: &'static str) -> Result<T, DocumentError<ReadErrorKind<P>>> {
    string(json, member)?
        .parse()
        .map_err(|source| error(ReadErrorKind::Query { source: Box::new(source) }))
}

/// Reads an array member, refusing an empty one when the specification forbids it.
pub(crate) fn list<P: Error>(json: Json, emptiness: Emptiness) -> Result<Vec<Json>, DocumentError<ReadErrorKind<P>>> {
    let items = array(json)?;
    match emptiness {
        Emptiness::Refused if items.is_empty() => Err(empty()),
        Emptiness::Refused | Emptiness::Allowed => Ok(items),
    }
}

/// Whether an array member may be empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Emptiness {
    /// The specification forbids an empty array.
    Refused,
    /// The specification places no restriction.
    Allowed,
}

/// Reads an array of attribute names, or of the names the `attrs` of a temporal query takes.
pub(crate) fn names<T: FromStr<Err = NameError>, P: Error>(
    json: Json,
    member: &'static str,
    emptiness: Emptiness,
) -> Result<Vec<T>, DocumentError<ReadErrorKind<P>>> {
    list(json, emptiness)?.into_iter().map(|item| parsed(item, member)).collect()
}

/// Reads attribute names written as one comma-separated string (`expandValues`, `jsonKeys`).
pub(crate) fn comma_separated_names<P: Error>(json: Json, member: &'static str) -> Result<Vec<AttributeName>, DocumentError<ReadErrorKind<P>>> {
    let text = string(json, member)?;
    text.split(',')
        .filter(|name| !name.is_empty())
        .map(|item| item.parse().map_err(|source| error(MemberError::Name { source }.into())))
        .collect()
}

/// Checks the `type` of a document.
pub(crate) fn expect_type<P: Error>(json: Json, expected: DocumentType) -> Result<(), DocumentError<ReadErrorKind<P>>> {
    let found = string(json, "type")?;
    if found == <&str>::from(expected) {
        Ok(())
    } else {
        Err(error(ReadErrorKind::WrongType { expected, found }))
    }
}

/// Reads `ngsildConformance`, written as the member `member`.
pub(crate) fn conformance_version<P: Error>(json: Json, member: &'static str) -> Result<ConformanceVersion, DocumentError<ReadErrorKind<P>>> {
    string(json, member)?.parse().map_err(|source| error(ReadErrorKind::Conformance { source }))
}

/// Reads a `geoQ` member.
pub(crate) fn geo_query<P: Error>(json: Json) -> Result<GeoQuery, DocumentError<ReadErrorKind<P>>> {
    GeoQuery::from_json(json).map_err(|error| error.map_kind(|source| ReadErrorKind::GeoQuery { source: Box::new(source) }))
}

/// Reads a `temporalQ` member.
pub(crate) fn temporal_query<P: Error>(json: Json) -> Result<TemporalQuery, DocumentError<ReadErrorKind<P>>> {
    TemporalQuery::from_json(json).map_err(|error| error.map_kind(|source| ReadErrorKind::TemporalQuery { source: Box::new(source) }))
}

/// Reads the text of a subscription of kind `K` in direction `D`.
pub(crate) fn read_subscription<D: SubscriptionDirection, K: SubscriptionKind>(
    text: &str,
) -> Result<Subscription<D, K>, DocumentError<ReadErrorKind<Problem>>> {
    read_subscription_json(parse_json(text)?)
}

/// Reads a subscription of kind `K` in direction `D` from JSON already parsed, such as an element of the array
/// Query Subscriptions answers with.
pub(crate) fn read_subscription_json<D: SubscriptionDirection, K: SubscriptionKind>(
    json: Json,
) -> Result<Subscription<D, K>, DocumentError<ReadErrorKind<Problem>>> {
    let subscription = read_subscription_object::<D, K>(unalias_members(object(json)?)?)?;
    BuildError::finish(rules::subscription_problems(&subscription), ()).map_err(|source| error(ReadErrorKind::Invalid { source }))?;
    Ok(subscription)
}

/// Reads the members of a subscription object.
fn read_subscription_object<D: SubscriptionDirection, K: SubscriptionKind>(
    object: Object,
) -> Result<Subscription<D, K>, DocumentError<ReadErrorKind<Problem>>> {
    let mut id = None;
    let mut typed = None;
    let mut name = None;
    let mut description = None;
    let mut entities = None;
    let mut watched = None;
    let mut triggers = None;
    let mut time_interval = None;
    let mut throttling = None;
    let mut q = None;
    let mut geo_q = None;
    let mut scope_q = None;
    let mut kind_members = K::Members::default();
    let mut csf = None;
    let mut dataset_ids = None;
    let mut lang = None;
    let mut expand_values = None;
    let mut json_keys = None;
    let mut locality = None;
    let mut split_entities = None;
    let mut notification = None;
    let mut activation = None;
    let mut expires_at = None;
    let mut jsonld_context = None;
    let mut conformance = None;
    let mut subscription_context = None;
    let mut output = D::SubscriptionOutput::default();
    for Entry { key, value } in object {
        let key = key.as_str();
        let Ok(member) = key.parse::<SubscriptionMember>() else {
            if let Ok(mandatory) = key.parse::<SubscriptionMandatoryMember>() {
                let key = mandatory.into();
                match mandatory {
                    SubscriptionMandatoryMember::Id => once(&mut id, key, within(key, identifier(value, key))?)?,
                    SubscriptionMandatoryMember::Type => once(&mut typed, key, within(key, expect_type(value, DocumentType::Subscription))?)?,
                    SubscriptionMandatoryMember::Notification => once(&mut notification, key, within(key, read_notification_params::<D>(value))?)?,
                }
            } else if key == CONTEXT {
                once(&mut subscription_context, key, within(key, context(value))?)?;
            } else if output.read(Seal::new(), key, value)?.is_some() {
                return Err(unexpected(key));
            }
            continue;
        };
        let key = member.into();
        match member {
            SubscriptionMember::SubscriptionName => once(&mut name, key, within(key, string(value, key))?)?,
            SubscriptionMember::Description => once(&mut description, key, within(key, string(value, key))?)?,
            SubscriptionMember::Entities => once(&mut entities, key, within(key, selectors(value))?)?,
            SubscriptionMember::WatchedAttributes => once(&mut watched, key, within(key, names(value, key, Emptiness::Refused))?)?,
            SubscriptionMember::NotificationTrigger => once(&mut triggers, key, within(key, notification_triggers(value, key))?)?,
            SubscriptionMember::TimeInterval => once(&mut time_interval, key, within(key, positive(value, key))?)?,
            SubscriptionMember::Throttling => once(&mut throttling, key, within(key, positive(value, key))?)?,
            SubscriptionMember::Q => once(&mut q, key, within(key, language(value, key))?)?,
            SubscriptionMember::GeoQ => once(&mut geo_q, key, within(key, geo_query(value))?)?,
            SubscriptionMember::ScopeQ => once(&mut scope_q, key, within(key, language(value, key))?)?,
            SubscriptionMember::TemporalQ => refuse_unread(kind_members.read(Seal::new(), member, value)?, key)?,
            SubscriptionMember::Csf => once(&mut csf, key, within(key, language(value, key))?)?,
            SubscriptionMember::DatasetId => once(&mut dataset_ids, key, within(key, dataset_id_list(value, key))?)?,
            SubscriptionMember::Lang => once(&mut lang, key, within(key, language(value, key))?)?,
            SubscriptionMember::ExpandValues => once(&mut expand_values, key, within(key, comma_separated_names(value, key))?)?,
            SubscriptionMember::JsonKeys => once(&mut json_keys, key, within(key, comma_separated_names(value, key))?)?,
            SubscriptionMember::LocalOnly => once(&mut locality, key, within(key, boolean(&value, key)).map(Locality::from_flag)?)?,
            SubscriptionMember::SplitEntities => once(&mut split_entities, key, within(key, boolean(&value, key)).map(EntitySplitting::from_flag)?)?,
            SubscriptionMember::IsActive => once(&mut activation, key, within(key, boolean(&value, key)).map(Activation::from_flag)?)?,
            SubscriptionMember::ExpiresAt => once(&mut expires_at, key, within(key, date_time(value, key))?)?,
            SubscriptionMember::JsonldContext => once(&mut jsonld_context, key, within(key, url(value, key))?)?,
            SubscriptionMember::NgsildConformance => once(&mut conformance, key, within(key, conformance_version(value, key))?)?,
        }
    }
    required(typed, SubscriptionMandatoryMember::Type.into())?;
    Ok(Subscription {
        id: D::Id::from_read(Seal::new(), id)?,
        name,
        description,
        entities: entities.unwrap_or_default(),
        watched_attributes: watched.unwrap_or_default(),
        notification_triggers: triggers,
        time_interval,
        throttling,
        q,
        geo_q,
        scope_q,
        kind_members,
        csf,
        dataset_ids,
        lang,
        expand_values: expand_values.unwrap_or_default(),
        json_keys: json_keys.unwrap_or_default(),
        locality: locality.unwrap_or_default(),
        split_entities: split_entities.unwrap_or_default(),
        notification: required(notification, SubscriptionMandatoryMember::Notification.into())?,
        activation: activation.unwrap_or_default(),
        expires_at,
        jsonld_context,
        conformance,
        output,
        context: subscription_context,
    })
}

/// Refuses `key`, a member of the other kind of subscription, when the member block of this kind handed its value
/// back unread (Table 5.2.12-1).
pub(crate) fn refuse_unread(unread: Option<Json>, key: &str) -> Result<(), DocumentError<ReadErrorKind<Problem>>> {
    unread.map_or(Ok(()), |_| Err(unexpected(key)))
}

/// Reads `entities`, written as the member `member`: a non-empty array of entity selectors (clause 5.2.33).
pub(crate) fn selectors<P: Error>(json: Json) -> Result<Vec<EntitySelector>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Refused)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            EntitySelector::from_json(item).map_err(|error| {
                error
                    .map_kind(|source| ReadErrorKind::EntitySelector { source: Box::new(source) })
                    .at_index(index)
            })
        })
        .collect()
}

/// Reads `notificationTrigger`, written as the member `member`: a non-empty array of triggers, none repeated.
pub(crate) fn notification_triggers(json: Json, member: &'static str) -> Result<NotificationTriggers, DocumentError<ReadErrorKind<Problem>>> {
    let triggers = list(json, Emptiness::Allowed)?
        .into_iter()
        .map(|item| token(item, member))
        .collect::<Result<Vec<_>, _>>()?;
    NotificationTriggers::try_from_iter(triggers).map_err(|source| {
        error(ReadErrorKind::Invalid {
            source: BuildError::single(Problem::Triggers { source }),
        })
    })
}

/// Reads `datasetId`, written as the member `member`: an array of dataset ids, `@none` for the default instance.
pub(crate) fn dataset_id_list<P: Error>(json: Json, member: &'static str) -> Result<Vec<DatasetId>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Allowed)?.into_iter().map(|item| identifier(item, member)).collect()
}
