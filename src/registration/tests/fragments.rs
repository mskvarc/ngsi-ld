//! Registration fragments.

use crate::{
    model::{MemberChange, MemberProblem},
    registration::{
        ContextSourceInfoKey,
        EntityInfoMember,
        ManagementMember,
        OperationGroup,
        Problem,
        RegistrationField,
        RegistrationFragment,
        RegistrationInfo,
        RegistrationInfoMember,
        RegistrationMember,
        RegistrationMode,
        RegistrationTypeMember,
        tests::{problems, vehicle_speed},
    },
};
use pretty_assertions::assert_eq;
use strum::IntoEnumIterator;

#[test]
fn a_fragment_writes_its_members_in_canonical_order_and_deletes_with_the_null() {
    let fragment = RegistrationFragment::builder()
        .refresh_rate("PT5M".parse().unwrap())
        .information([vehicle_speed()])
        .description("Vehicles")
        .endpoint("http://my.csource.org:1027/")
        .mode(RegistrationMode::Inclusive)
        .delete(RegistrationMember::Location)
        .delete(RegistrationMember::RegistrationName)
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        concat!(
            r#"{"type":"ContextSourceRegistration","registrationName":"urn:ngsi-ld:null","description":"Vehicles","#,
            r#""information":[{"entities":[{"type":"Vehicle"}],"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1027/","#,
            r#""mode":"inclusive","location":"urn:ngsi-ld:null","refreshRate":"PT5M"}"#
        )
    );
    assert!(fragment.deletes(RegistrationMember::Location));
    assert!(!fragment.deletes(RegistrationMember::Mode));
    assert_eq!(fragment.information().map(<[_]>::len), Some(1));
}

#[test]
fn a_fragment_can_leave_its_type_out_and_writes_it_otherwise() {
    let written = RegistrationFragment::builder().description("Vehicles").build().unwrap();
    assert_eq!(written.type_member(), RegistrationTypeMember::Written);
    assert_eq!(written.render().to_string(), r#"{"type":"ContextSourceRegistration","description":"Vehicles"}"#);
    let omitted = RegistrationFragment::builder()
        .type_member(RegistrationTypeMember::Omitted)
        .description("Vehicles")
        .build()
        .unwrap();
    assert_eq!(omitted.type_member(), RegistrationTypeMember::Omitted);
    assert_eq!(omitted.render().to_string(), r#"{"description":"Vehicles"}"#);
}

#[test]
fn a_fragment_refuses_what_no_registration_can_hold() {
    let both = RegistrationFragment::builder()
        .mode(RegistrationMode::Redirect)
        .delete(RegistrationMember::Mode)
        .build();
    assert_eq!(
        problems(both),
        [Problem::Member(MemberProblem::SetAndDeleted {
            member: RegistrationField::Optional(RegistrationMember::Mode)
        })]
    );
    let none: [RegistrationInfo; 0] = [];
    assert_eq!(
        problems(RegistrationFragment::builder().information(none).build()),
        [Problem::Member(MemberProblem::Empty {
            member: RegistrationField::Information
        })]
    );
    let endpoint = problems(RegistrationFragment::builder().endpoint("not a url").build());
    assert!(
        matches!(
            endpoint.as_slice(),
            [Problem::Member(MemberProblem::InvalidUrl {
                member: RegistrationField::Endpoint,
                ..
            })]
        ),
        "{endpoint:?}"
    );
    let auxiliary = RegistrationFragment::builder()
        .mode(RegistrationMode::Auxiliary)
        .operations([OperationGroup::FederationOps])
        .build();
    assert_eq!(problems(auxiliary), [Problem::AuxiliaryOperations]);
    let exclusive = RegistrationFragment::builder()
        .mode(RegistrationMode::Exclusive)
        .information([vehicle_speed()])
        .build();
    assert_eq!(problems(exclusive), [Problem::ExclusiveWithoutEntityIdAndAttributes]);
    assert_eq!(
        problems(RegistrationFragment::builder().description("").build()),
        [Problem::Member(MemberProblem::Empty {
            member: RegistrationField::Optional(RegistrationMember::Description)
        })]
    );
    assert!(RegistrationFragment::builder().mode(RegistrationMode::Auxiliary).build().is_ok());
}

#[test]
fn rf_01_a_fragment_reads_back_the_members_it_sets_and_deletes() {
    let fragment = RegistrationFragment::builder()
        .mode(RegistrationMode::Redirect)
        .delete(RegistrationMember::Location)
        .build()
        .unwrap();
    assert_eq!(fragment.mode(), Some(MemberChange::Set(RegistrationMode::Redirect)));
    assert_eq!(fragment.location(), Some(MemberChange::Delete));
    assert_eq!(fragment.description(), None);
    assert!(fragment.deletes(RegistrationMember::Location));
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"ContextSourceRegistration","mode":"redirect","location":"urn:ngsi-ld:null"}"#
    );
}

#[test]
fn me_01_every_registration_member_parses_back_from_its_name() {
    for member in RegistrationMember::iter() {
        assert_eq!(member.to_string().parse::<RegistrationMember>(), Ok(member));
    }
}

#[test]
fn every_member_a_fragment_deletes_is_reported_as_deleted() {
    for member in RegistrationMember::iter() {
        assert!(RegistrationFragment::builder().delete(member).build().unwrap().deletes(member), "{member}");
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
    members_round_trip!(
        RegistrationMember,
        RegistrationInfoMember,
        EntityInfoMember,
        ManagementMember,
        ContextSourceInfoKey
    );
    assert_eq!(RegistrationField::Information.to_string(), "information");
    assert_eq!(RegistrationField::Endpoint.to_string(), "endpoint");
    assert_eq!(RegistrationField::from(ManagementMember::Timeout).to_string(), "timeout");
    assert_eq!(RegistrationField::from(RegistrationMember::Description).to_string(), "description");
}
