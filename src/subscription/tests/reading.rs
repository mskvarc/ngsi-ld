//! Reading subscriptions in both directions, and each refusal of the readers.

use crate::{
    model::{Input, MemberError, NgsiLdMediaType, Output, PathSegment, Representation, SimplifiedSpelling},
    query::{EntitySelectorError, EntitySplitting, Locality},
    subscription::{
        Activation,
        ChangeReporting,
        DocumentType,
        NotificationStatus,
        NotificationTrigger,
        NotificationTriggersError,
        Problem,
        ReadErrorKind,
        Subscription,
        SubscriptionStatus,
        SystemAttributeReporting,
        tests::{read_error, read_problems, round_trip},
    },
};
use pretty_assertions::assert_eq;
use strum::IntoEnumIterator;

/// A notification member holding nothing but the endpoint of Annex C.4.
const NOTIFICATION: &str = r#""notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}"#;

/// A subscription of vehicles with `members` before its notification and nothing after it.
fn vehicles_with(members: &str) -> String {
    format!(r#"{{"type":"Subscription","entities":[{{"type":"Vehicle"}}],{members}{NOTIFICATION}}}"#)
}

/// A subscription of vehicles whose notification holds `members` before its endpoint.
fn notification_with(members: &str) -> String {
    format!(r#"{{"type":"Subscription","entities":[{{"type":"Vehicle"}}],"notification":{{{members}"endpoint":{{"uri":"http://my.endpoint.org/notify"}}}}}}"#)
}

/// A subscription as a broker returns it, with its notification status and its system-generated members.
const RETRIEVED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Subscription:mySubscription","type":"Subscription","entities":[{"type":"Vehicle"}],"#,
    r#""notification":{"format":"normalized","endpoint":{"uri":"http://my.endpoint.org/notify"},"status":"ok","timesSent":3,"timesFailed":1,"#,
    r#""lastNotification":"2024-05-01T10:00:00Z","lastSuccess":"2024-05-01T10:00:00Z","lastFailure":"2024-04-30T10:00:00Z"},"#,
    r#""status":"active","createdAt":"2024-04-01T10:00:00Z","modifiedAt":"2024-04-02T10:00:00Z"}"#
);

#[test]
fn a_retrieved_subscription_round_trips_with_the_members_only_a_broker_writes() {
    let subscription = round_trip::<Output>(RETRIEVED);
    assert_eq!(subscription.id().as_str(), "urn:ngsi-ld:Subscription:mySubscription");
    assert_eq!(subscription.output().status, Some(SubscriptionStatus::Active));
    assert_eq!(
        subscription.output().created_at.as_ref().map(ToString::to_string).as_deref(),
        Some("2024-04-01T10:00:00Z")
    );
    assert_eq!(
        subscription.output().modified_at.as_ref().map(ToString::to_string).as_deref(),
        Some("2024-04-02T10:00:00Z")
    );
    let delivery = subscription.notification().output();
    assert_eq!(delivery.status, Some(NotificationStatus::Ok));
    assert_eq!((delivery.times_sent, delivery.times_failed), (Some(3), Some(1)));
    assert_eq!(delivery.last_failure.as_ref().map(ToString::to_string).as_deref(), Some("2024-04-30T10:00:00Z"));
    let input = subscription.into_input();
    assert_eq!(
        input.render().to_string(),
        r#"{"id":"urn:ngsi-ld:Subscription:mySubscription","type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"format":"normalized","endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#
    );
}

#[test]
fn a_request_refuses_each_member_only_a_broker_writes() {
    for member in [
        r#""status":"active","#,
        r#""createdAt":"2024-04-01T10:00:00Z","#,
        r#""modifiedAt":"2024-04-01T10:00:00Z","#,
    ] {
        let error = read_error::<Input>(&vehicles_with(member));
        assert!(
            matches!(error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { .. })),
            "{member}: {error}"
        );
        assert!(error.path.is_empty());
    }
    for member in [
        r#""status":"ok","#,
        r#""timesSent":3,"#,
        r#""timesFailed":1,"#,
        r#""lastNotification":"2024-05-01T10:00:00Z","#,
        r#""lastSuccess":"2024-05-01T10:00:00Z","#,
        r#""lastFailure":"2024-05-01T10:00:00Z","#,
    ] {
        let error = read_error::<Input>(&notification_with(member));
        assert!(
            matches!(error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { .. })),
            "{member}: {error}"
        );
        assert_eq!(error.path.to_string(), "notification");
    }
}

#[test]
fn a_response_needs_an_id_and_a_request_does_not() {
    let without_id = vehicles_with("");
    assert!(matches!(
        read_error::<Output>(&without_id).kind,
        ReadErrorKind::Member(MemberError::MissingMember { member: "id" })
    ));
    assert!(round_trip::<Input>(&without_id).id().is_none());
}

#[test]
fn the_type_the_notification_and_the_endpoint_uri_are_required_and_unknown_members_refused() {
    let wrong_type = vehicles_with("").replace(r#""type":"Subscription""#, r#""type":"Subscriptions""#);
    assert!(matches!(
        read_error::<Input>(&wrong_type).kind,
        ReadErrorKind::WrongType {
            expected: DocumentType::Subscription,
            ..
        }
    ));
    let untyped = vehicles_with("").replace(r#""type":"Subscription","#, "");
    assert!(matches!(
        read_error::<Input>(&untyped).kind,
        ReadErrorKind::Member(MemberError::MissingMember { member: "type" })
    ));
    let silent = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}]}"#;
    assert!(matches!(
        read_error::<Input>(silent).kind,
        ReadErrorKind::Member(MemberError::MissingMember { member: "notification" })
    ));
    let nowhere = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"accept":"application/json"}}}"#;
    let error = read_error::<Input>(nowhere);
    assert!(matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "uri" })));
    assert_eq!(error.path.to_string(), "notification.endpoint");
    assert!(matches!(
        read_error::<Input>(&vehicles_with(r#""foo":1,"#)).kind,
        ReadErrorKind::Member(MemberError::UnexpectedMember { .. })
    ));
}

#[test]
fn a_read_subscription_obeys_the_rules_of_the_builder() {
    let periodic = vehicles_with(r#""watchedAttributes":["speed"],"timeInterval":60,"#);
    assert_eq!(read_problems::<Input>(&periodic), [Problem::WatchedAttributesWithTimeInterval]);
    let error = read_error::<Input>(&vehicles_with(r#""throttling":0,"#));
    assert!(matches!(error.kind, ReadErrorKind::NotPositive { .. }));
    assert_eq!(error.path.to_string(), "throttling");
}

#[test]
fn each_format_round_trips_and_key_values_is_the_simplified_representation_spelled_otherwise() {
    for format in ["normalized", "concise", "simplified", "keyValues"] {
        round_trip::<Input>(&notification_with(&format!(r#""format":"{format}","#)));
    }
    let key_values = Subscription::<Input>::parse(&notification_with(r#""format":"keyValues","#)).unwrap();
    let simplified = Subscription::<Input>::parse(&notification_with(r#""format":"simplified","#)).unwrap();
    assert_eq!(key_values.notification().format(), Some(Representation::Simplified));
    assert_eq!(simplified.notification().format(), Some(Representation::Simplified));
    assert_eq!(key_values.notification().simplified_spelling(), SimplifiedSpelling::KeyValues);
    assert_eq!(simplified.notification().simplified_spelling(), SimplifiedSpelling::Simplified);
    assert_ne!(key_values, simplified);
    let error = read_error::<Input>(&notification_with(r#""format":"geojson","#));
    assert!(matches!(error.kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "format", .. })));
    assert_eq!(error.path.to_string(), "notification.format");
}

#[test]
fn each_accepted_media_type_round_trips() {
    for accept in [NgsiLdMediaType::Json, NgsiLdMediaType::JsonLd, NgsiLdMediaType::GeoJson] {
        let text = format!(
            r#"{{"type":"Subscription","entities":[{{"type":"Vehicle"}}],"notification":{{"endpoint":{{"uri":"http://my.endpoint.org/notify","accept":"{accept}"}}}}}}"#
        );
        assert_eq!(round_trip::<Input>(&text).notification().endpoint().accept(), Some(accept));
    }
    let plain =
        r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify","accept":"text/plain"}}}"#;
    assert!(matches!(
        read_error::<Input>(plain).kind,
        ReadErrorKind::Member(MemberError::InvalidToken { member: "accept", .. })
    ));
}

#[test]
fn triggers_are_written_in_the_order_of_the_table() {
    let reversed: Vec<String> = NotificationTrigger::iter().rev().map(|trigger| format!(r#""{trigger}""#)).collect();
    let in_order: Vec<String> = NotificationTrigger::iter().map(|trigger| format!(r#""{trigger}""#)).collect();
    let subscription = Subscription::<Input>::parse(&vehicles_with(&format!(r#""notificationTrigger":[{}],"#, reversed.join(",")))).unwrap();
    assert_eq!(
        subscription.render().to_string(),
        vehicles_with(&format!(r#""notificationTrigger":[{}],"#, in_order.join(",")))
    );
    let unknown = read_error::<Input>(&vehicles_with(r#""notificationTrigger":["entityChanged"],"#));
    assert!(matches!(
        unknown.kind,
        ReadErrorKind::Member(MemberError::InvalidToken {
            member: "notificationTrigger",
            ..
        })
    ));
    assert_eq!(
        read_problems::<Input>(&vehicles_with(r#""notificationTrigger":[],"#)),
        [Problem::Triggers {
            source: NotificationTriggersError::Empty
        }]
    );
}

#[test]
fn an_empty_list_and_a_join_level_of_zero_are_refused_where_they_stand() {
    let empty = read_error::<Input>(&vehicles_with(r#""watchedAttributes":[],"#));
    assert_eq!(empty.path.to_string(), "watchedAttributes");
    assert!(matches!(empty.kind, ReadErrorKind::Member(MemberError::Empty)), "{empty}");
    let zero = read_error::<Input>(&notification_with(r#""join":"flat","joinLevel":0,"#));
    assert_eq!(zero.path.to_string(), "notification.joinLevel");
    assert!(matches!(zero.kind, ReadErrorKind::NotPositive { .. }), "{zero}");
}

#[test]
fn each_query_language_member_is_refused_with_its_language_error() {
    for (member, text) in [
        ("q", r#""q":"speed>","#),
        ("scopeQ", r#""scopeQ":"/a//b","#),
        ("lang", r#""lang":"not a tag","#),
        ("csf", r#""csf":")","#),
    ] {
        let error = read_error::<Input>(&vehicles_with(text));
        assert!(matches!(error.kind, ReadErrorKind::Query { .. }), "{member}: {error}");
        assert_eq!(error.path.to_string(), [member].join("."));
    }
    let geo = read_error::<Input>(&vehicles_with(r#""geoQ":{"geometry":"Point","coordinates":[8,40]},"#));
    assert!(matches!(geo.kind, ReadErrorKind::GeoQuery { .. }));
    assert_eq!(geo.path.to_string(), "geoQ");
    let pick = read_error::<Input>(&notification_with(r#""pick":["{"],"#));
    assert!(matches!(pick.kind, ReadErrorKind::Query { .. }));
    assert_eq!(pick.path.to_string(), "notification.pick");
}

#[test]
fn key_value_pairs_round_trip_and_need_both_members() {
    let text = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify","receiverInfo":[{"key":"Prefer","value":"body=json"}]}}}"#;
    let subscription = round_trip::<Input>(text);
    let pair = &subscription.notification().endpoint().receiver_info()[0];
    assert_eq!((pair.key(), pair.value()), ("Prefer", "body=json"));
    let error = read_error::<Input>(&text.replace(r#","value":"body=json""#, ""));
    assert!(matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "value" })));
    assert_eq!(error.path.to_string(), "notification.endpoint.receiverInfo");
}

#[test]
fn explicit_booleans_are_kept() {
    assert_eq!(round_trip::<Input>(&vehicles_with(r#""localOnly":true,"#)).locality(), Locality::Local);
    assert_eq!(
        round_trip::<Input>(&vehicles_with(r#""splitEntities":false,"#)).split_entities(),
        EntitySplitting::Whole
    );
    let text = vehicles_with("").replace(
        NOTIFICATION,
        r#""notification":{"showChanges":false,"sysAttrs":false,"endpoint":{"uri":"http://my.endpoint.org/notify"}},"isActive":true"#,
    );
    let subscription = round_trip::<Input>(&text);
    assert_eq!(subscription.notification().show_changes(), ChangeReporting::CurrentOnly);
    assert_eq!(subscription.notification().system_attributes(), SystemAttributeReporting::Omit);
    assert_eq!(subscription.activation(), Activation::Active);
    let paused = vehicles_with("").replace(NOTIFICATION, &format!(r#"{NOTIFICATION},"isActive":false"#));
    assert_eq!(round_trip::<Input>(&paused).activation(), Activation::Paused);
}

#[test]
fn rd_06_an_invalid_entity_selector_is_placed_under_its_index() {
    let text = format!(r#"{{"type":"Subscription","entities":[{{"type":"Vehicle"}},{{"id":"urn:ngsi-ld:Vehicle:A4567"}}],{NOTIFICATION}}}"#);
    let error = read_error::<Input>(&text);
    assert_eq!(error.path.to_string(), "entities.1");
    assert_eq!(
        error.path.segments().collect::<Vec<_>>(),
        [&PathSegment::Member("entities".to_owned()), &PathSegment::Index(1)]
    );
    assert!(matches!(
        error.kind,
        ReadErrorKind::EntitySelector { source } if matches!(*source, EntitySelectorError::Member(MemberError::MissingMember { member: "type" }))
    ));
}

#[test]
fn an_empty_dataset_id_list_round_trips_and_differs_from_none() {
    let empty = round_trip::<Input>(&vehicles_with(r#""datasetId":[],"#));
    assert_eq!(empty.dataset_ids().map(<[_]>::len), Some(0));
    assert_eq!(round_trip::<Input>(&vehicles_with("")).dataset_ids(), None);
}

#[test]
fn an_mqtt_endpoint_round_trips_with_or_without_a_user_a_password_and_a_port() {
    let endpoints = [
        "mqtt://broker.example/building/forms/1",
        "mqtt://broker.example:8085/building/forms/2",
        "mqtt://user@broker.example/building/forms/3",
        "mqtt://user:password@broker-auth.example/building/forms/4",
        "mqtt://user:password@broker-auth.example:8085/building/forms/5",
        "mqtts://broker.example:8883/building/forms/6",
    ];
    for uri in endpoints {
        let text = vehicles_with("").replace("http://my.endpoint.org/notify", uri);
        let subscription = round_trip::<Input>(&text);
        assert_eq!(subscription.notification().endpoint().uri().as_str(), uri);
    }
}

#[test]
fn an_mqtt_endpoint_whose_port_is_not_a_number_is_refused_as_a_url() {
    let text = vehicles_with("").replace("http://my.endpoint.org/notify", "mqtt://broker.example:{{port}}/building/forms/1");
    let error = read_error::<Input>(&text);
    assert!(
        matches!(
            error.kind,
            ReadErrorKind::Member(MemberError::Url {
                source: url::ParseError::InvalidPort
            })
        ),
        "{error}"
    );
    assert_eq!(error.path.to_string(), "notification.endpoint.uri");
}

#[test]
fn a_request_may_write_the_keywords_for_its_id_and_type_and_those_of_its_selectors() {
    let aliased = format!(r#"{{"@id":"urn:ngsi-ld:Subscription:s1","@type":"Subscription","entities":[{{"@type":"Vehicle"}}],{NOTIFICATION}}}"#);
    let terms = aliased.replace("@id", "id").replace("@type", "type");
    assert_eq!(Subscription::<Input>::parse(&aliased).unwrap().render().to_string(), terms);
    let twice = format!(r#"{{"type":"Subscription","@type":"Subscription","entities":[{{"type":"Vehicle"}}],{NOTIFICATION}}}"#);
    let error = Subscription::<Input>::parse(&twice).unwrap_err();
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "type"),
        "{error}"
    );
}
