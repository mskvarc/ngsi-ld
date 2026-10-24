//! The bodies of Clone Snapshot and Update Snapshot Status.

use crate::{
    model::MemberProblem,
    snapshot::{AggregationMember, Problem, QueryMember, SnapshotClone, SnapshotField, SnapshotFragment, SnapshotMember},
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
