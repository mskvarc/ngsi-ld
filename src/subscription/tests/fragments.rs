//! Subscription fragments.

use crate::{
    model::{MemberChange, MemberProblem},
    query::{q::Query, temporal_query},
    subscription::{
        Activation,
        Endpoint,
        EndpointMember,
        NotificationMember,
        NotificationParams,
        Problem,
        SubscriptionField,
        SubscriptionFragment,
        SubscriptionMember,
        tests::problems,
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
fn every_member_a_fragment_deletes_is_reported_as_deleted() {
    for member in SubscriptionMember::iter() {
        let entity = SubscriptionFragment::builder().delete(member).build();
        let registration = SubscriptionFragment::registration_builder().delete(member).build();
        if member == SubscriptionMember::TemporalQ {
            assert_eq!(problems(entity), [Problem::MemberOfOtherKind { member }]);
            assert!(registration.unwrap().deletes(member));
        } else if member == SubscriptionMember::Csf {
            assert!(entity.unwrap().deletes(member));
            assert_eq!(problems(registration), [Problem::MemberOfOtherKind { member }]);
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
