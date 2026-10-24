//! The simplified fragment: what each builder call writes, and every rejection `build()` promises.

use crate::model::{
    AttributesError,
    BuildProblem,
    DatasetId,
    InstancesError,
    LanguageMap,
    NullMember,
    OneOrMany,
    ProblemKind,
    RawJson,
    ScopeChange,
    SimplifiedFragment,
    SimplifiedValue,
    Value,
    VocabTerm,
    geometry::{Geometry, Position, Shape},
};
use indexmap::IndexMap;
use iri_rs::iri;

/// The body a fragment writes, as compact JSON text.
fn rendered(fragment: &SimplifiedFragment) -> String {
    fragment.render().to_string()
}

#[test]
fn a_value_is_written_bare_as_the_default_instance() {
    let fragment = SimplifiedFragment::builder().value("speed", 55).value("brandName", "Mercedes").build().unwrap();
    assert_eq!(rendered(&fragment), r#"{"speed":55,"brandName":"Mercedes"}"#);
}

#[test]
fn every_simplified_kind_is_written_as_clause_4_5_4_shows_it() {
    let fragment = SimplifiedFragment::builder()
        .set("location", Geometry::from(Shape::Point(Position::try_new(13.3986, 52.5547).unwrap())))
        .set("street", LanguageMap::try_from_pairs([("fr", "Grand Place"), ("nl", "Grote Markt")]).unwrap())
        .set("gender", OneOrMany::from("Male".parse::<VocabTerm>().unwrap()))
        .set("parkingTickets", r#"{"id": "85a6cc52-0589-45f9"}"#.parse::<RawJson>().unwrap())
        .value("providedBy", "urn:ngsi-ld:Device:31415")
        .build()
        .unwrap();
    assert_eq!(
        rendered(&fragment),
        concat!(
            r#"{"location":{"type":"Point","coordinates":[13.3986,52.5547]},"#,
            r#""street":{"languageMap":{"fr":"Grand Place","nl":"Grote Markt"}},"#,
            r#""gender":{"vocab":"Male"},"#,
            r#""parkingTickets":{"json":{"id":"85a6cc52-0589-45f9"}},"#,
            r#""providedBy":"urn:ngsi-ld:Device:31415"}"#,
        )
    );
}

#[test]
fn a_deletion_writes_the_bare_ngsi_ld_null() {
    let fragment = SimplifiedFragment::builder().delete("isParked").build().unwrap();
    assert_eq!(rendered(&fragment), r#"{"isParked":"urn:ngsi-ld:null"}"#);
}

#[test]
fn instances_are_written_as_a_dataset_object_with_the_default_instance_as_none() {
    let fragment = SimplifiedFragment::builder()
        .value("speed", 79)
        .set_instance("speed", iri!("urn:ngsi-ld:dataset:gps"), Value::from(80))
        .build()
        .unwrap();
    assert_eq!(rendered(&fragment), r#"{"speed":{"dataset":{"@none":79,"urn:ngsi-ld:dataset:gps":80}}}"#);
}

#[test]
fn one_named_instance_is_written_as_a_dataset_object() {
    let fragment = SimplifiedFragment::builder()
        .delete_instance("speed", iri!("urn:ngsi-ld:dataset:gps"))
        .set_instance("temperature", DatasetId::DefaultInstance, Value::from(21))
        .build()
        .unwrap();
    assert_eq!(
        rendered(&fragment),
        r#"{"speed":{"dataset":{"urn:ngsi-ld:dataset:gps":"urn:ngsi-ld:null"}},"temperature":{"dataset":{"@none":21}}}"#
    );
}

#[test]
fn id_type_and_scope_come_first_and_a_scope_deletion_writes_the_null() {
    let fragment = SimplifiedFragment::builder()
        .value("speed", 0)
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .scope(ScopeChange::Delete)
        .build()
        .unwrap();
    assert_eq!(
        rendered(&fragment),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","scope":"urn:ngsi-ld:null","speed":0}"#
    );
}

#[test]
fn a_value_holding_the_ngsi_ld_null_is_refused_where_the_null_is() {
    let mut member = IndexMap::new();
    member.insert("inner".to_owned(), Value::from("urn:ngsi-ld:null"));
    let error = SimplifiedFragment::builder()
        .value("speed", "urn:ngsi-ld:null")
        .value("tyres", vec!["urn:ngsi-ld:null"])
        .value("address", Value::Object(member))
        .set("street", LanguageMap::try_from_pairs([("@none", "urn:ngsi-ld:null")]).unwrap())
        .build()
        .unwrap_err();
    let members: Vec<_> = error
        .problems()
        .iter()
        .map(|problem| match &problem.kind {
            ProblemKind::NgsiLdNull { member } => *member,
            other @ (ProblemKind::AttributeName { .. }
            | ProblemKind::Term { .. }
            | ProblemKind::Scope { .. }
            | ProblemKind::Identifier { .. }
            | ProblemKind::LanguageMap { .. }
            | ProblemKind::Geometry { .. }
            | ProblemKind::Json { .. }
            | ProblemKind::JsonShape
            | ProblemKind::Attributes { .. }
            | ProblemKind::EmptyList { .. }
            | ProblemKind::SubAttributeOfUnsetAttribute { .. }
            | ProblemKind::ProofNotAnObject
            | ProblemKind::DatasetValue) => panic!("unexpected problem {other}"),
        })
        .collect();
    assert_eq!(
        members,
        [NullMember::Value, NullMember::ValueList, NullMember::ValueMember, NullMember::LanguageMap]
    );
}

#[test]
fn a_dataset_value_is_refused() {
    let mut instances = IndexMap::new();
    instances.insert(DatasetId::DefaultInstance, SimplifiedValue::Value(Value::from(1)));
    let error = SimplifiedFragment::builder()
        .set("speed", SimplifiedValue::Dataset(instances))
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [BuildProblem {
            kind: ProblemKind::DatasetValue,
            ..
        }]
    ));
}

#[test]
fn a_geometry_name_takes_only_a_geometry_or_a_deletion() {
    let error = SimplifiedFragment::builder().value("location", "Brussels").build().unwrap_err();
    assert!(matches!(
        error.problems(),
        [BuildProblem {
            kind: ProblemKind::Attributes {
                source: AttributesError::GeoPropertyRequired,
                ..
            },
            ..
        }]
    ));
    let fragment = SimplifiedFragment::builder()
        .set("location", Geometry::from(Shape::Point(Position::try_new(4.35, 50.85).unwrap())))
        .delete("observationSpace")
        .build();
    assert!(fragment.is_ok());
}

#[test]
fn an_instance_named_twice_is_refused() {
    let error = SimplifiedFragment::builder()
        .set_instance("speed", iri!("urn:ngsi-ld:dataset:gps"), Value::from(1))
        .set_instance("speed", iri!("urn:ngsi-ld:dataset:gps"), Value::from(2))
        .value("rpm", 1)
        .set_instance("rpm", DatasetId::DefaultInstance, Value::from(2))
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [
            BuildProblem {
                kind: ProblemKind::Attributes {
                    source: AttributesError::Instances {
                        source: InstancesError::DuplicateDatasetId { .. }
                    },
                    ..
                },
                ..
            },
            BuildProblem {
                kind: ProblemKind::Attributes {
                    source: AttributesError::Instances {
                        source: InstancesError::DefaultTwice
                    },
                    ..
                },
                ..
            },
        ]
    ));
}

#[test]
fn a_bare_deletion_among_instances_is_refused() {
    let error = SimplifiedFragment::builder()
        .delete("speed")
        .set_instance("speed", iri!("urn:ngsi-ld:dataset:gps"), Value::from(1))
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [BuildProblem {
            kind: ProblemKind::Attributes {
                source: AttributesError::Instances {
                    source: InstancesError::BareDeletionAmongInstances
                },
                ..
            },
            ..
        }]
    ));
}

#[test]
fn an_invalid_name_and_an_invalid_type_are_reported_together() {
    let error = SimplifiedFragment::builder().value("1speed", 1).entity_type("1Vehicle").build().unwrap_err();
    assert!(matches!(
        error.problems(),
        [
            BuildProblem {
                kind: ProblemKind::Term { .. },
                ..
            },
            BuildProblem {
                kind: ProblemKind::AttributeName { .. },
                ..
            },
        ]
    ));
}
