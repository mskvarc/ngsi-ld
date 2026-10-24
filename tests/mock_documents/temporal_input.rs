//! The `instanceId` a client may name for an instance of the temporal evolution it sends to Create or Update
//! Temporal Evolution or Add Attributes (clauses 4.5.7, 5.6.11.3 and 5.6.12.3, Tables 5.2.5-2 and 5.2.6-2), which an
//! entity of the Core API never carries.

use iri_rs::iri;
use ngsi_ld::model::{
    AttributeName,
    AttributesError,
    DateTime,
    DocumentErrorKind,
    Entity,
    Input,
    InstanceId,
    Lossless,
    MemberError,
    ProblemKind,
    Property,
    evolution::{TemporalAttributes, TemporalEntity},
};

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
