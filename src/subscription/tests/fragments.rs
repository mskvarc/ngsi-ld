//! Subscription fragments.

use crate::{
    model::{MemberChange, MemberError, MemberProblem},
    query::{q::Query, temporal_query},
    subscription::{
        Activation,
        Endpoint,
        EndpointMember,
        EntitySubscription,
        NotificationMember,
        NotificationParams,
        Problem,
        ReadErrorKind,
        RegistrationSubscription,
        SubscriptionField,
        SubscriptionFragment,
        SubscriptionMember,
        read_error::tests::broken_rules,
        tests::{building::FULL, problems},
    },
};
use pretty_assertions::assert_eq;
use strum::IntoEnumIterator;
use url::Url;

#[test]
fn a_fragment_sets_q_and_deletes_the_watched_attributes() {
    let fragment = SubscriptionFragment::builder()
        .q("speed>50".parse().unwrap())
        .delete(SubscriptionMember::WatchedAttributes)
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"Subscription","watchedAttributes":"urn:ngsi-ld:null","q":"speed>50"}"#
    );
    assert!(fragment.deletes(SubscriptionMember::WatchedAttributes));
    assert!(!fragment.deletes(SubscriptionMember::Q));
}

#[test]
fn a_fragment_replaces_the_notification_whole() {
    let endpoint = Endpoint::builder("http://example.org/newNotificationEndPoint").build().unwrap();
    let fragment = SubscriptionFragment::builder()
        .notification(NotificationParams::builder(endpoint).build().unwrap())
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"Subscription","notification":{"endpoint":{"uri":"http://example.org/newNotificationEndPoint"}}}"#
    );
}

#[test]
fn a_fragment_pauses_until_a_date_and_deletes_members_in_canonical_order() {
    let pause = SubscriptionFragment::builder()
        .activation(Activation::Paused)
        .expires_at("2024-05-01T10:00:00Z".parse().unwrap())
        .build()
        .unwrap();
    assert_eq!(
        pause.render().to_string(),
        r#"{"type":"Subscription","isActive":false,"expiresAt":"2024-05-01T10:00:00Z"}"#
    );
    let deletions = SubscriptionFragment::builder()
        .delete(SubscriptionMember::Throttling)
        .delete(SubscriptionMember::Lang)
        .delete(SubscriptionMember::GeoQ)
        .delete(SubscriptionMember::ExpiresAt)
        .build()
        .unwrap();
    assert_eq!(
        deletions.render().to_string(),
        r#"{"type":"Subscription","throttling":"urn:ngsi-ld:null","geoQ":"urn:ngsi-ld:null","lang":"urn:ngsi-ld:null","expiresAt":"urn:ngsi-ld:null"}"#
    );
}

#[test]
fn a_fragment_refuses_members_a_subscription_cannot_hold_together() {
    let watching = SubscriptionFragment::builder().watched_attributes(["speed"]).time_interval(60).build();
    assert_eq!(problems(watching), [Problem::WatchedAttributesWithTimeInterval]);
    let throttled = SubscriptionFragment::builder().throttling(5).time_interval(60).build();
    assert_eq!(problems(throttled), [Problem::ThrottlingWithTimeInterval]);
    let both = SubscriptionFragment::builder()
        .q("speed>50".parse().unwrap())
        .delete(SubscriptionMember::Q)
        .build();
    assert_eq!(
        problems(both),
        [Problem::Member(MemberProblem::SetAndDeleted {
            member: SubscriptionField::Subscription(SubscriptionMember::Q)
        })]
    );
    let no_name: [&str; 0] = [];
    assert_eq!(
        problems(SubscriptionFragment::builder().watched_attributes(no_name).build()),
        [Problem::Member(MemberProblem::Empty {
            member: SubscriptionField::Subscription(SubscriptionMember::WatchedAttributes)
        })]
    );
}

#[test]
fn a_fragment_body_with_every_member_of_an_entity_subscription_is_written_back_as_it_was_read() {
    let body = FULL.replacen(r#""id":"urn:ngsi-ld:Subscription:full","#, "", 1);
    let fragment = SubscriptionFragment::<EntitySubscription>::parse(&body).unwrap();
    assert_eq!(fragment.render().to_string(), body);
    assert_eq!(fragment.q(), Some(MemberChange::Set(&"speed>50".parse().unwrap())));
    assert_eq!(fragment.csf(), Some(MemberChange::Set(&"owner".parse().unwrap())));
    assert_eq!(fragment.activation(), Some(MemberChange::Set(Activation::Paused)));
}

#[test]
fn a_fragment_body_reads_back_as_the_builder_writes_it_in_both_kinds() {
    let entity = SubscriptionFragment::builder()
        .q("speed>50".parse().unwrap())
        .csf("owner".parse().unwrap())
        .delete(SubscriptionMember::WatchedAttributes)
        .delete(SubscriptionMember::Lang)
        .build()
        .unwrap();
    assert_eq!(SubscriptionFragment::<EntitySubscription>::parse(&entity.render().to_string()).unwrap(), entity);
    let temporal = temporal_query::between("2018-08-01T12:00:00Z".parse().unwrap(), "2018-08-01T13:00:00Z".parse().unwrap())
        .build()
        .unwrap();
    let registration = SubscriptionFragment::registration_builder()
        .temporal_q(temporal)
        .delete(SubscriptionMember::Throttling)
        .build()
        .unwrap();
    assert_eq!(
        SubscriptionFragment::<RegistrationSubscription>::parse(&registration.render().to_string()).unwrap(),
        registration
    );
    let deleted = SubscriptionFragment::registration_builder()
        .delete(SubscriptionMember::TemporalQ)
        .build()
        .unwrap();
    assert_eq!(
        SubscriptionFragment::<RegistrationSubscription>::parse(&deleted.render().to_string()).unwrap(),
        deleted
    );
}

#[test]
fn a_fragment_body_may_leave_out_its_type_and_name_its_target_and_carry_a_context() {
    let paused = SubscriptionFragment::builder().activation(Activation::Paused).build().unwrap();
    assert_eq!(SubscriptionFragment::<EntitySubscription>::parse(r#"{"isActive":false}"#).unwrap(), paused);
    let named = concat!(
        r#"{"@context":"https://example.org/context.jsonld","id":"urn:ngsi-ld:Subscription:mySubscription","#,
        r#""type":"Subscription","isActive":false}"#
    );
    let fragment = SubscriptionFragment::<EntitySubscription>::parse(named).unwrap();
    assert_eq!(fragment, paused);
    // The fragment is always written with its `type`, which Tables 6.11.3.2-1 and 6.13.3.2-1 ask for.
    assert_eq!(fragment.render().to_string(), r#"{"type":"Subscription","isActive":false}"#);
}

#[test]
fn a_fragment_body_refuses_what_a_subscription_cannot_hold() {
    let refused = |body: &str| SubscriptionFragment::<EntitySubscription>::parse(body).unwrap_err();
    let unexpected =
        |body: &str, member: &str| matches!(refused(body).kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member: ref found }) if found == member);
    // The notification replaces the whole member and cannot be deleted.
    assert!(SubscriptionFragment::<EntitySubscription>::parse(r#"{"notification":"urn:ngsi-ld:null"}"#).is_err());
    assert!(SubscriptionFragment::<EntitySubscription>::parse(r#"{"notification":{}}"#).is_err());
    assert!(SubscriptionFragment::<EntitySubscription>::parse(r#"{"entities":[]}"#).is_err());
    assert!(
        SubscriptionFragment::<EntitySubscription>::parse(r#"{"entities":"urn:ngsi-ld:null"}"#)
            .is_ok_and(|fragment| fragment.deletes(SubscriptionMember::Entities))
    );
    assert!(unexpected(r#"{"temporalQ":"urn:ngsi-ld:null"}"#, "temporalQ"));
    assert!(unexpected(r#"{"status":"active"}"#, "status"));
    assert!(unexpected(r#"{"createdAt":"2024-05-01T10:00:00Z"}"#, "createdAt"));
    assert!(matches!(
        refused(r#"{"type":"ContextSourceRegistration"}"#).kind,
        ReadErrorKind::WrongType { .. }
    ));
    assert!(matches!(refused(r#"{"throttling":0}"#).kind, ReadErrorKind::NotPositive { .. }));
    assert!(matches!(
        refused(r#"{"q":"speed>50","q":"urn:ngsi-ld:null"}"#).kind,
        ReadErrorKind::Member(MemberError::DuplicateMember { .. })
    ));
    assert_eq!(
        broken_rules(refused(r#"{"timeInterval":60,"watchedAttributes":["speed"],"throttling":5}"#)),
        [Problem::WatchedAttributesWithTimeInterval, Problem::ThrottlingWithTimeInterval]
    );
    assert_eq!(
        broken_rules(refused(
            r#"{"notification":{"format":"simplified","showChanges":true,"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#
        )),
        [Problem::ShowChangesWithSimplified]
    );
}

#[test]
fn every_member_a_fragment_deletes_is_reported_as_deleted() {
    for member in SubscriptionMember::iter() {
        let entity = SubscriptionFragment::builder().delete(member).build();
        let registration = SubscriptionFragment::registration_builder().delete(member).build();
        if member == SubscriptionMember::TemporalQ {
            assert_eq!(problems(entity), [Problem::MemberOfOtherKind { member }]);
            assert!(registration.unwrap().deletes(member));
        } else {
            assert!(entity.unwrap().deletes(member), "{member}");
            assert!(registration.unwrap().deletes(member), "{member}");
        }
    }
}

#[test]
fn sf_01_a_fragment_reads_back_the_members_it_sets_and_deletes() {
    let query: Query = "speed>50".parse().unwrap();
    let fragment = SubscriptionFragment::builder()
        .q(query.clone())
        .delete(SubscriptionMember::Throttling)
        .build()
        .unwrap();
    assert_eq!(fragment.q(), Some(MemberChange::Set(&query)));
    assert_eq!(fragment.throttling(), Some(MemberChange::Delete));
    assert_eq!(fragment.description(), None);
    assert_eq!(fragment.csf(), None);
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"Subscription","throttling":"urn:ngsi-ld:null","q":"speed>50"}"#
    );
}

#[test]
fn sf_02_each_kind_reads_back_its_own_member() {
    let csf: Query = "p==1".parse().unwrap();
    let entity = SubscriptionFragment::builder().csf(csf.clone()).build().unwrap();
    assert_eq!(entity.csf(), Some(MemberChange::Set(&csf)));
    assert_eq!(entity.render().to_string(), r#"{"type":"Subscription","csf":"p==1"}"#);
    let registration_csf = SubscriptionFragment::registration_builder().csf(csf.clone()).build().unwrap();
    assert_eq!(registration_csf.csf(), Some(MemberChange::Set(&csf)));
    let temporal = temporal_query::between("2018-08-01T12:00:00Z".parse().unwrap(), "2018-08-01T13:00:00Z".parse().unwrap())
        .build()
        .unwrap();
    let registration = SubscriptionFragment::registration_builder().temporal_q(temporal.clone()).build().unwrap();
    assert_eq!(registration.temporal_q(), Some(MemberChange::Set(&temporal)));
}

#[test]
fn sf_03_typed_members_read_back_as_given() {
    let fragment = SubscriptionFragment::builder()
        .subscription_name("vehicles")
        .watched_attributes(["speed"])
        .expand_values(["brand"])
        .activation(Activation::Paused)
        .jsonld_context("http://example.org/context.jsonld")
        .build()
        .unwrap();
    assert_eq!(fragment.name(), Some(MemberChange::Set("vehicles")));
    assert_eq!(
        fragment
            .watched_attributes()
            .map(|change| change.map(|names| names.iter().map(AsRef::as_ref).collect::<Vec<&str>>())),
        Some(MemberChange::Set(vec!["speed"]))
    );
    assert_eq!(fragment.expand_values().map(|change| change.map(<[_]>::len)), Some(MemberChange::Set(1)));
    assert_eq!(fragment.activation(), Some(MemberChange::Set(Activation::Paused)));
    assert_eq!(
        fragment.jsonld_context().map(|change| change.map(Url::as_str)),
        Some(MemberChange::Set("http://example.org/context.jsonld"))
    );
    assert!(!fragment.deletes(SubscriptionMember::IsActive));
}

#[test]
fn me_01_every_subscription_member_parses_back_from_its_name() {
    for member in SubscriptionMember::iter() {
        assert_eq!(member.to_string().parse::<SubscriptionMember>(), Ok(member));
    }
}

/// Asserts that every variant of each member enum is written and read back as itself.
macro_rules! members_round_trip {
    ($($member:ty),+ $(,)?) => {$(
        for member in <$member>::iter() {
            assert_eq!(member.to_string().parse::<$member>(), Ok(member));
        }
    )+};
}

#[test]
fn every_member_name_is_written_and_read_back() {
    members_round_trip!(SubscriptionMember, NotificationMember, EndpointMember);
    assert_eq!(SubscriptionField::from(EndpointMember::Uri).to_string(), "uri");
    assert_eq!(SubscriptionField::from(NotificationMember::JoinLevel).to_string(), "joinLevel");
    assert_eq!(SubscriptionField::from(SubscriptionMember::Q).to_string(), "q");
}
