//! The two kinds of subscription, and the member only each kind has.

use crate::{
    model::{DateTime, DocumentError, Input, MemberChange, MemberError, Output},
    query::temporal_query::{self, TemporalQuery},
    subscription::{
        Problem as SubscriptionProblem,
        Problem,
        ReadErrorKind,
        RegistrationSubscription,
        Subscription,
        SubscriptionDirection,
        SubscriptionFragment,
        SubscriptionMember,
        tests::{notification, problems, round_trip, vehicles},
    },
};
use pretty_assertions::assert_eq;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// A temporal query between two instants, which only a registration subscription carries.
fn between() -> TemporalQuery {
    temporal_query::between(time("2018-08-01T12:00:00Z"), time("2018-08-01T13:00:00Z"))
        .build()
        .unwrap()
}

/// The `temporalQ` member `between` writes.
const TEMPORAL_Q: &str = r#""temporalQ":{"timerel":"between","timeAt":"2018-08-01T12:00:00Z","endTimeAt":"2018-08-01T13:00:00Z"}"#;

/// A registration subscription of vehicles, with a member `member` between `entities` and `notification`.
fn registration_text(member: &str) -> String {
    format!(r#"{{"type":"Subscription","entities":[{{"type":"Vehicle"}}],{member},"notification":{{"endpoint":{{"uri":"http://my.endpoint.org/notify"}}}}}}"#)
}

/// Reads a registration subscription in direction `D` and checks that it is written back as the same text.
fn registration_round_trip<D: SubscriptionDirection>(text: &str) -> Subscription<D, RegistrationSubscription> {
    let subscription = Subscription::<D, RegistrationSubscription>::parse(text).unwrap();
    assert_eq!(subscription.render().to_string(), text);
    subscription
}

/// The error of a registration subscription read that must fail.
fn registration_read_error(text: &str) -> DocumentError<ReadErrorKind<SubscriptionProblem>> {
    Subscription::<Input, RegistrationSubscription>::parse(text).unwrap_err()
}

#[test]
fn the_temporal_query_of_a_registration_subscription_is_written_after_the_scope_query() {
    let subscription = Subscription::registration_builder()
        .entities([vehicles()])
        .scope_q("/Madrid/#".parse().unwrap())
        .temporal_q(between())
        .notification(notification())
        .build()
        .unwrap();
    let text = subscription.render().to_string();
    assert!(text.contains(&format!(r#""scopeQ":"/Madrid/#",{TEMPORAL_Q},"notification""#)), "{text}");
    assert_eq!(registration_round_trip::<Input>(&text), subscription);
    let retrieved = text.replacen('{', r#"{"id":"urn:ngsi-ld:Subscription:mySubscription","#, 1);
    let read = registration_round_trip::<Output>(&retrieved);
    assert_eq!(read.temporal_q(), Some(&between()));
    assert_eq!(read.into_input().temporal_q(), Some(&between()));
}

#[test]
fn a_registration_subscription_fragment_sets_or_deletes_the_temporal_query() {
    let set = SubscriptionFragment::registration_builder().temporal_q(between()).build().unwrap();
    assert_eq!(set.render().to_string(), format!(r#"{{"type":"Subscription",{TEMPORAL_Q}}}"#));
    let deleted = SubscriptionFragment::registration_builder()
        .delete(SubscriptionMember::TemporalQ)
        .build()
        .unwrap();
    assert_eq!(deleted.render().to_string(), r#"{"type":"Subscription","temporalQ":"urn:ngsi-ld:null"}"#);
}

#[test]
fn an_invalid_temporal_query_is_refused_where_it_stands() {
    let error = registration_read_error(&registration_text(r#""temporalQ":{"timerel":"between","timeAt":"2018-08-01T12:00:00Z"}"#));
    assert_eq!(error.path.to_string(), "temporalQ");
    assert!(matches!(error.kind, ReadErrorKind::TemporalQuery { .. }));
}

#[test]
fn the_context_source_filter_of_an_entity_subscription_is_written_after_the_scope_query() {
    let subscription = Subscription::builder()
        .entities([vehicles()])
        .scope_q("/Madrid/#".parse().unwrap())
        .csf("owner".parse().unwrap())
        .notification(notification())
        .build()
        .unwrap();
    let text = subscription.render().to_string();
    assert!(text.contains(r#""scopeQ":"/Madrid/#","csf":"owner","notification""#), "{text}");
    assert_eq!(round_trip::<Input>(&text).csf().map(ToString::to_string).as_deref(), Some("owner"));
    let fragment = SubscriptionFragment::builder().csf("owner".parse().unwrap()).build().unwrap();
    assert_eq!(fragment.render().to_string(), r#"{"type":"Subscription","csf":"owner"}"#);
}

#[test]
fn the_context_source_filter_of_a_registration_subscription_is_written_after_its_temporal_query() {
    let subscription = Subscription::registration_builder()
        .entities([vehicles()])
        .scope_q("/Madrid/#".parse().unwrap())
        .temporal_q(between())
        .csf("owner".parse().unwrap())
        .notification(notification())
        .build()
        .unwrap();
    let text = subscription.render().to_string();
    assert!(
        text.contains(&format!(r#""scopeQ":"/Madrid/#",{TEMPORAL_Q},"csf":"owner","notification""#)),
        "{text}"
    );
    assert_eq!(registration_round_trip::<Input>(&text), subscription);
    assert_eq!(subscription.csf().map(ToString::to_string).as_deref(), Some("owner"));
    let retrieved = text.replacen('{', r#"{"id":"urn:ngsi-ld:Subscription:mySubscription","#, 1);
    let read = registration_round_trip::<Output>(&retrieved);
    assert_eq!(read.into_input().csf().map(ToString::to_string).as_deref(), Some("owner"));
    let alone = registration_round_trip::<Input>(&registration_text(r#""csf":"owner""#));
    assert_eq!((alone.csf().map(ToString::to_string), alone.temporal_q()), (Some("owner".to_owned()), None));
}

#[test]
fn a_registration_subscription_fragment_sets_or_deletes_the_context_source_filter() {
    let set = SubscriptionFragment::registration_builder().csf("owner".parse().unwrap()).build().unwrap();
    assert_eq!(set.render().to_string(), r#"{"type":"Subscription","csf":"owner"}"#);
    assert_eq!(
        set.csf().map(|change| change.map(ToString::to_string)),
        Some(MemberChange::Set("owner".to_owned()))
    );
    let deleted = SubscriptionFragment::registration_builder().delete(SubscriptionMember::Csf).build().unwrap();
    assert_eq!(deleted.render().to_string(), r#"{"type":"Subscription","csf":"urn:ngsi-ld:null"}"#);
    assert_eq!(
        SubscriptionFragment::<RegistrationSubscription>::parse(r#"{"csf":"owner"}"#).unwrap(),
        SubscriptionFragment::registration_builder().csf("owner".parse().unwrap()).build().unwrap()
    );
}

#[test]
fn a_reader_refuses_the_temporal_query_of_an_entity_subscription() {
    let error = Subscription::<Input>::parse(&registration_text(&TEMPORAL_Q.replace("\\\"", "\""))).unwrap_err();
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "temporalQ"),
        "{error}"
    );
}

#[test]
fn a_fragment_of_an_entity_subscription_refuses_to_delete_the_temporal_query() {
    let entity = SubscriptionFragment::builder().delete(SubscriptionMember::TemporalQ).build();
    assert_eq!(
        problems(entity),
        [Problem::MemberOfOtherKind {
            member: SubscriptionMember::TemporalQ
        }]
    );
}

#[test]
fn both_kinds_share_every_other_member_and_rule() {
    let subscription = Subscription::registration_builder().notification(notification()).build();
    assert_eq!(problems(subscription), [Problem::NoSubscribedEntities]);
    let text = registration_text(r#""watchedAttributes":["speed"],"q":"speed>50""#);
    let read = registration_round_trip::<Input>(&text);
    assert_eq!(read.watched_attributes()[0].as_ref(), "speed");
    assert_eq!(read.temporal_q(), None);
}
