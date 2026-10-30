//! The `instanceId` a client may name for an instance of the temporal evolution it sends to Create or Update
//! Temporal Evolution or Add Attributes (clauses 4.5.7, 5.6.11.3 and 5.6.12.3, Tables 5.2.5-2 and 5.2.6-2), which an
//! entity of the Core API never carries, and the record of a deleted instance (clauses 4.5.7 and 4.5.8): the NGSI-LD
//! Null with its `deletedAt`.

use iri_rs::iri;
use ngsi_ld::model::{
    AttributeKind,
    AttributeName,
    AttributesError,
    DateTime,
    DocumentErrorKind,
    Entity,
    Input,
    InstanceId,
    Lossless,
    MemberError,
    NullMember,
    ProblemKind,
    Property,
    Scope,
    evolution::{DeletedInstance, ScopeInstance, TemporalAttributes, TemporalEntity},
};
use strum::IntoEnumIterator;

/// A time.
fn at(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// An instance id.
fn instance(text: &str) -> InstanceId {
    text.parse().unwrap()
}

#[test]
fn a_temporal_request_names_the_instance_ids_it_chooses_and_reads_them_back() {
    let entity = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .identified(
            "speed",
            instance("urn:ngsi-ld:attribute:instance:1"),
            Property::builder(120).observed_at(at("2018-08-01T12:03:00Z")).property("accuracy", 1),
        )
        .property("speed", Property::builder(80).observed_at(at("2018-08-01T12:05:00Z")))
        .build()
        .unwrap();
    assert_eq!(
        entity.render(Lossless::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:attribute:instance:1","accuracy":1},{"value":80,"observedAt":"2018-08-01T12:05:00Z"}]}"#
    );
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = entity.render(form).to_string();
        assert_eq!(TemporalEntity::<Input>::parse(&text, form).unwrap(), entity, "{text}");
    }
    let speed: AttributeName = "speed".parse().unwrap();
    let ids: Vec<Option<String>> = entity.attribute(&speed).unwrap().instance_ids().map(|id| id.map(ToString::to_string)).collect();
    assert_eq!(ids, [Some("urn:ngsi-ld:attribute:instance:1".to_owned()), None]);
}

#[test]
fn the_attributes_added_to_a_temporal_evolution_may_name_their_instance_ids() {
    let mut attributes = TemporalAttributes::<Input>::new();
    attributes
        .insert_identified(
            "speed".parse().unwrap(),
            instance("urn:ngsi-ld:attribute:instance:7"),
            Property::builder(42).observed_at(at("2021-01-01T00:00:00Z")).build().unwrap().into(),
        )
        .unwrap();
    assert_eq!(
        attributes.render(Lossless::Normalized).to_string(),
        r#"{"speed":[{"type":"Property","value":42,"observedAt":"2021-01-01T00:00:00Z","instanceId":"urn:ngsi-ld:attribute:instance:7"}]}"#
    );
}

#[test]
fn an_entity_of_the_core_api_refuses_an_instance_id() {
    let error = Entity::<Input>::parse(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","value":80,"instanceId":"urn:ngsi-ld:attribute:instance:1"}}"#,
        Lossless::Normalized,
    )
    .unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "instanceId"));
}

#[test]
fn a_temporal_request_refuses_an_instance_id_given_twice_and_one_that_is_not_a_uri() {
    let read = |instance: &str| {
        TemporalEntity::<Input>::parse(
            &format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{instance}]}}"#),
            Lossless::Normalized,
        )
    };
    assert!(read(r#"{"type":"Property","value":1,"instanceId":"urn:ngsi-ld:i:1","instanceId":"urn:ngsi-ld:i:2"}"#).is_err());
    assert!(read(r#"{"type":"Property","value":1,"instanceId":"not a uri"}"#).is_err());
}

#[test]
fn a_temporal_request_refuses_two_instances_naming_the_same_instance_id() {
    let first = || Property::builder(120).observed_at(at("2018-08-01T12:03:00Z"));
    let second = || Property::builder(80).observed_at(at("2018-08-01T12:05:00Z"));
    let error = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .identified("speed", instance("urn:ngsi-ld:attribute:instance:1"), first())
        .identified("speed", instance("urn:ngsi-ld:attribute:instance:1"), second())
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [problem] if matches!(problem.kind, ProblemKind::Attributes { source: AttributesError::InstanceIdRepeated, .. })
    ));

    let mut attributes = TemporalAttributes::<Input>::new();
    let speed: AttributeName = "speed".parse().unwrap();
    // Clone: the name is given for both instances.
    attributes
        .insert_identified(speed.clone(), instance("urn:ngsi-ld:attribute:instance:1"), first().build().unwrap().into())
        .unwrap();
    // Instances that name no `instanceId`, and other ones, are still added.
    attributes.insert(speed.clone(), second().build().unwrap().into()).unwrap();
    attributes.insert(speed.clone(), second().build().unwrap().into()).unwrap();
    attributes
        .insert_identified(speed.clone(), instance("urn:ngsi-ld:attribute:instance:2"), second().build().unwrap().into())
        .unwrap();
    assert_eq!(
        attributes.insert_identified(speed, instance("urn:ngsi-ld:attribute:instance:1"), second().build().unwrap().into()),
        Err(AttributesError::InstanceIdRepeated)
    );

    let read = TemporalEntity::<Input>::parse(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"type":"Property","value":1,"instanceId":"urn:ngsi-ld:i:1"},{"type":"Property","value":2,"instanceId":"urn:ngsi-ld:i:1"}]}"#,
        Lossless::Normalized,
    );
    assert!(read.is_err());
}

/// A temporal entity of one vehicle holding `member`.
fn vehicle_with(member: &str) -> String {
    format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle",{member}}}"#)
}

#[test]
fn a_temporal_request_records_a_deleted_instance_beside_the_instances_with_a_value_and_reads_it_back() {
    let entity = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(at("2018-08-01T12:03:00Z")))
        .deleted(
            "speed",
            DeletedInstance::builder(AttributeKind::Property, at("2018-08-01T12:07:00Z"))
                .observed_at(at("2018-08-01T12:07:00Z"))
                .instance_id(instance("urn:ngsi-ld:attribute:instance:3"))
                .build(),
        )
        .build()
        .unwrap();
    assert_eq!(
        entity.render(Lossless::Normalized).to_string(),
        vehicle_with(concat!(
            r#""speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"},"#,
            r#"{"type":"Property","value":"urn:ngsi-ld:null","observedAt":"2018-08-01T12:07:00Z","#,
            r#""deletedAt":"2018-08-01T12:07:00Z","instanceId":"urn:ngsi-ld:attribute:instance:3"}]"#
        ))
    );
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = entity.render(form).to_string();
        assert_eq!(TemporalEntity::<Input>::parse(&text, form).unwrap(), entity, "{text}");
    }
    let speed: AttributeName = "speed".parse().unwrap();
    let evolution = entity.attribute(&speed).unwrap();
    assert_eq!(evolution.deleted_ats().collect::<Vec<_>>(), [None, Some(&at("2018-08-01T12:07:00Z"))]);
    assert_eq!(evolution.kind(), Some(AttributeKind::Property));
}

#[test]
fn a_deleted_instance_of_every_kind_is_written_with_its_null_and_read_back() {
    for kind in AttributeKind::iter() {
        let entity = TemporalEntity::builder()
            .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
            .entity_type("Vehicle")
            .deleted(
                "gone",
                DeletedInstance::builder(kind, at("2018-08-01T12:07:00Z"))
                    .dataset_id(iri!("urn:ngsi-ld:Dataset:a"))
                    .build(),
            )
            .build()
            .unwrap();
        let text = entity.render(Lossless::Normalized).to_string();
        assert!(text.contains(r#""deletedAt":"2018-08-01T12:07:00Z""#), "{kind}: {text}");
        assert!(text.contains(r#""datasetId":"urn:ngsi-ld:Dataset:a""#), "{kind}: {text}");
        assert_eq!(TemporalEntity::<Input>::parse(&text, Lossless::Normalized).unwrap(), entity, "{kind}: {text}");
    }
}

#[test]
fn the_attributes_added_to_a_temporal_evolution_may_record_a_deleted_instance() {
    let mut attributes = TemporalAttributes::<Input>::new();
    let humidity: AttributeName = "humidity".parse().unwrap();
    attributes
        .insert_deleted(
            humidity,
            DeletedInstance::builder(AttributeKind::Property, at("2026-10-19T11:00:00Z"))
                .observed_at(at("2026-10-19T11:00:00Z"))
                .build(),
        )
        .unwrap();
    assert_eq!(
        attributes.render(Lossless::Normalized).to_string(),
        concat!(
            r#"{"humidity":[{"type":"Property","value":"urn:ngsi-ld:null","#,
            r#""observedAt":"2026-10-19T11:00:00Z","deletedAt":"2026-10-19T11:00:00Z"}]}"#
        )
    );
}

#[test]
fn a_temporal_request_reads_a_deleted_instance_in_either_form() {
    let normalized = TemporalEntity::<Input>::parse(
        &vehicle_with(concat!(
            r#""speed":[{"type":"Property","value":1,"observedAt":"2024-01-01T00:00:00Z"},"#,
            r#"{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","observedAt":"2024-01-02T00:00:00Z"}]"#
        )),
        Lossless::Normalized,
    )
    .unwrap();
    let concise = TemporalEntity::<Input>::parse(
        &vehicle_with(concat!(
            r#""speed":[{"value":1,"observedAt":"2024-01-01T00:00:00Z"},"#,
            r#"{"value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","observedAt":"2024-01-02T00:00:00Z"}]"#
        )),
        Lossless::Concise,
    )
    .unwrap();
    assert_eq!(normalized, concise);
    let speed: AttributeName = "speed".parse().unwrap();
    assert_eq!(
        normalized.attribute(&speed).unwrap().deleted_ats().collect::<Vec<_>>(),
        [None, Some(&at("2024-01-02T00:00:00Z"))]
    );
}

#[test]
fn a_null_with_no_deleted_at_and_a_deleted_at_beside_a_value_are_still_refused() {
    let read = |instance: &str| TemporalEntity::<Input>::parse(&vehicle_with(&format!(r#""speed":[{instance}]"#)), Lossless::Normalized).unwrap_err();
    let bare_null = read(r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#);
    assert!(matches!(bare_null.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }));
    let beside_a_value = read(r#"{"type":"Property","value":1,"deletedAt":"2024-01-02T00:00:00Z"}"#);
    assert!(matches!(beside_a_value.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "deletedAt"));
    let created = read(r#"{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","createdAt":"2024-01-02T00:00:00Z"}"#);
    assert!(matches!(created.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "createdAt"));
    let twice = read(r#"{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","deletedAt":"2024-01-03T00:00:00Z"}"#);
    assert!(matches!(twice.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { .. })));
    let not_a_time = read(r#"{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"yesterday"}"#);
    assert_eq!(not_a_time.path.to_string(), "speed");
    let nested =
        read(r#"{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","accuracy":{"type":"Property","value":"urn:ngsi-ld:null"}}"#);
    assert!(matches!(nested.kind, DocumentErrorKind::NgsiLdNull { .. }));
}

#[test]
fn a_temporal_request_records_the_removal_of_a_scope_with_its_deletion_time_and_reads_it_back() {
    let entity = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .scope(ScopeInstance::new("/Madrid/Centro".parse::<Scope>().unwrap()).with_observed_at(at("2018-08-01T11:00:00Z")))
        .scope(ScopeInstance::removed(at("2018-08-01T12:00:00Z")).with_observed_at(at("2018-08-01T12:00:00Z")))
        .build()
        .unwrap();
    assert_eq!(
        entity.render(Lossless::Normalized).to_string(),
        vehicle_with(concat!(
            r#""scope":[{"type":"Property","value":"/Madrid/Centro","observedAt":"2018-08-01T11:00:00Z"},"#,
            r#"{"type":"Property","value":"urn:ngsi-ld:null","observedAt":"2018-08-01T12:00:00Z","deletedAt":"2018-08-01T12:00:00Z"}]"#
        ))
    );
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = entity.render(form).to_string();
        assert_eq!(TemporalEntity::<Input>::parse(&text, form).unwrap(), entity, "{text}");
    }
    assert_eq!(entity.scope[1].deleted_at(), Some(&at("2018-08-01T12:00:00Z")));
    assert!(entity.scope[0].deleted_at().is_none());
}

#[test]
fn a_removed_scope_with_no_deletion_time_and_a_deletion_time_beside_scopes_are_refused_in_a_request() {
    let read = |instance: &str| TemporalEntity::<Input>::parse(&vehicle_with(&format!(r#""scope":[{instance}]"#)), Lossless::Normalized).unwrap_err();
    let bare_null = read(r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#);
    assert!(matches!(bare_null.kind, DocumentErrorKind::NgsiLdNull { .. }));
    let beside_scopes = read(r#"{"type":"Property","value":"/Madrid","deletedAt":"2024-01-02T00:00:00Z"}"#);
    assert!(matches!(beside_scopes.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "deletedAt"));
}

#[test]
fn the_system_times_of_a_removed_scope_stay_out_of_a_request() {
    let read = |instance: &str| TemporalEntity::<Input>::parse(&vehicle_with(&format!(r#""scope":[{instance}]"#)), Lossless::Normalized).unwrap_err();
    for member in ["createdAt", "modifiedAt"] {
        let error = read(&format!(
            r#"{{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","{member}":"2024-01-02T00:00:00Z"}}"#
        ));
        assert!(
            matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member: ref found }) if found == member),
            "{member}"
        );
    }
    let twice = read(r#"{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z","deletedAt":"2024-01-03T00:00:00Z"}"#);
    assert!(matches!(twice.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { .. })));
}

#[test]
fn a_recorded_deletion_is_held_to_the_kind_and_the_instance_id_rules_of_its_evolution() {
    let kind_change = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(120))
        .deleted(
            "speed",
            DeletedInstance::builder(AttributeKind::Relationship, at("2018-08-01T12:07:00Z")).build(),
        )
        .build()
        .unwrap_err();
    assert!(matches!(
        kind_change.problems(),
        [problem] if matches!(problem.kind, ProblemKind::Attributes { source: AttributesError::KindChange { .. }, .. })
    ));
    let repeated = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .identified("speed", instance("urn:ngsi-ld:attribute:instance:1"), Property::builder(120))
        .deleted(
            "speed",
            DeletedInstance::builder(AttributeKind::Property, at("2018-08-01T12:07:00Z"))
                .instance_id(instance("urn:ngsi-ld:attribute:instance:1"))
                .build(),
        )
        .build()
        .unwrap_err();
    assert!(matches!(
        repeated.problems(),
        [problem] if matches!(problem.kind, ProblemKind::Attributes { source: AttributesError::InstanceIdRepeated, .. })
    ));
    let twice = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .deleted("speed", DeletedInstance::builder(AttributeKind::Property, at("2018-08-01T12:07:00Z")).build())
        .deleted("speed", DeletedInstance::builder(AttributeKind::Property, at("2018-08-01T12:09:00Z")).build())
        .build();
    assert!(twice.is_ok());
}
