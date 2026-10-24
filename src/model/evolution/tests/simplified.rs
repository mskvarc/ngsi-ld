use crate::model::{
    AttributeKind,
    AttributeName,
    DateTime,
    DocumentErrorKind,
    GeoValue,
    MemberError,
    Value,
    evolution::{ScopeValue, SimplifiedTemporalEntity, TemporalValue, TemporalValues},
};
use pretty_assertions::assert_eq;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// A vehicle whose members besides `id` and `type` are `members`, written as given.
fn vehicle_with(members: &str) -> String {
    format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle",{members}}}"#)
}

/// The entity of Annex C.5.6.3, with the v1.9 core context.
const C56: &str = r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z"],[80,"2018-08-01T12:05:00Z"],[100,"2018-08-01T12:07:00Z"]]},"@context":["http://example.org/ngsi-ld/latest/vehicle.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"]}"#;

#[test]
fn the_annex_simplified_evolution_reads_as_pairs_and_round_trips() {
    let entity = SimplifiedTemporalEntity::parse(C56).unwrap();
    let speed = entity.attributes.get(&"speed".parse::<AttributeName>().unwrap());
    let expected = TemporalValues::Property(vec![
        TemporalValue {
            value: Value::from(120),
            time: time("2018-08-01T12:03:00Z"),
        },
        TemporalValue {
            value: Value::from(80),
            time: time("2018-08-01T12:05:00Z"),
        },
        TemporalValue {
            value: Value::from(100),
            time: time("2018-08-01T12:07:00Z"),
        },
    ]);
    assert_eq!(speed, Some(&expected));
    assert_eq!(entity.render().to_string(), C56);
}

#[test]
fn every_example_of_clause_4_5_9_reads_as_its_kind_and_round_trips() {
    let examples = [
        (
            r#""name":{"type":"Property","values":[["Joe Bloggs","2022-08-09T18:25:02Z"],["Bill Smith","2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::Property,
        ),
        (
            r#""says":{"type":"LanguageProperty","languageMaps":[[{"languageMap":{"en":"yes","fr":"oui"}},"2022-08-09T18:25:02Z"],[{"languageMap":{"en":"no","fr":"non"}},"2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::LanguageProperty,
        ),
        (
            r#""period":{"type":"ListProperty","valueLists":[[["First","Second","Third","Fourth"],"2022-08-09T18:25:02Z"],[["1st","2nd","3rd","4th"],"2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::ListProperty,
        ),
        (
            r#""parkingTickets":{"type":"JsonProperty","jsons":[[{"json":[{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"}]},"2022-08-09T18:25:02Z"],[{"json":[{"id":"85a6cc52-0589-45f9","value":"Overstay 60 minutes"},{"id":"x5c56s-0589-45f9","value":"Overstay 45 minutes"}]},"2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::JsonProperty,
        ),
        (
            r#""gender":{"type":"VocabProperty","vocabs":[[{"vocab":"Male"},"2022-08-09T18:25:02Z"],[{"vocab":"Female"},"2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::VocabProperty,
        ),
        (
            r#""spouse":{"type":"Relationship","objects":[["urn:ngsi-ld:Person:123455","2022-08-09T18:25:02Z"],["urn:ngsi-ld:Person:999999","2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::Relationship,
        ),
        (
            r#""activeDevices":{"type":"Relationship","objects":[[["urn:ngsi-ld:Device:14142","urn:ngsi-ld:Device:13562"],"2022-08-09T18:25:02Z"]]}"#,
            AttributeKind::Relationship,
        ),
        (
            r#""membersPresent":{"type":"ListRelationship","objectLists":[[["urn:ngsi-ld:Person:Alice","urn:ngsi-ld:Person:Bob"],"2022-08-09T18:25:02Z"],[["urn:ngsi-ld:Person:Alice","urn:ngsi-ld:Person:Eve","urn:ngsi-ld:Person:Mallory"],"2022-08-10T18:25:02Z"]]}"#,
            AttributeKind::ListRelationship,
        ),
        (
            r#""location":{"type":"GeoProperty","values":[[{"type":"Point","coordinates":[8,40]},"2022-08-09T18:25:02Z"],[{"type":"Point","coordinates":[8.5,40]},"2022-08-10T18:25:02Z"],["urn:ngsi-ld:null","2022-08-11T18:25:02Z"]]}"#,
            AttributeKind::GeoProperty,
        ),
    ];
    for (member, kind) in examples {
        let text = vehicle_with(member);
        let entity = SimplifiedTemporalEntity::parse(&text).unwrap();
        let values = entity.attributes.values().next().unwrap();
        assert_eq!(values.kind(), kind, "{member}");
        assert_eq!(entity.render().to_string(), text);
    }
    let location = SimplifiedTemporalEntity::parse(&vehicle_with(examples[8].0)).unwrap();
    assert!(matches!(location.attributes.values().next(), Some(TemporalValues::GeoProperty(points)) if points[2].value == GeoValue::Null));
}

#[test]
fn the_simplified_reader_refuses_every_member_out_of_shape() {
    let refusal = |members: &str| SimplifiedTemporalEntity::parse(&vehicle_with(members)).unwrap_err();
    let untyped = refusal(r#""speed":{"values":[[120,"2018-08-01T12:03:00Z"]]}"#);
    assert_eq!(untyped.path.to_string(), "speed");
    assert!(matches!(untyped.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "type" })));
    let objects = refusal(r#""speed":{"type":"Property","objects":[]}"#);
    assert!(matches!(objects.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "objects"));
    let observed = refusal(r#""speed":{"type":"Property","values":[],"observedAt":"2018-08-01T12:03:00Z"}"#);
    assert!(matches!(observed.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "observedAt"));
    let triple = refusal(r#""speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z","x"]]}"#);
    assert_eq!(triple.path.to_string(), "speed.0");
    assert!(matches!(triple.kind, DocumentErrorKind::TemporalEntry { expected: 2, found: 3 }));
    let noon = refusal(r#""speed":{"type":"Property","values":[[120,"noon"]]}"#);
    assert!(matches!(noon.kind, DocumentErrorKind::Member(MemberError::Temporal { .. })));
    let unwrapped = refusal(r#""says":{"type":"LanguageProperty","languageMaps":[[{"en":"yes"},"2022-08-09T18:25:02Z"]]}"#);
    assert!(matches!(unwrapped.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "en"));
    let expires = refusal(r#""expiresAt":"2030-01-01T00:00:00Z""#);
    assert!(matches!(expires.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "expiresAt"));
}

#[test]
fn the_scope_is_a_property_with_values() {
    let text = vehicle_with(r#""scope":{"type":"Property","values":[[["/Madrid/Centro","/Company123/UnitA"],"2018-08-01T12:10:00Z"]]}"#);
    let entity = SimplifiedTemporalEntity::parse(&text).unwrap();
    assert!(matches!(entity.scope.as_slice(), [TemporalValue { value: ScopeValue::Scopes(scopes), .. }] if scopes.as_slice().len() == 2));
    assert_eq!(entity.render().to_string(), text);
}
