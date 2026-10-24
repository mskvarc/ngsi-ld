//! The fragments a broker forwards to a context source for Update Attributes, Append Attributes, Merge Entity and
//! Partial Attribute Update (clauses 5.4, 5.6.2 to 5.6.4 and 5.6.17), read in typed form, with every deletion the
//! NGSI-LD Null writes (clauses 5.5.8 and 5.5.12) and member order kept.

use iri_rs::iri;
use ngsi_ld::model::{
    Attribute,
    AttributeKind,
    AttributeName,
    Change,
    Complete,
    Deletion,
    DocumentErrorKind,
    EntityFragment,
    InstanceDeletion,
    Lossless,
    MemberError,
    NullMember,
    Patch,
    Projected,
    Property,
    ScopeChange,
};
use strum::IntoEnumIterator;

/// An attribute name.
fn name(text: &str) -> AttributeName {
    text.parse().unwrap()
}

/// The first change the fragment makes to `attribute`.
fn first_change<'a>(fragment: &'a EntityFragment, attribute: &str) -> &'a Change<Attribute<Patch>> {
    fragment.attributes.get(&name(attribute)).unwrap().first()
}

#[test]
fn every_change_a_fragment_builder_makes_reads_back_in_both_forms() {
    let mut builder = EntityFragment::builder()
        .scope(ScopeChange::Delete)
        .property("speed", Property::builder(0).property("source", "radar"))
        .delete_sub_attribute("speed", "accuracy")
        .delete("isParked")
        .delete_instance("brand", AttributeKind::Property, iri!("urn:ngsi-ld:Dataset:a"))
        .property("brand", Property::builder("Mercedes").dataset_id(iri!("urn:ngsi-ld:Dataset:b")));
    for kind in AttributeKind::iter() {
        builder = builder.delete_kind(format!("gone{kind}").as_str(), kind);
    }
    let fragment = builder.build().unwrap();
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = fragment.render(form).to_string();
        let read = EntityFragment::<Projected>::parse(&text, form).unwrap();
        assert_eq!(read, fragment, "{text}");
        assert_eq!(read.render(form).to_string(), text);
    }
    assert!(matches!(first_change(&fragment, "isParked"), Change::Delete(Deletion::Member)));
}

#[test]
fn a_fragment_that_names_its_entity_reads_back_and_requires_id_and_type() {
    let fragment = EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
        .property("speed", 0)
        .delete("isParked")
        .build()
        .unwrap();
    let text = fragment.render(Lossless::Normalized).to_string();
    assert_eq!(EntityFragment::<Complete>::parse(&text, Lossless::Normalized).unwrap(), fragment);
    let error = EntityFragment::<Complete>::parse(r#"{"type":"Vehicle","speed":0}"#, Lossless::Concise).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })));
}

#[test]
fn a_forwarded_merge_keeps_a_null_nested_in_a_value_and_the_member_order() {
    let text = r#"{"address":{"type":"Property","value":{"street":"urn:ngsi-ld:null","city":"Berlin"}},"speed":{"type":"Property","value":5},"isParked":"urn:ngsi-ld:null"}"#;
    let fragment = EntityFragment::<Projected>::parse(text, Lossless::Normalized).unwrap();
    assert!(first_change(&fragment, "address").is_set());
    assert_eq!(
        fragment.attributes.names().map(AsRef::as_ref).collect::<Vec<&str>>(),
        ["address", "speed", "isParked"]
    );
    assert_eq!(fragment.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn a_concise_deletion_of_an_instance_reads_as_a_deletion_of_its_kind() {
    let fragment = EntityFragment::<Projected>::parse(
        r#"{"street":{"languageMap":{"@none":"urn:ngsi-ld:null"}},"owner":{"object":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:Dataset:a"}}"#,
        Lossless::Concise,
    )
    .unwrap();
    assert!(matches!(
        first_change(&fragment, "street"),
        Change::Delete(Deletion::Instance(InstanceDeletion {
            kind: AttributeKind::LanguageProperty,
            dataset_id: None
        }))
    ));
    assert!(matches!(
        first_change(&fragment, "owner"),
        Change::Delete(Deletion::Instance(InstanceDeletion {
            kind: AttributeKind::Relationship,
            dataset_id: Some(_)
        }))
    ));
}

#[test]
fn a_forwarded_fragment_keeps_its_context_and_writes_it_last() {
    let text = r#"{"speed":{"type":"Property","value":5},"@context":"https://example.org/context.jsonld"}"#;
    let fragment = EntityFragment::<Projected>::parse(text, Lossless::Normalized).unwrap();
    assert!(fragment.context.is_some());
    assert_eq!(fragment.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn a_fragment_refuses_system_and_output_members_and_a_null_beside_other_members() {
    let read = |text: &str| EntityFragment::<Projected>::parse(text, Lossless::Normalized).unwrap_err();
    let created = read(r#"{"createdAt":"2024-05-01T10:00:00Z"}"#);
    assert!(matches!(created.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "createdAt"));
    let instance_id = read(r#"{"speed":{"type":"Property","value":5,"instanceId":"urn:ngsi-ld:attribute:instance:1"}}"#);
    assert!(matches!(instance_id.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "instanceId"));
    let previous = read(r#"{"speed":{"type":"Property","value":5,"previousValue":4}}"#);
    assert!(matches!(previous.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "previousValue"));
    let observed_null = read(r#"{"speed":{"type":"Property","value":"urn:ngsi-ld:null","observedAt":"2024-05-01T10:00:00Z"}}"#);
    assert_eq!(observed_null.path.to_string(), "speed");
    assert!(matches!(observed_null.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }));
}

#[test]
fn an_attribute_fragment_reads_back_with_its_sub_attribute_deletions() {
    let fragment = EntityFragment::builder()
        .property("speed", Property::builder(0).property("source", "radar"))
        .delete_sub_attribute("speed", "accuracy")
        .build()
        .unwrap();
    let Change::Set(attribute) = first_change(&fragment, "speed") else {
        panic!("speed is set");
    };
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = attribute.render(form).to_string();
        let read = Change::<Attribute<Patch>, InstanceDeletion>::parse(&text, form).unwrap();
        assert!(matches!(&read, Change::Set(read) if read == attribute), "{text}");
    }
    let whole = Change::<Attribute<Patch>, InstanceDeletion>::parse(r#""urn:ngsi-ld:null""#, Lossless::Normalized).unwrap_err();
    assert!(matches!(whole.kind, DocumentErrorKind::NgsiLdNull { .. }));
}
