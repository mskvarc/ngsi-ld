use crate::model::{
    AttributeFragment,
    AttributeKind,
    AttributeMembers,
    Change,
    DateTime,
    EntityTypeName,
    InstanceDeletion,
    Lossless,
    MemberChange,
    MemberOfOtherKind,
    PatchMember,
    ProblemKind,
    VocabTerm,
};
use cefact_units::UnitCode;
use pretty_assertions::assert_eq;

fn observed() -> DateTime {
    "2022-03-14T12:51:02Z".parse().unwrap()
}

fn sensor() -> EntityTypeName {
    "Sensor".parse().unwrap()
}

fn render(members: AttributeMembers, form: Lossless) -> String {
    AttributeFragment::from(members).render(form).to_string()
}

#[test]
fn a_fragment_with_no_member_is_the_empty_object_in_both_forms() {
    for form in [Lossless::Normalized, Lossless::Concise] {
        assert_eq!(render(AttributeMembers::builder().build().unwrap(), form), "{}");
    }
}

#[test]
fn a_fragment_without_a_payload_writes_the_members_it_names_and_its_sub_attributes() {
    let members = || AttributeMembers::builder().observed_at(observed()).property("accuracy", 3).build().unwrap();
    assert_eq!(render(members(), Lossless::Concise), r#"{"observedAt":"2022-03-14T12:51:02Z","accuracy":3}"#);
    assert_eq!(
        render(members(), Lossless::Normalized),
        r#"{"observedAt":"2022-03-14T12:51:02Z","accuracy":{"type":"Property","value":3}}"#
    );
}

#[test]
fn a_fragment_writes_its_type_only_when_it_names_one() {
    let property = AttributeMembers::builder()
        .kind(AttributeKind::Property)
        .unit_code(UnitCode::Cel)
        .build()
        .unwrap();
    assert_eq!(render(property, Lossless::Concise), r#"{"type":"Property","unitCode":"CEL"}"#);
    let relationship = AttributeMembers::builder()
        .kind(AttributeKind::Relationship)
        .object_type(sensor())
        .build()
        .unwrap();
    assert_eq!(render(relationship, Lossless::Normalized), r#"{"type":"Relationship","objectType":"Sensor"}"#);
}

#[test]
fn a_member_the_named_kind_has_not_is_refused_when_the_fragment_is_built() {
    let error = AttributeMembers::builder()
        .kind(AttributeKind::Relationship)
        .unit_code(UnitCode::Cel)
        .value_type("Number".parse::<VocabTerm>().unwrap())
        .build()
        .unwrap_err();
    let problems: Vec<ProblemKind> = error.problems().iter().map(|problem| problem.kind.clone()).collect();
    assert_eq!(
        problems,
        [
            ProblemKind::MemberOfOtherKind {
                member: PatchMember::UnitCode,
                kind: AttributeKind::Relationship
            },
            ProblemKind::MemberOfOtherKind {
                member: PatchMember::ValueType,
                kind: AttributeKind::Relationship
            }
        ]
    );
    assert!(AttributeMembers::builder().kind(AttributeKind::Property).object_type(sensor()).build().is_err());
}

#[test]
fn a_fragment_without_a_kind_leaves_the_check_of_its_members_to_the_target() {
    let fragment = AttributeMembers::builder().unit_code(UnitCode::Cel).build().unwrap();
    assert_eq!(fragment.kind(), None);
    assert_eq!(fragment.unit_code(), Some(MemberChange::Set(&UnitCode::Cel)));
}

#[test]
fn an_empty_object_type_is_refused() {
    assert!(AttributeMembers::builder().object_type(Vec::<EntityTypeName>::new()).build().is_err());
}

#[test]
fn a_deleted_member_is_written_as_the_null_and_a_member_of_another_kind_cannot_be_deleted() {
    let deleted = AttributeMembers::builder()
        .build()
        .unwrap()
        .delete_member(PatchMember::UnitCode)
        .unwrap()
        .delete_member(PatchMember::ExpiresAt)
        .unwrap();
    assert_eq!(deleted.unit_code(), Some(MemberChange::Delete));
    assert_eq!(
        render(deleted, Lossless::Concise),
        r#"{"unitCode":"urn:ngsi-ld:null","expiresAt":"urn:ngsi-ld:null"}"#
    );
    let sensor = AttributeMembers::builder().kind(AttributeKind::Relationship).build().unwrap();
    assert_eq!(
        sensor.delete_member(PatchMember::UnitCode),
        Err(MemberOfOtherKind {
            member: PatchMember::UnitCode,
            kind: AttributeKind::Relationship
        })
    );
}

#[test]
fn a_deleted_sub_attribute_is_written_with_the_null_in_the_shape_of_its_kind() {
    let fragment = AttributeMembers::builder()
        .build()
        .unwrap()
        .delete_sub_attribute("accuracy", InstanceDeletion::new(AttributeKind::Property))
        .unwrap();
    assert_eq!(
        render(fragment, Lossless::Normalized),
        r#"{"accuracy":{"type":"Property","value":"urn:ngsi-ld:null"}}"#
    );
    let twice = AttributeMembers::builder()
        .build()
        .unwrap()
        .delete_sub_attribute("accuracy", InstanceDeletion::new(AttributeKind::Property))
        .unwrap()
        .delete_sub_attribute("accuracy", InstanceDeletion::new(AttributeKind::Property));
    assert!(twice.is_err());
    assert!(
        AttributeMembers::builder()
            .build()
            .unwrap()
            .delete_sub_attribute("not a name", InstanceDeletion::new(AttributeKind::Property))
            .is_err()
    );
}

#[test]
fn a_fragment_that_names_no_payload_reads_back_in_both_forms() {
    let members = || {
        AttributeMembers::builder()
            .kind(AttributeKind::Property)
            .observed_at(observed())
            .property("accuracy", 3)
            .build()
            .unwrap()
            .delete_member(PatchMember::UnitCode)
            .unwrap()
    };
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = render(members(), form);
        let read = Change::<AttributeFragment, InstanceDeletion>::parse(&text, form).unwrap();
        assert_eq!(read, Change::Set(members().into()), "{text}");
    }
}
