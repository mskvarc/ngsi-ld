//! Projected entities and the inline linked entities of a response (clauses 4.21 and 4.5.23.2), built in typed form as
//! a mock broker or context source answers: build, write, read back, compare.

use iri_rs::iri;
use ngsi_ld::model::{
    Attribute,
    AttributeKind,
    DateTime,
    Entity,
    EntityId,
    ListRelationship,
    Lossless,
    OneOrMany,
    Output,
    ProblemKind,
    Projected,
    Property,
    Relationship,
    Representation,
};

/// A time.
fn at(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// Writes `entity` in both lossless forms and reads each back, expecting `entity` again.
fn assert_projected_round_trip(entity: &Entity<Output, Projected>) {
    for (representation, form) in [(Representation::Normalized, Lossless::Normalized), (Representation::Concise, Lossless::Concise)] {
        let text = entity.render(representation).to_string();
        assert_eq!(&Entity::<Output, Projected>::parse(&text, form).unwrap(), entity, "{text}");
    }
}

/// A parking a vehicle links to, built complete.
fn parking() -> Entity<Output> {
    Entity::output_builder()
        .id(iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
        .entity_type("OffStreetParking")
        .property("availableSpotNumber", 121)
        .build()
        .unwrap()
}

#[test]
fn a_projected_entity_without_id_and_type_round_trips() {
    let entity = Entity::projected_output_builder()
        .property("brandName", "Mercedes")
        .output_attribute("speed", Property::output_builder(80).modified_at(at("2024-05-01T10:00:00Z")))
        .build()
        .unwrap();
    assert_eq!(entity.id, None);
    assert_eq!(entity.entity_type, None);
    assert_projected_round_trip(&entity);
}

#[test]
fn a_projected_entity_keeps_the_identifying_member_it_is_given() {
    let only_type = Entity::projected_output_builder().entity_type("Vehicle").property("speed", 80).build().unwrap();
    assert_eq!(only_type.render(Representation::Concise).to_string(), r#"{"type":"Vehicle","speed":80}"#);
    assert_projected_round_trip(&only_type);
    let only_id = Entity::projected_output_builder().id(iri!("urn:ngsi-ld:Vehicle:A4567")).build().unwrap();
    assert_eq!(only_id.render(Representation::Concise).to_string(), r#"{"id":"urn:ngsi-ld:Vehicle:A4567"}"#);
    assert_projected_round_trip(&only_id);
}

#[test]
fn the_projected_builder_reports_an_invalid_id_and_type() {
    let error = Entity::projected_output_builder().id("not a uri").entity_type("1Vehicle").build().unwrap_err();
    assert!(matches!(
        error.problems(),
        [first, second] if matches!(first.kind, ProblemKind::Identifier { .. }) && matches!(second.kind, ProblemKind::Term { .. })
    ));
}

#[test]
fn a_complete_entity_converts_into_a_projected_one_holding_its_id_and_type() {
    let projected = Entity::<Output, Projected>::from(parking());
    assert_eq!(
        projected.id.as_ref().map(ToString::to_string).as_deref(),
        Some("urn:ngsi-ld:OffStreetParking:Downtown1")
    );
    assert_eq!(
        projected.render(Representation::Normalized).to_string(),
        parking().render(Representation::Normalized).to_string()
    );
}

#[test]
fn a_relationship_with_its_linked_entity_round_trips() {
    let vehicle = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .output_attribute(
            "isParked",
            Relationship::output_builder(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")).entity(parking()),
        )
        .build()
        .unwrap();
    let text = vehicle.render(Representation::Normalized).to_string();
    assert!(
        text.contains(r#""entity":{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"OffStreetParking","availableSpotNumber":"#),
        "{text}"
    );
    for (representation, form) in [(Representation::Normalized, Lossless::Normalized), (Representation::Concise, Lossless::Concise)] {
        let text = vehicle.render(representation).to_string();
        assert_eq!(Entity::<Output>::parse(&text, form).unwrap(), vehicle, "{text}");
    }
}

#[test]
fn a_relationship_with_several_objects_links_several_projected_entities() {
    let first = Entity::projected_output_builder().property("name", "first").build().unwrap();
    let second = Entity::projected_output_builder().property("name", "second").build().unwrap();
    let owners = Relationship::output_builder(vec![iri!("urn:ngsi-ld:Person:1").into(), iri!("urn:ngsi-ld:Person:2").into()])
        .entity(OneOrMany::try_from(vec![first, second]).unwrap())
        .build()
        .unwrap();
    assert_eq!(owners.output().entity.as_ref().map(|entities| entities.iter().count()), Some(2));
    let attribute = Attribute::from(owners);
    let text = attribute.render(Lossless::Normalized).to_string();
    assert_eq!(Attribute::<Output>::parse(&text, Lossless::Normalized).unwrap(), attribute);
}

#[test]
fn a_list_relationship_with_its_linked_entities_round_trips() {
    let passenger = Entity::projected_output_builder().entity_type("Person").build().unwrap();
    let vehicle = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .output_attribute(
            "parkedAt",
            ListRelationship::output_builder([iri!("urn:ngsi-ld:OffStreetParking:Downtown1")]).entity_list([parking()]),
        )
        .output_attribute(
            "passengers",
            ListRelationship::output_builder([iri!("urn:ngsi-ld:Person:Bob")]).entity_list([passenger]),
        )
        .build()
        .unwrap();
    for (representation, form) in [(Representation::Normalized, Lossless::Normalized), (Representation::Concise, Lossless::Concise)] {
        let text = vehicle.render(representation).to_string();
        assert_eq!(Entity::<Output>::parse(&text, form).unwrap(), vehicle, "{text}");
    }
}

#[test]
fn a_deleted_vocab_property_holds_the_null_term() {
    let deleted = Attribute::deleted(AttributeKind::VocabProperty).deleted_at(at("2024-01-02T00:00:00Z")).build();
    let Attribute::VocabProperty(property) = &deleted else {
        panic!("a VocabProperty");
    };
    assert_eq!(property.vocab().iter().map(ToString::to_string).collect::<Vec<_>>(), ["urn:ngsi-ld:null"]);
    assert_eq!(
        deleted.render(Lossless::Normalized).to_string(),
        r#"{"type":"VocabProperty","vocab":"urn:ngsi-ld:null","deletedAt":"2024-01-02T00:00:00Z"}"#
    );
    let read = Attribute::<Output>::parse(&deleted.render(Lossless::Concise).to_string(), Lossless::Concise).unwrap();
    assert_eq!(read, deleted);
}

#[test]
fn a_relationship_object_written_verbatim_keeps_its_scheme_and_host_case() {
    let entity = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .relationship("owner", EntityId::verbatim("HTTP://Example.ORG/x").unwrap())
        .build()
        .unwrap();
    assert_eq!(
        entity.render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","owner":{"type":"Relationship","object":"HTTP://Example.ORG/x"}}"#
    );
    assert_eq!(
        entity.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","owner":{"object":"HTTP://Example.ORG/x"}}"#
    );
}
