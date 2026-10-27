use crate::model::{
    EntityId,
    Input,
    InstanceId,
    Lossless,
    Output,
    Property,
    Relationship,
    Scope,
    evolution::{InstanceSpelling, ScopeInstance, TemporalEntity},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

const AT_1: &str = "2018-08-01T12:03:00Z";
const AT_2: &str = "2018-08-01T12:05:00Z";

/// `speed` once, at `AT_1`.
fn one_instance() -> TemporalEntity<Input> {
    TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(AT_1.parse().unwrap()))
        .build()
        .unwrap()
}

#[test]
fn a_single_instance_is_one_object_in_normalized_form() {
    assert_eq!(
        one_instance().render_spelled(Lossless::Normalized, InstanceSpelling::SingleWhenOne).to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""speed":{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}}"#
        )
    );
}

#[test]
fn a_single_instance_is_one_object_in_concise_form() {
    assert_eq!(
        one_instance().render_spelled(Lossless::Concise, InstanceSpelling::SingleWhenOne).to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""speed":{"value":120,"observedAt":"2018-08-01T12:03:00Z"}}"#
        )
    );
}

#[test]
fn the_array_spelling_and_render_keep_a_single_instance_in_an_array() {
    let entity = one_instance();
    let text = concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"}]}"#
    );
    assert_eq!(InstanceSpelling::default(), InstanceSpelling::Array);
    assert_eq!(entity.render(Lossless::Concise).to_string(), text);
    assert_eq!(entity.render_spelled(Lossless::Concise, InstanceSpelling::Array).to_string(), text);
}

#[test]
fn only_the_evolutions_of_one_instance_become_objects() {
    let entity = TemporalEntity::<Input>::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(AT_1.parse().unwrap()))
        .property("speed", Property::builder(80).observed_at(AT_2.parse().unwrap()))
        .property("brandName", Property::builder("Mercedes").observed_at(AT_1.parse().unwrap()))
        .build()
        .unwrap();
    assert_eq!(
        entity.render_spelled(Lossless::Concise, InstanceSpelling::SingleWhenOne).to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"},{"value":80,"observedAt":"2018-08-01T12:05:00Z"}],"#,
            r#""brandName":{"value":"Mercedes","observedAt":"2018-08-01T12:03:00Z"}}"#
        )
    );
}

#[test]
fn a_single_relationship_instance_is_one_object() {
    let entity = TemporalEntity::<Input>::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .relationship(
            "isParked",
            Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))),
        )
        .build()
        .unwrap();
    assert_eq!(
        entity.render_spelled(Lossless::Normalized, InstanceSpelling::SingleWhenOne).to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
            r#""isParked":{"type":"Relationship","object":"urn:ngsi-ld:OffStreetParking:Downtown1"}}"#
        )
    );
}

#[test]
fn an_instance_that_names_its_instance_id_keeps_it_as_an_object() {
    let entity = TemporalEntity::<Input>::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .identified(
            "speed",
            "urn:ngsi-ld:attribute:instance:1".parse::<InstanceId>().unwrap(),
            Property::builder(120).observed_at(AT_1.parse().unwrap()),
        )
        .build()
        .unwrap();
    let text = entity.render_spelled(Lossless::Concise, InstanceSpelling::SingleWhenOne).to_string();
    assert!(text.contains(r#""speed":{"#), "{text}");
    assert!(text.contains(r#""instanceId":"urn:ngsi-ld:attribute:instance:1""#), "{text}");
}

#[test]
fn a_scope_of_one_instance_is_one_object_and_of_two_an_array() {
    let scope = |text: &str| ScopeInstance::new(text.parse::<Scope>().unwrap()).with_observed_at(AT_1.parse().unwrap());
    let entity = |instances: &[&str]| {
        instances
            .iter()
            .fold(
                TemporalEntity::<Input>::builder().id(iri!("urn:ngsi-ld:Vehicle:B9211")).entity_type("Vehicle"),
                |builder, text| builder.scope(scope(text)),
            )
            .build()
            .unwrap()
    };
    let one = entity(&["/Madrid"]);
    assert_eq!(
        one.render_spelled(Lossless::Concise, InstanceSpelling::SingleWhenOne).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":{"value":"/Madrid","observedAt":"2018-08-01T12:03:00Z"}}"#
    );
    assert_eq!(
        one.render_spelled(Lossless::Concise, InstanceSpelling::Array).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":[{"value":"/Madrid","observedAt":"2018-08-01T12:03:00Z"}]}"#
    );
    let two = entity(&["/Madrid", "/Lisbon"]);
    assert_eq!(
        two.render_spelled(Lossless::Concise, InstanceSpelling::SingleWhenOne).to_string(),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":["#,
            r#"{"value":"/Madrid","observedAt":"2018-08-01T12:03:00Z"},"#,
            r#"{"value":"/Lisbon","observedAt":"2018-08-01T12:03:00Z"}]}"#
        )
    );
}

#[test]
fn the_single_spelling_is_read_back_as_the_same_evolution() {
    for form in [Lossless::Normalized, Lossless::Concise] {
        let entity = one_instance();
        let single = entity.render_spelled(form, InstanceSpelling::SingleWhenOne).to_string();
        assert_eq!(TemporalEntity::<Input>::parse(&single, form).unwrap(), entity, "{form:?}");
        let read = TemporalEntity::<Output>::parse(&single, form).unwrap();
        assert_eq!(read.render(form).to_string(), entity.render(form).to_string(), "{form:?}");
    }
}
