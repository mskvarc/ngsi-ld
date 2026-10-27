//! The bodies of Clone Snapshot and Update Snapshot Status.

use crate::{
    model::{MemberError, MemberProblem},
    snapshot::{
        AggregationMember,
        Problem,
        QueryMember,
        SnapshotClone,
        SnapshotField,
        SnapshotFragment,
        SnapshotMember,
        tests::{missing, refused, unexpected},
    },
    subscription::ReadErrorKind,
};
use iri_rs::iri;
use pretty_assertions::assert_eq;
use strum::IntoEnumIterator;

#[test]
fn a_clone_names_itself_and_chooses_its_settings() {
    let clone = SnapshotClone::builder()
        .id(iri!("urn:ngsi-ld:Snapshot:2"))
        .lifetime("P1D".parse().unwrap())
        .priority(3)
        .build()
        .unwrap();
    assert_eq!(
        clone.render().to_string(),
        r#"{"id":"urn:ngsi-ld:Snapshot:2","type":"Snapshot","snapshotLifetime":"P1D","snapshotPriority":3}"#
    );
    assert_eq!(SnapshotClone::builder().build().unwrap().render().to_string(), r#"{"type":"Snapshot"}"#);
}

#[test]
fn a_fragment_sets_and_deletes_settings_in_canonical_order() {
    let fragment = SnapshotFragment::builder()
        .delete(SnapshotMember::Endpoint)
        .lifetime("PT2H".parse().unwrap())
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"Snapshot","snapshotLifetime":"PT2H","endpoint":"urn:ngsi-ld:null"}"#
    );
    assert!(fragment.deletes(SnapshotMember::Endpoint));
    let error = SnapshotFragment::builder()
        .priority(4)
        .delete(SnapshotMember::SnapshotPriority)
        .build()
        .unwrap_err();
    assert_eq!(
        error.problems(),
        [Problem::Member(MemberProblem::SetAndDeleted {
            member: SnapshotField::Snapshot(SnapshotMember::SnapshotPriority)
        })]
    );
}

#[test]
fn every_member_a_fragment_deletes_is_reported_as_deleted() {
    for member in SnapshotMember::iter() {
        assert!(SnapshotFragment::builder().delete(member).build().unwrap().deletes(member), "{member}");
    }
}

#[test]
fn a_clone_body_reads_back_as_written_with_or_without_an_id() {
    let named = SnapshotClone::builder()
        .id(iri!("urn:ngsi-ld:Snapshot:2"))
        .lifetime("P1D".parse().unwrap())
        .priority(3)
        .endpoint("http://my.endpoint.org/snapshots")
        .build()
        .unwrap();
    assert_eq!(SnapshotClone::parse(&named.render().to_string()).unwrap(), named);
    let anonymous = SnapshotClone::builder().build().unwrap();
    assert_eq!(SnapshotClone::parse(r#"{"type":"Snapshot"}"#).unwrap(), anonymous);
    // The binding delivers the `@context` of a body, so the clone does not keep it.
    assert_eq!(
        SnapshotClone::parse(r#"{"@context":"https://example.org/context.jsonld","type":"Snapshot"}"#).unwrap(),
        anonymous
    );
}

#[test]
fn a_clone_body_refuses_what_a_clone_cannot_carry() {
    assert!(missing(&refused(SnapshotClone::parse(r#"{"id":"urn:ngsi-ld:Snapshot:2"}"#)), "type"));
    for member in ["snapshotQueries", "snapshotTemporalQueries", "snapshotStatus", "createdAt", "unknown"] {
        let body = format!(r#"{{"type":"Snapshot","{member}":[]}}"#);
        assert!(unexpected(&refused(SnapshotClone::parse(&body)), member), "{member}");
    }
    assert!(matches!(
        refused(SnapshotClone::parse(r#"{"type":"Subscription"}"#)).kind,
        ReadErrorKind::WrongType { .. }
    ));
    assert!(matches!(
        refused(SnapshotClone::parse(r#"{"type":"Snapshot","snapshotPriority":11}"#)).kind,
        ReadErrorKind::Invalid { .. }
    ));
    assert!(SnapshotClone::parse(r#"{"type":"Snapshot","id":"not an id"}"#).is_err());
}

#[test]
fn a_fragment_body_reads_back_as_written_and_may_leave_out_its_type_and_name_its_target() {
    let fragment = SnapshotFragment::builder()
        .lifetime("PT2H".parse().unwrap())
        .priority(4)
        .delete(SnapshotMember::Endpoint)
        .delete(SnapshotMember::ReceiverInfo)
        .build()
        .unwrap();
    assert_eq!(SnapshotFragment::parse(&fragment.render().to_string()).unwrap(), fragment);
    let typeless = r#"{"snapshotLifetime":"PT2H","snapshotPriority":4,"endpoint":"urn:ngsi-ld:null","receiverInfo":"urn:ngsi-ld:null"}"#;
    assert_eq!(SnapshotFragment::parse(typeless).unwrap(), fragment);
    let named = r#"{"id":"urn:ngsi-ld:Snapshot:1","type":"Snapshot","@context":"https://example.org/context.jsonld","snapshotLifetime":"PT2H","snapshotPriority":4,"endpoint":"urn:ngsi-ld:null","receiverInfo":"urn:ngsi-ld:null"}"#;
    assert_eq!(SnapshotFragment::parse(named).unwrap(), fragment);
    assert_eq!(SnapshotFragment::parse("{}").unwrap(), SnapshotFragment::builder().build().unwrap());
}

#[test]
fn a_fragment_body_refuses_what_a_client_cannot_change() {
    for member in ["snapshotQueries", "snapshotTemporalQueries", "snapshotStatus", "expiresAt", "unknown"] {
        let body = format!(r#"{{"type":"Snapshot","{member}":"urn:ngsi-ld:null"}}"#);
        assert!(unexpected(&refused(SnapshotFragment::parse(&body)), member), "{member}");
    }
    assert!(matches!(
        refused(SnapshotFragment::parse(r#"{"type":"Snapshot","snapshotPriority":0}"#)).kind,
        ReadErrorKind::Invalid { .. }
    ));
    assert!(matches!(
        refused(SnapshotFragment::parse(r#"{"type":"ContextSourceRegistration"}"#)).kind,
        ReadErrorKind::WrongType { .. }
    ));
    assert!(matches!(
        refused(SnapshotFragment::parse(
            r#"{"endpoint":"urn:ngsi-ld:null","endpoint":"http://my.endpoint.org/"}"#
        ))
        .kind,
        ReadErrorKind::Member(MemberError::DuplicateMember { .. })
    ));
    assert!(SnapshotFragment::parse(r#"{"endpoint":"not a url"}"#).is_err());
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
    members_round_trip!(SnapshotMember, QueryMember, AggregationMember);
    assert_eq!(SnapshotField::from(SnapshotMember::Endpoint).to_string(), "endpoint");
    assert_eq!(SnapshotField::from(QueryMember::JoinLevel).to_string(), "joinLevel");
}

#[test]
fn a_priority_out_of_range_and_an_endpoint_that_is_no_url_are_refused() {
    let error = SnapshotClone::builder().priority(11).endpoint("x").build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [
                Problem::Priority { .. },
                Problem::Member(MemberProblem::InvalidUrl {
                    member: SnapshotField::Snapshot(SnapshotMember::Endpoint),
                    ..
                })
            ]
        ),
        "{error:?}"
    );
}
