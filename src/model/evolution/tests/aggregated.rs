use crate::model::{
    AttributeKind,
    AttributeName,
    DateTime,
    DocumentErrorKind,
    MemberError,
    Value,
    evolution::{AggregatedKind, AggregatedTemporalEntity, AggregatedValue, AggregationMethod},
};
use pretty_assertions::assert_eq;
use strum::IntoEnumIterator;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// A vehicle whose members besides `id` and `type` are `members`, written as given.
fn vehicle_with(members: &str) -> String {
    format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle",{members}}}"#)
}

/// The entity of Annex C.5.14.3, with the v1.9 core context.
const C514: &str = r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","max":[[120,"2018-08-01T12:00:00Z","2018-08-01T12:04:00Z"],[100,"2018-08-01T12:04:00Z","2018-08-01T12:08:00Z"]],"avg":[[120,"2018-08-01T12:00:00Z","2018-08-01T12:04:00Z"],[90,"2018-08-01T12:04:00Z","2018-08-01T12:08:00Z"]]},"@context":["http://example.org/ngsi-ld/latest/vehicle.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"]}"#;

#[test]
fn the_annex_aggregated_evolution_reads_per_method_and_round_trips() {
    let entity = AggregatedTemporalEntity::parse(C514).unwrap();
    let speed = entity.attributes.get(&"speed".parse::<AttributeName>().unwrap()).unwrap();
    assert_eq!(speed.kind, AggregatedKind::Property);
    assert_eq!(
        speed.methods.keys().copied().collect::<Vec<_>>(),
        [AggregationMethod::Max, AggregationMethod::Avg]
    );
    assert_eq!(
        speed.methods.get(&AggregationMethod::Max).and_then(|periods| periods.first()),
        Some(&AggregatedValue {
            value: Value::from(120),
            start: time("2018-08-01T12:00:00Z"),
            end: time("2018-08-01T12:04:00Z"),
        })
    );
    assert_eq!(entity.render().to_string(), C514);
}

#[test]
fn a_relationship_and_every_method_round_trip_in_document_order() {
    let period = r#"[[3,"2018-08-01T12:00:00Z","2018-08-01T13:00:00Z"]]"#;
    let relationship = vehicle_with(&format!(
        r#""isParked":{{"type":"Relationship","distinctCount":{period},"totalCount":{period}}}"#
    ));
    let entity = AggregatedTemporalEntity::parse(&relationship).unwrap();
    assert!(entity.attributes.values().all(|attribute| attribute.kind == AggregatedKind::Relationship));
    assert_eq!(entity.render().to_string(), relationship);
    let methods: Vec<_> = AggregationMethod::iter().map(|method| format!(r#""{method}":{period}"#)).collect();
    let property = vehicle_with(&format!(r#""speed":{{"type":"Property",{}}}"#, methods.join(",")));
    let entity = AggregatedTemporalEntity::parse(&property).unwrap();
    assert_eq!(entity.attributes.values().next().map(|attribute| attribute.methods.len()), Some(8));
    assert_eq!(entity.render().to_string(), property);
}

#[test]
fn the_aggregated_reader_refuses_every_member_out_of_shape() {
    let refusal = |members: &str| AggregatedTemporalEntity::parse(&vehicle_with(members)).unwrap_err();
    let geo = refusal(r#""location":{"type":"GeoProperty","max":[]}"#);
    assert_eq!(geo.path.to_string(), "location");
    assert!(matches!(
        geo.kind,
        DocumentErrorKind::AggregatedKind {
            found: AttributeKind::GeoProperty
        }
    ));
    let json = refusal(r#""payload":{"type":"JsonProperty","max":[]}"#);
    assert_eq!(json.path.to_string(), "payload");
    assert!(matches!(
        json.kind,
        DocumentErrorKind::AggregatedKind {
            found: AttributeKind::JsonProperty
        }
    ));
    let median = refusal(r#""speed":{"type":"Property","median":[]}"#);
    assert!(matches!(median.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "median"));
    let dataset = refusal(r#""speed":{"type":"Property","datasetId":"urn:ngsi-ld:Dataset:1","max":[]}"#);
    assert!(matches!(dataset.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "datasetId"));
    let null = refusal(r#""speed":{"type":"Property","avg":[[null,"2018-08-01T12:00:00Z","2018-08-01T12:04:00Z"]]}"#);
    assert_eq!(null.path.to_string(), "speed.avg.0");
    let pair = refusal(r#""speed":{"type":"Property","max":[[120,"2018-08-01T12:00:00Z"]]}"#);
    assert_eq!(pair.path.to_string(), "speed.max.0");
    assert!(matches!(pair.kind, DocumentErrorKind::TemporalEntry { expected: 3, found: 2 }));
    let untyped = refusal(r#""speed":{"max":[]}"#);
    assert!(matches!(untyped.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "type" })));
    let scope = refusal(r#""scope":"/Madrid""#);
    assert!(matches!(scope.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "scope"));
}
