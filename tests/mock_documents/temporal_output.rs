//! Temporal evolutions a broker or a context source returns to Retrieve or Query Temporal Evolution, built in typed
//! form in each representation (clauses 4.5.7 to 4.5.9 and 4.5.19): build, write, read back, compare.

use indexmap::IndexMap;
use iri_rs::iri;
use ngsi_ld::model::{
    Attribute,
    AttributeKind,
    AttributesError,
    DateTime,
    InstanceId,
    Lossless,
    Output,
    ProblemKind,
    Property,
    Relationship,
    Scope,
    Value,
    evolution::{
        AggregatedAttribute,
        AggregatedKind,
        AggregatedTemporalEntity,
        AggregatedValue,
        AggregationMethod,
        ScopeInstance,
        ScopeValue,
        SimplifiedTemporalEntity,
        TemporalEntity,
        TemporalValue,
        TemporalValues,
    },
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
fn a_temporal_entity_with_identified_and_deleted_instances_round_trips() {
    let entity = TemporalEntity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .scope(
            ScopeInstance::output_builder(ScopeValue::Scopes("/Madrid".parse::<Scope>().unwrap().into()))
                .observed_at(at("2018-08-01T11:00:00Z"))
                .build(),
        )
        .scope(
            ScopeInstance::output_builder(ScopeValue::Removed)
                .observed_at(at("2018-08-01T12:00:00Z"))
                .created_at(at("2018-08-01T12:00:01Z"))
                .build(),
        )
        .output_attribute(
            "speed",
            Property::output_builder(Property::builder(120).observed_at(at("2018-08-01T12:03:00Z")))
                .instance_id(instance("urn:ngsi-ld:attribute:instance:1"))
                .created_at(at("2018-08-01T12:03:01Z")),
        )
        .property("speed", Property::builder(80).observed_at(at("2018-08-01T12:05:00Z")))
        .output_attribute(
            "speed",
            Attribute::deleted(AttributeKind::Property)
                .deleted_at(at("2018-08-01T12:07:00Z"))
                .instance_id(instance("urn:ngsi-ld:attribute:instance:3")),
        )
        .output_attribute(
            "isParked",
            Relationship::output_builder(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")).instance_id(instance("urn:ngsi-ld:attribute:instance:4")),
        )
        .output_attribute(
            "isParked",
            Attribute::deleted(AttributeKind::Relationship).deleted_at(at("2018-08-01T13:00:00Z")),
        )
        .modified_at(at("2018-08-01T13:00:00Z"))
        .build()
        .unwrap();
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = entity.render(form).to_string();
        assert_eq!(TemporalEntity::<Output>::parse(&text, form).unwrap(), entity, "{text}");
    }
    assert!(
        entity
            .render(Lossless::Concise)
            .to_string()
            .contains(r#"{"value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z","instanceId":"urn:ngsi-ld:attribute:instance:3"}"#)
    );
}

#[test]
fn the_temporal_output_builder_refuses_an_instance_of_another_kind() {
    let error = TemporalEntity::output_builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("speed", 120)
        .output_attribute("speed", Relationship::output_builder(iri!("urn:ngsi-ld:T:1")))
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [problem] if matches!(problem.kind, ProblemKind::Attributes { source: AttributesError::KindChange { .. }, .. })
    ));
}

#[test]
fn an_aggregated_temporal_entity_round_trips() {
    let period = |value: i32| AggregatedValue {
        value: Value::from(value),
        start: at("2018-08-01T12:00:00Z"),
        end: at("2018-08-01T12:04:00Z"),
    };
    let speed = AggregatedAttribute {
        kind: AggregatedKind::Property,
        methods: IndexMap::from([(AggregationMethod::Max, vec![period(120)]), (AggregationMethod::Min, vec![period(80)])]),
    };
    let parked = AggregatedAttribute {
        kind: AggregatedKind::Relationship,
        methods: IndexMap::from([(AggregationMethod::TotalCount, vec![period(2)])]),
    };
    let entity = AggregatedTemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .attribute("speed", speed)
        .attribute("isParked", parked)
        .build()
        .unwrap();
    assert_eq!(AggregatedTemporalEntity::parse(&entity.render().to_string()).unwrap(), entity);
}

#[test]
fn the_aggregated_builder_refuses_an_invalid_name_and_an_attribute_given_twice() {
    let empty = || AggregatedAttribute {
        kind: AggregatedKind::Property,
        methods: IndexMap::new(),
    };
    let error = AggregatedTemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .attribute("speed", empty())
        .attribute("speed", empty())
        .attribute("not a name", empty())
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [
            first,
            second
        ] if matches!(first.kind, ProblemKind::Attributes { source: AttributesError::ValuesNamedTwice, .. })
            && matches!(second.kind, ProblemKind::AttributeName { index: 2, .. })
    ));
}

#[test]
fn the_aggregated_builder_refuses_the_names_reserved_for_a_geo_property() {
    let aggregated = |kind| AggregatedAttribute {
        kind,
        methods: IndexMap::new(),
    };
    let error = AggregatedTemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .attribute("location", aggregated(AggregatedKind::Property))
        .attribute("observationSpace", aggregated(AggregatedKind::Relationship))
        .attribute("operationSpace", aggregated(AggregatedKind::Property))
        .attribute("speed", aggregated(AggregatedKind::Property))
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [first, second, third] if [first, second, third]
            .iter()
            .all(|problem| matches!(problem.kind, ProblemKind::Attributes { source: AttributesError::GeoPropertyRequired, .. }))
    ));
}

#[test]
fn only_properties_and_relationships_are_aggregated() {
    for kind in AttributeKind::iter() {
        match AggregatedKind::try_from(kind) {
            Ok(aggregated) => assert_eq!(aggregated.attribute_kind(), kind),
            Err(refused) => {
                assert_eq!(refused.kind, kind);
                assert_eq!(refused.to_string(), format!("a {kind} has no aggregated temporal representation"));
            }
        }
    }
    assert!(AggregatedKind::try_from(AttributeKind::GeoProperty).is_err());
}

#[test]
fn a_simplified_temporal_entity_with_a_deleted_point_and_a_removed_scope_round_trips() {
    let speed = TemporalValues::Property(vec![
        TemporalValue {
            value: Value::from(120),
            time: at("2018-08-01T12:03:00Z"),
        },
        TemporalValue {
            value: Value::from("urn:ngsi-ld:null"),
            time: at("2018-08-01T12:05:00Z"),
        },
    ]);
    let entity = SimplifiedTemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .scope(TemporalValue {
            value: ScopeValue::Removed,
            time: at("2018-08-01T12:00:00Z"),
        })
        .values("speed", speed)
        .build()
        .unwrap();
    assert_eq!(SimplifiedTemporalEntity::parse(&entity.render().to_string()).unwrap(), entity);
}

#[test]
fn the_simplified_temporal_builder_refuses_values_given_twice_and_a_location_of_another_kind() {
    let points = || TemporalValues::Property(Vec::new());
    let error = SimplifiedTemporalEntity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .values("speed", points())
        .values("speed", points())
        .values("location", points())
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [first, second]
            if matches!(first.kind, ProblemKind::Attributes { source: AttributesError::ValuesNamedTwice, .. })
            && matches!(second.kind, ProblemKind::Attributes { source: AttributesError::GeoPropertyRequired, .. })
    ));
}
