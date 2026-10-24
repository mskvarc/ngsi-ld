//! Building subscriptions, and each refusal of the builders.

use crate::{
    model::{AttributeName, ConformanceVersion, EntityId, Input, MemberProblem, NameError, NgsiLdMediaType, NotPositive, Number, OneOrMany, Representation},
    query::{EntitySelector, EntitySelectorDraft, EntitySplitting, Join, Locality, geoquery::GeoQuery},
    subscription::{
        Activation,
        ChangeReporting,
        Endpoint,
        EndpointMember,
        NotificationMember,
        NotificationParams,
        NotificationTrigger,
        NotificationTriggersError,
        Problem,
        Subscription,
        SubscriptionField,
        SubscriptionFragment,
        SubscriptionMember,
        SystemAttributeReporting,
        tests::{endpoint, notification, problems, round_trip, vehicles},
    },
};
use iri_rs::iri;
use pretty_assertions::assert_eq;
use url::Url;

/// A subscription with every member set.
pub(super) fn full_subscription() -> Subscription<Input> {
    let endpoint = Endpoint::builder("http://my.endpoint.org/notify")
        .accept(NgsiLdMediaType::JsonLd)
        .timeout(2000)
        .cooldown(1000)
        .receiver_info("Authorization", "Bearer abc")
        .notifier_info("MQTT-Version", "mqtt5.0")
        .build()
        .unwrap();
    let notification = NotificationParams::builder(endpoint)
        .attributes(["speed"])
        .pick("speed,brandName".parse().unwrap())
        .omit("owner".parse().unwrap())
        .format(Representation::Concise)
        .show_changes(ChangeReporting::WithPrevious)
        .system_attributes(SystemAttributeReporting::Omit)
        .join(Join::Inline)
        .join_level(2)
        .build()
        .unwrap();
    Subscription::builder()
        .id(iri!("urn:ngsi-ld:Subscription:full"))
        .subscription_name("Speed watch")
        .description("Vehicles above 50")
        .entities([
            vehicles().with_ids(
                OneOrMany::try_from(vec![
                    EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")),
                    EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211")),
                ])
                .unwrap(),
            ),
            vehicles().with_id_pattern("urn:ngsi-ld:Vehicle:.*".parse().unwrap()),
        ])
        .watched_attributes(["speed"])
        .notification_triggers([NotificationTrigger::AttributeUpdated, NotificationTrigger::EntityCreated])
        .throttling(5)
        .q("speed>50".parse().unwrap())
        .geo_q(GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}"#).unwrap())
        .scope_q("/Madrid/#".parse().unwrap())
        .csf("owner".parse().unwrap())
        .dataset_ids(["@none".parse().unwrap(), "urn:ngsi-ld:dataset:gps".parse().unwrap()])
        .lang("fr-CH,fr;q=0.9".parse().unwrap())
        .expand_values(["gender", "colour"])
        .json_keys(["parkingTickets"])
        .locality(Locality::Distributed)
        .split_entities(EntitySplitting::Split)
        .notification(notification)
        .activation(Activation::Paused)
        .expires_at("2024-05-01T10:00:00Z".parse().unwrap())
        .jsonld_context("https://example.org/context.jsonld")
        .ngsild_conformance(ConformanceVersion::new(1, 8))
        .build()
        .unwrap()
}

/// The canonical text of `full_subscription`.
pub(super) const FULL: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Subscription:full","type":"Subscription","subscriptionName":"Speed watch","description":"Vehicles above 50","#,
    r#""entities":[{"type":"Vehicle","id":["urn:ngsi-ld:Vehicle:A4567","urn:ngsi-ld:Vehicle:B9211"]},{"type":"Vehicle","idPattern":"urn:ngsi-ld:Vehicle:.*"}],"#,
    r#""watchedAttributes":["speed"],"notificationTrigger":["entityCreated","attributeUpdated"],"throttling":5,"q":"speed>50","#,
    r#""geoQ":{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]},"scopeQ":"/Madrid/#","csf":"owner","#,
    r#""datasetId":["@none","urn:ngsi-ld:dataset:gps"],"lang":"fr-CH,fr;q=0.9","expandValues":"gender,colour","jsonKeys":"parkingTickets","#,
    r#""localOnly":false,"splitEntities":true,"#,
    r#""notification":{"attributes":["speed"],"pick":["speed","brandName"],"omit":["owner"],"format":"concise","showChanges":true,"sysAttrs":false,"join":"inline","joinLevel":2,"#,
    r#""endpoint":{"uri":"http://my.endpoint.org/notify","accept":"application/ld+json","timeout":2000,"cooldown":1000,"#,
    r#""receiverInfo":[{"key":"Authorization","value":"Bearer abc"}],"notifierInfo":[{"key":"MQTT-Version","value":"mqtt5.0"}]}},"#,
    r#""isActive":false,"expiresAt":"2024-05-01T10:00:00Z","jsonldContext":"https://example.org/context.jsonld","ngsildConformance":"1.8"}"#
);

#[test]
fn a_subscription_of_vehicles_holds_the_entities_and_the_endpoint() {
    let subscription = Subscription::builder().entities([vehicles()]).notification(notification()).build().unwrap();
    assert_eq!(
        subscription.render().to_string(),
        r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#
    );
    assert_eq!(subscription.id(), None);
}

#[test]
fn every_member_is_written_in_canonical_order() {
    let subscription = full_subscription();
    assert_eq!(subscription.render().to_string(), FULL);
    assert_eq!(Subscription::<Input>::parse(FULL).unwrap(), subscription);
}

#[test]
fn a_subscription_needs_entities_or_watched_attributes_unless_it_is_local() {
    assert_eq!(
        problems(Subscription::builder().notification(notification()).build()),
        [Problem::NoSubscribedEntities]
    );
    assert!(Subscription::builder().locality(Locality::Local).notification(notification()).build().is_ok());
    assert!(
        Subscription::builder()
            .watched_attributes(["speed"])
            .notification(notification())
            .build()
            .is_ok()
    );
}

#[test]
fn a_periodic_subscription_watches_no_attribute_needs_entities_and_has_no_throttling() {
    let watching = Subscription::builder()
        .watched_attributes(["speed"])
        .time_interval(60)
        .notification(notification())
        .build();
    assert_eq!(
        problems(watching),
        [Problem::TimeIntervalWithoutEntities, Problem::WatchedAttributesWithTimeInterval]
    );
    let local = Subscription::builder()
        .locality(Locality::Local)
        .time_interval(60)
        .notification(notification())
        .build();
    assert!(local.is_ok());
    let throttled = Subscription::builder()
        .entities([vehicles()])
        .throttling(5)
        .time_interval(60)
        .notification(notification())
        .build();
    assert_eq!(problems(throttled), [Problem::ThrottlingWithTimeInterval]);
}

#[test]
fn periods_and_delays_must_be_greater_than_zero() {
    let minus_one = Number::from(-1);
    let subscription = Subscription::builder()
        .entities([vehicles()])
        .throttling(0)
        .time_interval(minus_one.clone())
        .notification(notification())
        .build();
    assert_eq!(
        problems(subscription),
        [
            not_positive(SubscriptionMember::TimeInterval.into(), minus_one),
            not_positive(SubscriptionMember::Throttling.into(), Number::from(0)),
        ]
    );
    let zero_point_zero = Number::try_from(0.0).unwrap();
    let endpoint = Endpoint::builder("http://my.endpoint.org/notify")
        .timeout(0)
        .cooldown(zero_point_zero.clone())
        .build();
    assert_eq!(
        problems(endpoint),
        [
            not_positive(EndpointMember::Timeout.into(), Number::from(0)),
            not_positive(EndpointMember::Cooldown.into(), zero_point_zero),
        ]
    );
}

/// The problem of a number refused for `member`.
fn not_positive(member: SubscriptionField, number: Number) -> Problem {
    Problem::Member(MemberProblem::NotPositive {
        member,
        source: NotPositive { number },
    })
}

/// The problem of an empty `member`.
fn empty(member: SubscriptionField) -> Problem {
    Problem::Member(MemberProblem::Empty { member })
}

#[test]
fn previous_values_need_a_lossless_format_and_a_join_level_alone_is_kept() {
    let simplified = NotificationParams::builder(endpoint())
        .format(Representation::Simplified)
        .show_changes(ChangeReporting::WithPrevious)
        .build();
    assert_eq!(problems(simplified), [Problem::ShowChangesWithSimplified]);
    assert!(NotificationParams::builder(endpoint()).join_level(2).build().is_ok());
    assert!(NotificationParams::builder(endpoint()).join(Join::None).join_level(2).build().is_ok());
}

#[test]
fn a_selector_of_every_type_implies_local_scope() {
    let distributed = Subscription::builder()
        .entities([EntitySelector::all()])
        .locality(Locality::Distributed)
        .notification(notification())
        .build();
    assert_eq!(problems(distributed), [Problem::AllTypesNotLocal]);
    let every_type = Subscription::builder()
        .entities([EntitySelector::all()])
        .notification(notification())
        .build()
        .unwrap();
    assert!(every_type.render().to_string().contains(r#""entities":[{"type":"*"}]"#));
}

#[test]
fn lists_the_specification_restricts_cannot_be_empty_and_triggers_cannot_repeat() {
    let no_name: [&str; 0] = [];
    assert!(
        problems(
            Subscription::builder()
                .entities([vehicles()])
                .watched_attributes(no_name)
                .notification(notification())
                .build()
        )
        .contains(&empty(SubscriptionMember::WatchedAttributes.into()))
    );
    assert!(
        problems(
            Subscription::builder()
                .entities(Vec::<EntitySelector>::new())
                .watched_attributes(["speed"])
                .notification(notification())
                .build()
        )
        .contains(&empty(SubscriptionMember::Entities.into()))
    );
    assert_eq!(
        problems(NotificationParams::builder(endpoint()).attributes(no_name).build()),
        [empty(NotificationMember::Attributes.into())]
    );
    assert_eq!(
        problems(
            Subscription::builder()
                .entities([vehicles()])
                .notification_triggers([])
                .notification(notification())
                .build()
        ),
        [Problem::Triggers {
            source: NotificationTriggersError::Empty
        }]
    );
    let twice = [NotificationTrigger::EntityCreated, NotificationTrigger::EntityCreated];
    assert_eq!(
        problems(
            Subscription::builder()
                .entities([vehicles()])
                .notification_triggers(twice)
                .notification(notification())
                .build()
        ),
        [Problem::Triggers {
            source: NotificationTriggersError::Duplicate {
                trigger: NotificationTrigger::EntityCreated
            }
        }]
    );
}

#[test]
fn every_invalid_name_and_url_is_reported_together() {
    let subscription = Subscription::builder()
        .entities([vehicles()])
        .watched_attributes(["speed", "bad name"])
        .expand_values(["2x"])
        .jsonld_context("x")
        .notification(notification())
        .build();
    let error = subscription.unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [
                Problem::Member(MemberProblem::InvalidAttributeName {
                    member: SubscriptionField::Subscription(SubscriptionMember::WatchedAttributes),
                    index: 1,
                    source: NameError::InvalidCharacter { found: ' ', position: 3 },
                }),
                Problem::Member(MemberProblem::InvalidAttributeName {
                    member: SubscriptionField::Subscription(SubscriptionMember::ExpandValues),
                    index: 0,
                    ..
                }),
                Problem::Member(MemberProblem::InvalidUrl {
                    member: SubscriptionField::Subscription(SubscriptionMember::JsonldContext),
                    ..
                }),
            ]
        ),
        "{error:?}"
    );
    let shown = error.to_string();
    assert!(
        shown.starts_with("3 problem(s): element 1 of `watchedAttributes` is not a valid attribute name: "),
        "{shown}"
    );
    assert!(shown.contains("; `jsonldContext` is not a valid URL: "), "{shown}");
}

#[test]
fn an_endpoint_reports_its_url_and_its_numbers_with_their_causes() {
    let error = Endpoint::builder("not a url").timeout(0).build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [
                Problem::Member(MemberProblem::InvalidUrl {
                    member: SubscriptionField::Endpoint(EndpointMember::Uri),
                    ..
                }),
                Problem::Member(MemberProblem::NotPositive {
                    member: SubscriptionField::Endpoint(EndpointMember::Timeout),
                    ..
                }),
            ]
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "2 problem(s): `uri` is not a valid URL: relative URL without a base; `timeout` must be greater than 0: 0 is not greater than 0"
    );
}

#[test]
fn typed_names_and_urls_build_unchanged() {
    let speed: AttributeName = "speed".parse().unwrap();
    let url = Url::parse("http://my.endpoint.org/notify").unwrap();
    let endpoint = Endpoint::builder(url.clone()).build().unwrap();
    assert_eq!(endpoint.uri(), &url);
    let subscription = Subscription::builder()
        .entities([vehicles()])
        .watched_attributes([speed.clone()])
        .notification(NotificationParams::builder(endpoint).build().unwrap())
        .build()
        .unwrap();
    assert_eq!(subscription.watched_attributes(), [speed]);
}

#[test]
fn the_full_subscription_round_trips_as_a_request() {
    assert_eq!(round_trip::<Input>(FULL), full_subscription());
}

#[test]
fn an_entity_type_given_as_text_selects_that_type_and_is_checked_when_built() {
    assert_eq!(
        Subscription::builder().entities(["Vehicle"]).notification(notification()).build().unwrap(),
        Subscription::builder().entities([vehicles()]).notification(notification()).build().unwrap()
    );
    let mixed = [EntitySelectorDraft::from(vehicles()), "Car;Bus".into()];
    let error = Subscription::builder().entities(mixed).notification(notification()).build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [Problem::Member(MemberProblem::InvalidSelectedType {
                member: SubscriptionField::Subscription(SubscriptionMember::Entities),
                index: 1,
                ..
            })]
        ),
        "{error:?}"
    );
    let error = SubscriptionFragment::builder().entities(["Car;Bus"]).build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [Problem::Member(MemberProblem::InvalidSelectedType {
                member: SubscriptionField::Subscription(SubscriptionMember::Entities),
                index: 0,
                ..
            })]
        ),
        "{error:?}"
    );
}
