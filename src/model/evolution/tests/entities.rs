use crate::model::{
    Attribute,
    AttributeKind,
    AttributeName,
    AttributesError,
    DateTime,
    DocumentError,
    DocumentErrorKind,
    EntityId,
    GeoValue,
    Input,
    Lossless,
    MemberError,
    NullMember,
    Output,
    ProblemKind,
    Projected,
    Property,
    Relationship,
    Value,
    evolution::{Evolution, TemporalEntity},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// An attribute name.
fn name(text: &str) -> AttributeName {
    text.parse().unwrap()
}

/// The entity of Annex C.5.5.3, with the v1.9 core context.
const C55: &str = r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"},{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z"},{"type":"Property","value":100,"observedAt":"2018-08-01T12:07:00Z"}],"@context":["http://example.org/ngsi-ld/latest/vehicle.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"]}"#;

/// Reads `text` as a response, normalized.
fn output(text: &str) -> TemporalEntity<Output> {
    TemporalEntity::<Output>::parse(text, Lossless::Normalized).unwrap()
}

/// The error reading `text` as a response, normalized.
fn output_error(text: &str) -> DocumentError {
    TemporalEntity::<Output>::parse(text, Lossless::Normalized).unwrap_err()
}

/// A vehicle whose only attribute is `member`, written as given.
fn vehicle_with(member: &str) -> String {
    format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle",{member}}}"#)
}

/// The values of the Properties of the evolution of `attribute`.
fn property_values(entity: &TemporalEntity<Output>, attribute: &str) -> Vec<Value> {
    entity
        .attribute(&name(attribute))
        .unwrap()
        .iter()
        .map(|instance| {
            let Attribute::Property(property) = instance else {
                panic!("not a Property: {instance:?}");
            };
            property.value().clone()
        })
        .collect()
}

#[test]
fn the_annex_evolution_keeps_its_instances_in_order_and_round_trips() {
    let entity = output(C55);
    assert_eq!(property_values(&entity, "speed"), [Value::from(120), Value::from(80), Value::from(100)]);
    let observed: Vec<_> = entity
        .attribute(&name("speed"))
        .unwrap()
        .iter()
        .map(|instance| instance.meta().observed_at)
        .collect();
    assert_eq!(
        observed,
        [
            Some(time("2018-08-01T12:03:00Z")),
            Some(time("2018-08-01T12:05:00Z")),
            Some(time("2018-08-01T12:07:00Z"))
        ]
    );
    assert_eq!(entity.render(Lossless::Normalized).to_string(), C55);
}

#[test]
fn the_concise_evolution_drops_each_type_and_reads_back_equal() {
    let entity = output(C55);
    let concise = entity.render(Lossless::Concise).to_string();
    assert!(
        concise.contains(r#""speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"},{"value":80,"#),
        "{concise}"
    );
    let read = TemporalEntity::<Output>::parse(&concise, Lossless::Concise).unwrap();
    assert_eq!(read, entity);
    assert_eq!(read.render(Lossless::Concise).to_string(), concise);
}

#[test]
fn a_single_instance_object_is_written_back_as_an_array_of_one() {
    let entity = output(&vehicle_with(r#""speed":{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}"#));
    assert_eq!(entity.attribute(&name("speed")).map(Evolution::len), Some(1));
    assert_eq!(
        entity.render(Lossless::Normalized).to_string(),
        vehicle_with(r#""speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}]"#)
    );
}

#[test]
fn instance_ids_system_members_and_a_deleted_instance_round_trip() {
    let text = vehicle_with(
        r#""speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z","createdAt":"2018-08-01T12:03:00Z","instanceId":"urn:ngsi-ld:attribute:instance:1"},{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z","instanceId":"urn:ngsi-ld:attribute:instance:3"}]"#,
    );
    let entity = output(&text);
    let speed = entity.attribute(&name("speed")).unwrap();
    let deleted = &speed.as_slice()[1];
    assert!(matches!(deleted, Attribute::Property(property) if property.value() == &Value::Text("urn:ngsi-ld:null".to_owned())));
    assert_eq!(deleted.meta().output.deleted_at, Some(time("2018-08-01T12:07:00Z")));
    assert_eq!(speed.kind(), Some(AttributeKind::Property));
    assert_eq!(entity.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn an_evolution_of_every_other_kind_round_trips_in_both_forms() {
    let members = [
        r#""location":[{"type":"GeoProperty","value":{"type":"Point","coordinates":[8,40]},"observedAt":"2018-08-01T12:03:00Z"},{"type":"GeoProperty","value":{"type":"Point","coordinates":[8.5,40]},"observedAt":"2018-08-01T12:05:00Z"},{"type":"GeoProperty","value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z"}]"#,
        r#""street":[{"type":"LanguageProperty","languageMap":{"fr":"Grand Place","nl":"Grote Markt"},"observedAt":"2018-08-01T12:03:00Z"}]"#,
        r#""gender":[{"type":"VocabProperty","vocab":"Male","observedAt":"2018-08-01T12:03:00Z"},{"type":"VocabProperty","vocab":"Female","observedAt":"2018-08-01T12:05:00Z"}]"#,
        r#""period":[{"type":"ListProperty","valueList":["First","Second"],"observedAt":"2018-08-01T12:03:00Z"}]"#,
        r#""parkingTickets":[{"type":"JsonProperty","json":{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"},"observedAt":"2018-08-01T12:03:00Z"}]"#,
        r#""spouse":[{"type":"Relationship","object":"urn:ngsi-ld:Person:123455","observedAt":"2018-08-01T12:03:00Z"},{"type":"Relationship","object":"urn:ngsi-ld:Person:999999","observedAt":"2018-08-01T12:05:00Z"},{"type":"Relationship","object":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z"}]"#,
        r#""membersPresent":[{"type":"ListRelationship","objectList":[{"object":"urn:ngsi-ld:Person:Alice"},{"object":"urn:ngsi-ld:Person:Bob"}],"observedAt":"2018-08-01T12:03:00Z"}]"#,
    ];
    for member in members {
        let text = vehicle_with(member);
        let entity = output(&text);
        assert_eq!(entity.render(Lossless::Normalized).to_string(), text);
        let concise = entity.render(Lossless::Concise).to_string();
        let read = TemporalEntity::<Output>::parse(&concise, Lossless::Concise).unwrap();
        assert_eq!(read, entity, "{concise}");
        assert_eq!(read.render(Lossless::Concise).to_string(), concise);
    }
    let location = output(&vehicle_with(members[0]));
    let deleted = &location.attribute(&name("location")).unwrap().as_slice()[2];
    assert!(matches!(deleted, Attribute::GeoProperty(property) if property.value() == &GeoValue::Null));
    let concise = location.render(Lossless::Concise).to_string();
    assert!(
        concise.contains(r#"{"type":"GeoProperty","value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z"}"#),
        "{concise}"
    );
}

#[test]
fn one_dataset_id_appears_in_many_instances() {
    let text = vehicle_with(
        r#""speed":[{"type":"Property","value":120,"datasetId":"urn:ngsi-ld:dataset:gps","observedAt":"2018-08-01T12:03:00Z"},{"type":"Property","value":80,"datasetId":"urn:ngsi-ld:dataset:gps","observedAt":"2018-08-01T12:05:00Z"}]"#,
    );
    assert_eq!(output(&text).render(Lossless::Normalized).to_string(), text);
}

#[test]
fn an_instance_of_another_kind_is_refused() {
    let error = output_error(&vehicle_with(
        r#""speed":[{"type":"Property","value":120},{"type":"Relationship","object":"urn:ngsi-ld:Person:1"}]"#,
    ));
    assert_eq!(error.path.to_string(), "speed");
    assert!(matches!(
        error.kind,
        DocumentErrorKind::Attributes {
            source: AttributesError::KindChange {
                expected: AttributeKind::Property,
                found: AttributeKind::Relationship
            }
        }
    ));
}

#[test]
fn a_location_evolution_holds_geo_properties_only() {
    let error = output_error(&vehicle_with(r#""location":[{"type":"Property","value":"Brussels"}]"#));
    assert_eq!(error.path.to_string(), "location");
    assert!(matches!(
        error.kind,
        DocumentErrorKind::Attributes {
            source: AttributesError::GeoPropertyRequired
        }
    ));
}

#[test]
fn a_request_holds_no_system_member_and_no_null() {
    let read = |member: &str| TemporalEntity::<Input>::parse(&vehicle_with(member), Lossless::Normalized).unwrap_err();
    let deleted_at = read(r#""speed":[{"type":"Property","value":120,"deletedAt":"2018-08-01T12:05:00Z"}]"#);
    assert_eq!(deleted_at.path.to_string(), "speed");
    assert!(matches!(deleted_at.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "deletedAt"));
    let sub_instance_id =
        read(r#""speed":[{"type":"Property","value":120,"accuracy":{"type":"Property","value":1,"instanceId":"urn:ngsi-ld:attribute:instance:1"}}]"#);
    assert!(matches!(sub_instance_id.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "instanceId"));
    let null = read(r#""speed":[{"type":"Property","value":"urn:ngsi-ld:null"}]"#);
    assert_eq!(null.path.to_string(), "speed");
    assert!(matches!(null.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }));
    let created = read(r#""createdAt":"2018-08-01T12:03:00Z""#);
    assert!(matches!(created.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "createdAt"));
}

#[test]
fn the_builder_adds_each_named_instance_to_its_evolution() {
    let entity = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(time("2018-08-01T12:03:00Z")))
        .property("speed", Property::builder(80).observed_at(time("2018-08-01T12:05:00Z")))
        .build()
        .unwrap();
    assert_eq!(
        entity.render(Lossless::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"},{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z"}]}"#
    );
    assert_eq!(
        TemporalEntity::<Input>::parse(&entity.render(Lossless::Concise).to_string(), Lossless::Concise).unwrap(),
        entity
    );
}

#[test]
fn the_builder_reports_every_problem_together() {
    let error = TemporalEntity::builder()
        .id("not a uri")
        .entity_type("Vehicle")
        .property("1speed", 1)
        .property("speed", 120)
        .relationship("speed", Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Person:1"))))
        .build()
        .unwrap_err();
    let kinds: Vec<_> = error.problems().iter().map(|problem| &problem.kind).collect();
    assert!(
        matches!(
            kinds.as_slice(),
            [
                ProblemKind::Identifier { .. },
                ProblemKind::AttributeName { .. },
                ProblemKind::Attributes {
                    source: AttributesError::KindChange {
                        expected: AttributeKind::Property,
                        found: AttributeKind::Relationship
                    },
                    ..
                }
            ]
        ),
        "{kinds:?}"
    );
}

#[test]
fn a_projected_evolution_may_lack_id_and_type() {
    let text = r#"{"speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}]}"#;
    let projected = TemporalEntity::<Output, Projected>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(projected.id, None);
    assert_eq!(projected.entity_type, None);
    assert_eq!(projected.render(Lossless::Normalized).to_string(), text);
    assert!(matches!(
        output_error(text).kind,
        DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })
    ));
}

#[test]
fn the_attributes_alone_are_the_body_of_add_attributes() {
    let entity = TemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(time("2018-08-01T12:03:00Z")))
        .property("speed", Property::builder(80).observed_at(time("2018-08-01T12:05:00Z")))
        .build()
        .unwrap();
    assert_eq!(
        entity.attributes.render(Lossless::Normalized).to_string(),
        r#"{"speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"},{"type":"Property","value":80,"observedAt":"2018-08-01T12:05:00Z"}]}"#
    );
}
