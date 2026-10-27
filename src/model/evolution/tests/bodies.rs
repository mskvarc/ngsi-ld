//! Reading the bodies a client sends that hold only the attributes of a temporal evolution, or the entity in
//! simplified temporal representation.

use crate::model::{
    AttributeKind,
    DocumentErrorKind,
    Input,
    Lossless,
    MemberError,
    NullMember,
    Output,
    Property,
    evolution::{DeletedInstance, Evolution, SimplifiedTemporalInput, TemporalAttributes},
};
use pretty_assertions::assert_eq;

/// The attributes of a temporal evolution a client sends: two instances of `speed`, then one of `brand`.
fn attributes() -> TemporalAttributes<Input> {
    let mut attributes = TemporalAttributes::<Input>::new();
    for (speed, at) in [(120, "2018-08-01T12:03:00Z"), (80, "2018-08-01T12:05:00Z")] {
        attributes
            .insert(
                "speed".parse().unwrap(),
                Property::builder(speed).observed_at(at.parse().unwrap()).build().unwrap().into(),
            )
            .unwrap();
    }
    attributes
        .insert_identified(
            "brand".parse().unwrap(),
            "urn:ngsi-ld:Instance:1".parse().unwrap(),
            Property::builder("Mercedes").build().unwrap().into(),
        )
        .unwrap();
    attributes
}

#[test]
fn the_attributes_of_an_evolution_read_back_as_they_are_written_in_both_forms() {
    let written = attributes();
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = written.render(form).to_string();
        assert_eq!(TemporalAttributes::<Input>::parse(&text, form).unwrap(), written, "{text}");
    }
}

#[test]
fn the_attributes_of_an_evolution_may_be_a_single_instance_and_carry_a_context() {
    let single = TemporalAttributes::<Input>::parse(
        r#"{"@context":"https://example.org/context.jsonld","speed":{"value":120,"observedAt":"2018-08-01T12:03:00Z"}}"#,
        Lossless::Concise,
    )
    .unwrap();
    assert_eq!(single.len(), 1);
    assert_eq!(single.get(&"speed".parse().unwrap()).map(Evolution::len), Some(1));
}

#[test]
fn the_attributes_of_an_evolution_keep_a_recorded_deletion() {
    let mut written = TemporalAttributes::<Input>::new();
    let deleted = DeletedInstance::builder(AttributeKind::Property, "2018-08-01T12:07:00Z".parse().unwrap()).build();
    written.insert_deleted("speed".parse().unwrap(), deleted).unwrap();
    let text = written.render(Lossless::Normalized).to_string();
    let read = TemporalAttributes::<Input>::parse(&text, Lossless::Normalized).unwrap();
    assert_eq!(read, written);
    assert_eq!(
        read.get(&"speed".parse().unwrap()).map(|evolution| evolution.deleted_ats().flatten().count()),
        Some(1)
    );
}

#[test]
fn the_attributes_of_an_evolution_refuse_what_a_request_cannot_carry() {
    let parse = |text: &str| TemporalAttributes::<Input>::parse(text, Lossless::Normalized);
    assert!(parse(r#"{"speed":[]}"#).is_err());
    assert!(TemporalAttributes::<Input>::parse(r#"{"speed":[]}"#, Lossless::Concise).is_err());
    assert!(parse(r#"{"speed":[{"type":"Property","value":1}],"speed":[{"type":"Property","value":2}]}"#).is_err());
    assert!(parse(r#"{"speed":[{"type":"Property","value":1},{"type":"Relationship","object":"urn:ngsi-ld:Vehicle:A4567"}]}"#).is_err());
    assert!(parse(r#"{"speed":[{"type":"Property","value":1,"createdAt":"2018-08-01T12:03:00Z"}]}"#).is_err());
    assert!(parse(r#"{"speed":[{"type":"Property","value":"urn:ngsi-ld:null"}]}"#).is_err());
    assert!(parse(r#"{"1speed":[{"type":"Property","value":1}]}"#).is_err());
    assert!(parse("[]").is_err());
}

#[test]
fn a_response_may_hold_an_attribute_with_no_instance_and_a_request_may_not() {
    let normalized = r#"{"speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}],"brand":[]}"#;
    let concise = r#"{"speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"}],"brand":[]}"#;
    for (text, form) in [(normalized, Lossless::Normalized), (concise, Lossless::Concise)] {
        let read = TemporalAttributes::<Output>::parse(text, form).unwrap();
        assert_eq!(read.get(&"brand".parse().unwrap()).map(Evolution::len), Some(0), "{text}");
        assert_eq!(read.render(form).to_string(), text);
        assert!(TemporalAttributes::<Input>::parse(text, form).is_err(), "{text}");
    }
}

#[test]
fn the_attributes_a_broker_answers_keep_their_output_only_members() {
    let text = r#"{"speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z","createdAt":"2018-08-01T12:03:01Z","instanceId":"urn:ngsi-ld:Instance:1"}]}"#;
    let read = TemporalAttributes::<Output>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn a_simplified_temporal_input_reads_back_as_it_is_written() {
    let text = concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""scope":{"type":"Property","values":[["/Madrid","2018-08-01T12:00:00Z"]]},"#,
        r#""speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z"],[80,"2018-08-01T12:05:00Z"]]}}"#
    );
    let read = SimplifiedTemporalInput::parse(text).unwrap();
    assert_eq!(read.render().to_string(), text);
    let with_context = text.replacen('{', r#"{"@context":"https://example.org/context.jsonld","#, 1);
    assert!(SimplifiedTemporalInput::parse(&with_context).unwrap().as_entity().context.is_some());
}

#[test]
fn a_simplified_temporal_input_reads_each_of_the_eight_families_of_clause_4_5_9() {
    let text = concat!(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
        r#""name":{"type":"Property","values":[["Joe Bloggs","2022-08-09T18:25:02Z"],["Bill Smith","2022-08-10T18:25:02Z"]]},"#,
        r#""location":{"type":"GeoProperty","values":[[{"type":"Point","coordinates":[13.5,52.5]},"2022-08-09T18:25:02Z"]]},"#,
        r#""says":{"type":"LanguageProperty","languageMaps":[[{"languageMap":{"en":"yes","fr":"oui"}},"2022-08-09T18:25:02Z"]]},"#,
        r#""period":{"type":"ListProperty","valueLists":[[["First","Second"],"2022-08-09T18:25:02Z"]]},"#,
        r#""parkingTickets":{"type":"JsonProperty","jsons":[[{"json":[{"id":"a","value":"b"}]},"2022-08-09T18:25:02Z"]]},"#,
        r#""gender":{"type":"VocabProperty","vocabs":[[{"vocab":"Male"},"2022-08-09T18:25:02Z"]]},"#,
        r#""spouse":{"type":"Relationship","objects":[["urn:ngsi-ld:Person:1","2022-08-09T18:25:02Z"],[["urn:ngsi-ld:Person:1","urn:ngsi-ld:Person:2"],"2022-08-10T18:25:02Z"]]},"#,
        r#""membersPresent":{"type":"ListRelationship","objectLists":[[["urn:ngsi-ld:Person:1","urn:ngsi-ld:Person:2"],"2022-08-09T18:25:02Z"]]}}"#
    );
    let read = SimplifiedTemporalInput::parse(text).unwrap();
    assert_eq!(read.as_entity().attributes.len(), 8);
    assert_eq!(read.render().to_string(), text);
}

#[test]
fn a_simplified_temporal_member_holds_its_type_and_its_data_points_only() {
    let head = r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#;
    for member in [
        r#""n":{"type":"Property","datasetId":"urn:ngsi-ld:Dataset:a","values":[[1,"2022-08-09T18:25:02Z"]]}"#,
        r#""o":{"type":"Relationship","datasetId":"urn:ngsi-ld:Dataset:a","objects":[["urn:ngsi-ld:Person:1","2022-08-09T18:25:02Z"]]}"#,
        r#""s":{"type":"LanguageProperty","datasetId":"urn:ngsi-ld:Dataset:a","languageMaps":[[{"languageMap":{"en":"yes"}},"2022-08-09T18:25:02Z"]]}"#,
        r#""p":{"type":"ListProperty","datasetId":"urn:ngsi-ld:Dataset:a","valueLists":[[["First"],"2022-08-09T18:25:02Z"]]}"#,
        r#""j":{"type":"JsonProperty","datasetId":"urn:ngsi-ld:Dataset:a","jsons":[[{"json":{"a":1}},"2022-08-09T18:25:02Z"]]}"#,
        r#""g":{"type":"VocabProperty","datasetId":"urn:ngsi-ld:Dataset:a","vocabs":[[{"vocab":"Male"},"2022-08-09T18:25:02Z"]]}"#,
        r#""m":{"type":"ListRelationship","datasetId":"urn:ngsi-ld:Dataset:a","objectLists":[[["urn:ngsi-ld:Person:1"],"2022-08-09T18:25:02Z"]]}"#,
        r#""n":{"type":"Property","unitCode":"KMH","values":[[1,"2022-08-09T18:25:02Z"]]}"#,
    ] {
        let error = SimplifiedTemporalInput::parse(&format!("{head}{member}}}")).unwrap_err();
        assert!(
            matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { .. })),
            "{member}: {error:?}"
        );
    }
}

#[test]
fn the_attributes_added_to_an_evolution_are_not_in_the_simplified_representation() {
    let simplified = r#"{"pairs":{"type":"JsonProperty","jsons":[[{"json":[{"code":"p"}]},"2022-08-09T18:25:02Z"]]}}"#;
    for form in [Lossless::Normalized, Lossless::Concise] {
        assert!(TemporalAttributes::<Input>::parse(simplified, form).is_err(), "{form:?}");
    }
}

#[test]
fn a_simplified_temporal_input_refuses_the_null_of_a_deleted_instance() {
    let attribute = SimplifiedTemporalInput::parse(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","values":[["urn:ngsi-ld:null","2018-08-01T12:03:00Z"]]}}"#,
    )
    .unwrap_err();
    assert!(matches!(attribute.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }));
    assert_eq!(attribute.path.to_string(), "speed");
    let scope = SimplifiedTemporalInput::parse(
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":{"type":"Property","values":[["urn:ngsi-ld:null","2018-08-01T12:03:00Z"]]}}"#,
    )
    .unwrap_err();
    assert!(matches!(scope.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Scope }));
    assert!(SimplifiedTemporalInput::parse(r#"{"type":"Vehicle"}"#).is_err());
    assert!(SimplifiedTemporalInput::parse(r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","expiresAt":"2030-01-01T00:00:00Z"}"#).is_err());
}
