//! Writing subscription documents in the member order `Subscription::render` documents: a subscription,
//! its notification parameters and endpoint, and the members a subscription fragment shares with them.

use crate::{
    model::{AttributeName, DatasetId, MemberChange, PositiveNumber, Representation, SimplifiedSpelling},
    query::{EntitySelector, geoquery::GeoQuery, projection::Projection},
    seal::Seal,
    subscription::{
        DocumentType,
        NotificationMember,
        NotificationParams,
        NotificationTriggers,
        Subscription,
        SubscriptionDirection,
        SubscriptionFragment,
        SubscriptionKind,
        SubscriptionMember,
        attribute_list::{array, comma_separated},
        direction::sealed::{IdMember, OutputBlock},
        kind::sealed::{KindChanges, KindMembers},
        member::SubscriptionMandatoryMember,
    },
};
use jstrict::{Object, Value as Json};
use std::fmt::Display;

/// Appends `member` with `value` when there is one.
pub(crate) fn push(object: &mut Object, member: &str, value: Option<Json>) {
    if let Some(value) = value {
        object.push(member.into(), value);
    }
}

/// Appends `member` with the change `change` makes to it, when there is one: the value `value` writes for a set
/// member, the NGSI-LD Null for a deleted one.
pub(crate) fn push_change<T>(object: &mut Object, member: &str, change: Option<&MemberChange<T>>, value: impl FnOnce(&T) -> Option<Json>) {
    push(object, member, change.and_then(|change| change.write(value)));
}

/// The printed value of a query-language tree, a timestamp or a token.
pub(crate) fn printed(value: &impl Display) -> Json {
    Json::from(value.to_string())
}

/// `entities`.
pub(crate) fn selectors(selectors: &[EntitySelector]) -> Option<Json> {
    (!selectors.is_empty()).then(|| Json::Array(selectors.iter().map(|selector| Json::Object(selector.to_object())).collect()))
}

/// A list of names written as an array, when there is one.
pub(crate) fn name_array(names: &[AttributeName]) -> Option<Json> {
    (!names.is_empty()).then(|| array(names))
}

/// A list of names written as one comma-separated string, when there is one.
pub(crate) fn name_string(names: &[AttributeName]) -> Option<Json> {
    (!names.is_empty()).then(|| comma_separated(names))
}

/// `notificationTrigger`, in the order Table 5.2.12-1 lists the triggers.
pub(crate) fn triggers(triggers: &NotificationTriggers) -> Json {
    Json::Array(triggers.iter().map(|trigger| Json::from(<&str>::from(trigger))).collect())
}

/// A number greater than 0.
pub(crate) fn number(number: &PositiveNumber) -> Json {
    number.as_number().to_json()
}

/// `datasetId`: an array, empty when the document holds an empty one (no table forbids it).
pub(crate) fn dataset_id_array(ids: &[DatasetId]) -> Json {
    Json::Array(ids.iter().map(printed).collect())
}

/// `geoQ`.
pub(crate) fn geo_query(query: &GeoQuery) -> Json {
    query.to_json()
}

/// `pick` or `omit`: one array element per projection term (Table 5.2.14.1-1).
pub(crate) fn projection(projection: &Projection) -> Json {
    Json::Array(projection.terms().iter().map(printed).collect())
}

/// `format`: the token of the representation; the simplified one is named by `spelling`, `simplified` or its
/// synonym `keyValues` (Table 5.2.14-1).
fn format(representation: Representation, spelling: SimplifiedSpelling) -> Json {
    match representation {
        Representation::Simplified => Json::from(<&str>::from(spelling)),
        Representation::Normalized | Representation::Concise => Json::from(<&str>::from(representation)),
    }
}

/// Writes the notification parameters: `attributes`, `pick`, `omit`, `format`, `showChanges`, `sysAttrs`,
/// `join`, `joinLevel`, `endpoint`, then the members only a broker writes.
pub(crate) fn notification_params<D: SubscriptionDirection>(params: &NotificationParams<D>) -> Json {
    let mut object = Object::new();
    push(&mut object, NotificationMember::Attributes.into(), name_array(&params.attributes));
    push(&mut object, NotificationMember::Pick.into(), params.pick.as_ref().map(projection));
    push(&mut object, NotificationMember::Omit.into(), params.omit.as_ref().map(projection));
    push(
        &mut object,
        NotificationMember::Format.into(),
        params.format.map(|representation| format(representation, params.simplified_spelling)),
    );
    push(&mut object, NotificationMember::ShowChanges.into(), params.show_changes.flag().map(Json::from));
    push(
        &mut object,
        NotificationMember::SysAttrs.into(),
        params.system_attributes.flag().map(Json::from),
    );
    push(
        &mut object,
        NotificationMember::Join.into(),
        params.join.map(|join| Json::from(<&str>::from(join))),
    );
    push(
        &mut object,
        NotificationMember::JoinLevel.into(),
        params.join_level.map(|level| Json::from(level.get())),
    );
    object.push(<&str>::from(NotificationMember::Endpoint).into(), params.endpoint.to_json());
    params.output.write(Seal::new(), &mut object);
    Json::Object(object)
}

/// Writes a subscription in canonical member order, without its `@context`: the body of a request, whose context
/// the request pipeline delivers (clause 6.3.5).
pub(crate) fn subscription_object<D: SubscriptionDirection, K: SubscriptionKind>(subscription: &Subscription<D, K>) -> Object {
    let mut object = Object::new();
    push(
        &mut object,
        SubscriptionMandatoryMember::Id.into(),
        subscription.id.get(Seal::new()).map(|id| Json::from(id.as_str())),
    );
    object.push(
        <&str>::from(SubscriptionMandatoryMember::Type).into(),
        Json::from(<&str>::from(DocumentType::Subscription)),
    );
    push(
        &mut object,
        SubscriptionMember::SubscriptionName.into(),
        subscription.name.as_deref().map(Json::from),
    );
    push(
        &mut object,
        SubscriptionMember::Description.into(),
        subscription.description.as_deref().map(Json::from),
    );
    push(&mut object, SubscriptionMember::Entities.into(), selectors(&subscription.entities));
    push(
        &mut object,
        SubscriptionMember::WatchedAttributes.into(),
        name_array(&subscription.watched_attributes),
    );
    push(
        &mut object,
        SubscriptionMember::NotificationTrigger.into(),
        subscription.notification_triggers.as_ref().map(triggers),
    );
    push(
        &mut object,
        SubscriptionMember::TimeInterval.into(),
        subscription.time_interval.as_ref().map(number),
    );
    push(&mut object, SubscriptionMember::Throttling.into(), subscription.throttling.as_ref().map(number));
    push(&mut object, SubscriptionMember::Q.into(), subscription.q.as_ref().map(printed));
    push(&mut object, SubscriptionMember::GeoQ.into(), subscription.geo_q.as_ref().map(geo_query));
    push(&mut object, SubscriptionMember::ScopeQ.into(), subscription.scope_q.as_ref().map(printed));
    subscription.kind_members.write(Seal::new(), &mut object);
    push(
        &mut object,
        SubscriptionMember::DatasetId.into(),
        subscription.dataset_ids.as_deref().map(dataset_id_array),
    );
    push(&mut object, SubscriptionMember::Lang.into(), subscription.lang.as_ref().map(printed));
    push(&mut object, SubscriptionMember::ExpandValues.into(), name_string(&subscription.expand_values));
    push(&mut object, SubscriptionMember::JsonKeys.into(), name_string(&subscription.json_keys));
    push(&mut object, SubscriptionMember::LocalOnly.into(), subscription.locality.flag().map(Json::from));
    push(
        &mut object,
        SubscriptionMember::SplitEntities.into(),
        subscription.split_entities.flag().map(Json::from),
    );
    object.push(
        <&str>::from(SubscriptionMandatoryMember::Notification).into(),
        notification_params(&subscription.notification),
    );
    push(&mut object, SubscriptionMember::IsActive.into(), subscription.activation.flag().map(Json::from));
    push(&mut object, SubscriptionMember::ExpiresAt.into(), subscription.expires_at.as_ref().map(printed));
    push(
        &mut object,
        SubscriptionMember::JsonldContext.into(),
        subscription.jsonld_context.as_ref().map(|url| Json::from(url.as_str())),
    );
    push(
        &mut object,
        SubscriptionMember::NgsildConformance.into(),
        subscription.conformance.as_ref().map(printed),
    );
    subscription.output.write(Seal::new(), &mut object);
    object
}

/// Writes a subscription fragment: `type`, then the members it sets or deletes in canonical order, `notification`
/// among them, without an `@context`.
pub(crate) fn fragment_object<K: SubscriptionKind>(fragment: &SubscriptionFragment<K>) -> Object {
    let mut object = Object::new();
    object.push(
        <&str>::from(SubscriptionMandatoryMember::Type).into(),
        Json::from(<&str>::from(DocumentType::Subscription)),
    );
    push_change(&mut object, SubscriptionMember::SubscriptionName.into(), fragment.name.as_ref(), |name| {
        Some(Json::from(name.as_str()))
    });
    push_change(&mut object, SubscriptionMember::Description.into(), fragment.description.as_ref(), |text| {
        Some(Json::from(text.as_str()))
    });
    push_change(&mut object, SubscriptionMember::Entities.into(), fragment.entities.as_ref(), |entities| {
        selectors(entities)
    });
    push_change(
        &mut object,
        SubscriptionMember::WatchedAttributes.into(),
        fragment.watched_attributes.as_ref(),
        |names| name_array(names),
    );
    push_change(
        &mut object,
        SubscriptionMember::NotificationTrigger.into(),
        fragment.notification_triggers.as_ref(),
        |chosen| Some(triggers(chosen)),
    );
    push_change(
        &mut object,
        SubscriptionMember::TimeInterval.into(),
        fragment.time_interval.as_ref(),
        |period| Some(number(period)),
    );
    push_change(&mut object, SubscriptionMember::Throttling.into(), fragment.throttling.as_ref(), |period| {
        Some(number(period))
    });
    push_change(&mut object, SubscriptionMember::Q.into(), fragment.q.as_ref(), |query| Some(printed(query)));
    push_change(&mut object, SubscriptionMember::GeoQ.into(), fragment.geo_q.as_ref(), |query| {
        Some(geo_query(query))
    });
    push_change(&mut object, SubscriptionMember::ScopeQ.into(), fragment.scope_q.as_ref(), |query| {
        Some(printed(query))
    });
    fragment.kind_members.write(Seal::new(), &mut object);
    // An empty `datasetId` is written, as the list of no dataset.
    push_change(&mut object, SubscriptionMember::DatasetId.into(), fragment.dataset_ids.as_ref(), |ids| {
        Some(dataset_id_array(ids))
    });
    push_change(&mut object, SubscriptionMember::Lang.into(), fragment.lang.as_ref(), |filter| {
        Some(printed(filter))
    });
    push_change(&mut object, SubscriptionMember::ExpandValues.into(), fragment.expand_values.as_ref(), |names| {
        name_string(names)
    });
    push_change(&mut object, SubscriptionMember::JsonKeys.into(), fragment.json_keys.as_ref(), |names| {
        name_string(names)
    });
    push_change(&mut object, SubscriptionMember::LocalOnly.into(), fragment.locality.as_ref(), |locality| {
        locality.flag().map(Json::from)
    });
    push_change(
        &mut object,
        SubscriptionMember::SplitEntities.into(),
        fragment.split_entities.as_ref(),
        |splitting| splitting.flag().map(Json::from),
    );
    if let Some(notification) = &fragment.notification {
        object.push(
            <&str>::from(SubscriptionMandatoryMember::Notification).into(),
            notification_params(notification),
        );
    }
    push_change(&mut object, SubscriptionMember::IsActive.into(), fragment.activation.as_ref(), |activation| {
        activation.flag().map(Json::from)
    });
    push_change(&mut object, SubscriptionMember::ExpiresAt.into(), fragment.expires_at.as_ref(), |time| {
        Some(printed(time))
    });
    push_change(&mut object, SubscriptionMember::JsonldContext.into(), fragment.jsonld_context.as_ref(), |url| {
        Some(Json::from(url.as_str()))
    });
    push_change(
        &mut object,
        SubscriptionMember::NgsildConformance.into(),
        fragment.conformance.as_ref(),
        |version| Some(printed(version)),
    );
    object
}
