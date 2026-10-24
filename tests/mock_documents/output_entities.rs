//! Entities and attributes a broker or a context source returns, built in typed form (clause 5.2.2 and the "output
//! only members" tables of clause 5.2): build, write, read back, compare.

use iri_rs::iri;
use ngsi_ld::model::{
    Attribute,
    AttributeKind,
    AttributeName,
    DateTime,
    Entity,
    EntityId,
    GeoProperty,
    Instances,
    JsonProperty,
    LanguageKey,
    LanguageMap,
    LanguageProperty,
    ListProperty,
    ListRelationship,
    Lossless,
    Output,
    ProblemKind,
    Property,
    Relationship,
    Representation,
    VocabProperty,
    VocabTerm,
    geometry::{Geometry, Position, Shape},
};
use strum::IntoEnumIterator;

/// A time.
fn at(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// An attribute name.
fn name(text: &str) -> AttributeName {
    text.parse().unwrap()
}

/// Writes `entity` in both lossless forms and reads each back, expecting `entity` again.
fn assert_round_trip(entity: &Entity<Output>) {
    for (representation, form) in [(Representation::Normalized, Lossless::Normalized), (Representation::Concise, Lossless::Concise)] {
        let text = entity.render(representation).to_string();
        assert_eq!(&Entity::<Output>::parse(&text, form).unwrap(), entity, "{text}");
    }
}

/// An entity with an attribute of every kind, each with the output-only members of its kind.
fn every_kind() -> Entity<Output> {
    let created = at("2024-05-01T10:00:00Z");
    let modified = at("2024-05-02T10:00:00Z");
    Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .scope("/Madrid")
        .expires_at(at("2030-01-01T00:00:00Z"))
        .property("brandName", "Mercedes")
        .output_attribute(
            "speed",
            Property::output_builder(Property::builder(80).observed_at(created))
                .created_at(created)
                .modified_at(modified)
                .previous_value(70),
        )
        .output_attribute(
            "location",
            GeoProperty::output_builder(Shape::Point(Position::try_new(-8.5, 41.2).unwrap()))
                .previous_value(Geometry::from(Shape::Point(Position::try_new(-8.6, 41.1).unwrap()))),
        )
        .output_attribute(
            "street",
            LanguageProperty::output_builder([("fr", "Grand Place")]).previous_language_map(LanguageMap::try_from_pairs([("fr", "Petite Place")]).unwrap()),
        )
        .output_attribute(
            "category",
            VocabProperty::output_builder("commercial").previous_vocab("non-commercial".parse::<VocabTerm>().unwrap()),
        )
        .output_attribute("readings", ListProperty::output_builder([1, 2]).previous_value_list(vec![0.into()]))
        .output_attribute("config", JsonProperty::output_builder("{\"a\":1}").previous_json("{\"a\":0}".parse().unwrap()))
        .output_attribute(
            "isParked",
            Relationship::output_builder(iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
                .previous_object(EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Uptown"))),
        )
        .output_attribute(
            "passengers",
            ListRelationship::output_builder([iri!("urn:ngsi-ld:Person:Bob")]).previous_object_list(vec![EntityId::from(iri!("urn:ngsi-ld:Person:Alice"))]),
        )
        .created_at(created)
        .modified_at(modified)
        .build()
        .unwrap()
}

#[test]
fn an_output_entity_with_every_kind_and_its_output_members_round_trips() {
    assert_round_trip(&every_kind());
}

#[test]
fn an_output_entity_writes_its_system_members_after_the_attributes() {
    let entity = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", 80)
        .created_at(at("2024-05-01T10:00:00Z"))
        .deleted_at(at("2024-05-03T10:00:00Z"))
        .build()
        .unwrap();
    assert_eq!(
        entity.render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":80},"createdAt":"2024-05-01T10:00:00Z","deletedAt":"2024-05-03T10:00:00Z"}"#
    );
}

#[test]
fn the_lang_of_the_language_filter_round_trips() {
    let entity = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:Building:1"))
        .entity_type("Building")
        .output_attribute("street", Property::output_builder("Grand Place").lang("fr".parse::<LanguageKey>().unwrap()))
        .build()
        .unwrap();
    assert!(entity.render(Representation::Normalized).to_string().contains(r#""lang":"fr""#));
    assert_round_trip(&entity);
}

#[test]
fn a_deleted_attribute_of_every_kind_round_trips_with_its_deleted_at() {
    for kind in AttributeKind::iter() {
        let deleted = Attribute::deleted(kind)
            .deleted_at(at("2024-01-02T00:00:00Z"))
            .instance_id(iri!("urn:ngsi-ld:attribute:instance:1"))
            .build();
        assert_eq!(deleted.kind(), kind);
        assert!(deleted.into_input().is_err(), "{kind}: a request cannot hold the deletion");
        let entity = Entity::output_builder()
            .id(iri!("urn:ngsi-ld:T:1"))
            .entity_type("T")
            .output_attribute("gone", Attribute::deleted(kind).deleted_at(at("2024-01-02T00:00:00Z")))
            .build()
            .unwrap();
        assert_round_trip(&entity);
    }
}

#[test]
fn a_deleted_attribute_of_every_kind_with_nothing_more_to_report_is_the_bare_null() {
    for kind in AttributeKind::iter() {
        let entity = Entity::output_builder()
            .id(iri!("urn:ngsi-ld:T:1"))
            .entity_type("T")
            .output_attribute("gone", Attribute::deleted(kind))
            .build()
            .unwrap();
        for (representation, form) in [(Representation::Normalized, Lossless::Normalized), (Representation::Concise, Lossless::Concise)] {
            let text = entity.render(representation).to_string();
            assert_eq!(text, r#"{"id":"urn:ngsi-ld:T:1","type":"T","gone":"urn:ngsi-ld:null"}"#, "{kind}");
            // The bare Null names no kind; it reads back as a deleted Property and writes the same text again.
            let read = Entity::<Output>::parse(&text, form).unwrap();
            assert_eq!(read.render(representation).to_string(), text, "{kind}");
        }
    }
}

#[test]
fn a_deleted_attribute_with_a_dataset_id_keeps_its_kind_and_shape() {
    let deleted = Attribute::deleted(AttributeKind::Relationship)
        .dataset_id(iri!("urn:ngsi-ld:Dataset:a"))
        .build();
    let text = deleted.render(Lossless::Normalized).to_string();
    assert_eq!(
        text,
        r#"{"type":"Relationship","object":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:Dataset:a"}"#
    );
    assert_eq!(Attribute::<Output>::parse(&text, Lossless::Normalized).unwrap(), deleted);
}

#[test]
fn a_deleted_attribute_of_every_kind_reports_its_previous_payload() {
    let point = Geometry::from(Shape::Point(Position::try_new(13.3986, 52.5547).unwrap()));
    let deleted: [(Attribute<Output>, &str); 8] = [
        (Property::deleted().previous_value(80).build().into(), r#""previousValue":80"#),
        (
            GeoProperty::deleted().previous_value(point).build().into(),
            r#""previousValue":{"type":"Point","#,
        ),
        (
            LanguageProperty::deleted()
                .previous_language_map(LanguageMap::try_from_pairs([("en", "Main Street")]).unwrap())
                .build()
                .into(),
            r#""previousLanguageMap":{"en":"Main Street"}"#,
        ),
        (
            VocabProperty::deleted().previous_vocab("Open".parse::<VocabTerm>().unwrap()).build().into(),
            r#""previousVocab":"Open""#,
        ),
        (
            ListProperty::deleted().previous_value_list(vec![1.into(), 2.into()]).build().into(),
            r#""previousValueList":[1,2]"#,
        ),
        (
            JsonProperty::deleted().previous_json(r#"{"a":1}"#.parse().unwrap()).build().into(),
            r#""previousJson":{"a":1}"#,
        ),
        (
            Relationship::deleted()
                .previous_object(EntityId::from(iri!("urn:ngsi-ld:Person:Bob")))
                .build()
                .into(),
            r#""previousObject":"urn:ngsi-ld:Person:Bob""#,
        ),
        (
            ListRelationship::deleted()
                .previous_object_list(vec![EntityId::from(iri!("urn:ngsi-ld:Person:Bob"))])
                .build()
                .into(),
            r#""previousObjectList":[{"object":"urn:ngsi-ld:Person:Bob"}]"#,
        ),
    ];
    for (attribute, previous) in deleted {
        let text = attribute.render(Lossless::Normalized).to_string();
        assert!(text.contains(previous), "{text}");
        for form in [Lossless::Normalized, Lossless::Concise] {
            let written = attribute.render(form).to_string();
            assert_eq!(Attribute::<Output>::parse(&written, form).unwrap(), attribute, "{written}");
        }
        assert!(attribute.into_input().is_err(), "a request cannot hold the deletion");
    }
}

#[test]
fn an_input_attribute_turned_into_output_keeps_its_content_and_sub_attributes() {
    let input: Attribute<_> = Property::builder(80).property("accuracy", 1).build().unwrap().into();
    let output = input.into_output();
    let text = output.render(Lossless::Normalized).to_string();
    assert_eq!(text, r#"{"type":"Property","value":80,"accuracy":{"type":"Property","value":1}}"#);
    assert_eq!(Attribute::<Output>::parse(&text, Lossless::Normalized).unwrap(), output);
    assert_eq!(output.into_input().unwrap().render(Lossless::Normalized).to_string(), text);
}

#[test]
fn several_instances_of_an_output_attribute_are_kept_in_order() {
    let entity = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .output_attribute("speed", Property::output_builder(80).created_at(at("2024-05-01T10:00:00Z")))
        .output_attribute(
            "speed",
            Property::output_builder(Property::builder(81).dataset_id(iri!("urn:ngsi-ld:Dataset:b"))),
        )
        .build()
        .unwrap();
    assert_eq!(entity.attribute(&name("speed")).map(Instances::len), Some(2));
    assert_round_trip(&entity);
}

#[test]
fn the_output_entity_builder_reports_every_problem() {
    let error = Entity::output_builder()
        .id("not a uri")
        .entity_type("1Vehicle")
        .output_attribute("speed", Property::output_builder("urn:ngsi-ld:null"))
        .property("location", "here")
        .build()
        .unwrap_err();
    let kinds: Vec<_> = error.problems().iter().map(|problem| &problem.kind).collect();
    assert!(matches!(
        kinds.as_slice(),
        [
            ProblemKind::Identifier { .. },
            ProblemKind::Term { .. },
            ProblemKind::NgsiLdNull { .. },
            ProblemKind::Attributes { .. }
        ]
    ));
}

#[test]
fn two_default_instances_of_an_output_attribute_are_refused() {
    let error = Entity::output_builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .property("speed", 80)
        .output_attribute("speed", Property::output_builder(81))
        .build()
        .unwrap_err();
    assert!(matches!(error.problems(), [problem] if matches!(problem.kind, ProblemKind::Attributes { .. })));
}
