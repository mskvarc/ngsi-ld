//! Every rejection the attribute builders promise.

use crate::model::{
    AttributesError,
    BuildError,
    BuildProblem,
    DatasetId,
    EntityId,
    EntityTypeName,
    GeoProperty,
    InstancesError,
    JsonProperty,
    LanguageMap,
    LanguageProperty,
    ListProperty,
    ListRelationship,
    NullMember,
    ProblemKind,
    Property,
    RawJson,
    Relationship,
    ReservedMember,
    Value,
    VocabProperty,
    VocabTerm,
    geometry::{Geometry, Position, Shape},
    null::NGSI_LD_NULL,
};
use iri_rs::iri;

/// The attribute path of a problem, as text.
fn within(problem: &BuildProblem) -> Vec<String> {
    problem.within().map(ToString::to_string).collect()
}

/// Asserts that a build failed with one problem: the NGSI-LD Null in `member`, at the attribute path `path`.
fn assert_null<T>(result: Result<T, BuildError>, path: &[&str], member: NullMember) {
    let Err(error) = result else { panic!("the build succeeded") };
    let [problem] = error.problems() else { panic!("{error}") };
    assert_eq!(within(problem), path, "{error}");
    assert!(matches!(problem.kind, ProblemKind::NgsiLdNull { member: found } if found == member), "{error}");
}

/// Asserts that a build failed with one problem: an empty list for `member`.
fn assert_empty<T>(result: Result<T, BuildError>, member: ReservedMember) {
    let Err(error) = result else { panic!("the build succeeded") };
    let [problem] = error.problems() else { panic!("{error}") };
    assert!(matches!(problem.kind, ProblemKind::EmptyList { member: found } if found == member), "{error}");
}

fn point() -> Geometry {
    Geometry::from(Shape::Point(Position::try_new(8.0, 40.0).unwrap()))
}

#[test]
fn every_builder_refuses_the_null_as_its_payload() {
    assert_null(Property::builder(NGSI_LD_NULL.as_str()).build(), &[], NullMember::Value);
    let holding = Value::from_json(r#"{"a":"urn:ngsi-ld:null"}"#.parse().unwrap()).unwrap();
    assert_null(Property::builder(holding).build(), &[], NullMember::ValueMember);
    let map = LanguageMap::try_from_pairs([("@none", NGSI_LD_NULL.as_str())]).unwrap();
    assert_null(LanguageProperty::builder(map).build(), &[], NullMember::LanguageMap);
    assert_null(
        VocabProperty::builder(NGSI_LD_NULL.as_str().parse::<VocabTerm>().unwrap()).build(),
        &[],
        NullMember::Vocab,
    );
    assert_null(
        ListProperty::builder(vec![Value::from(NGSI_LD_NULL.as_str())]).build(),
        &[],
        NullMember::ValueList,
    );
    let null_id: EntityId = NGSI_LD_NULL.as_str().parse().unwrap();
    assert_null(Relationship::builder(null_id.clone()).build(), &[], NullMember::Object);
    assert_null(ListRelationship::builder(vec![null_id]).build(), &[], NullMember::ObjectList);
}

#[test]
fn a_null_in_a_sub_attribute_is_reported_under_its_name() {
    let result = Property::builder(1).relationship("providedBy", iri!("urn:ngsi-ld:null")).build();
    assert_null(result, &["providedBy"], NullMember::Object);
    let nested = Property::builder(1).property("accuracy", Property::builder(NGSI_LD_NULL.as_str())).build();
    assert_null(nested, &["accuracy"], NullMember::Value);
}

#[test]
fn a_json_property_refuses_json_that_is_not_an_object_or_an_array_of_objects() {
    for text in ["1", r#""text""#, "[1, 2]", r#"[{"a": 1}, 2]"#] {
        let json: RawJson = text.parse().unwrap();
        let error = JsonProperty::builder(json).build().unwrap_err();
        assert!(
            matches!(
                error.problems(),
                [BuildProblem {
                    kind: ProblemKind::JsonShape,
                    ..
                }]
            ),
            "{text}"
        );
    }
    assert!(JsonProperty::builder(r#"[{"a": 1}]"#.parse::<RawJson>().unwrap()).build().is_ok());
}

#[test]
fn the_list_payloads_and_object_types_refuse_an_empty_list() {
    assert_empty(Relationship::builder(Vec::<EntityId>::new()).build(), ReservedMember::Object);
    assert_empty(VocabProperty::builder(Vec::<VocabTerm>::new()).build(), ReservedMember::Vocab);
    let no_types = Vec::<EntityTypeName>::new();
    assert_empty(
        Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:T:1")))
            .object_type(no_types.clone())
            .build(),
        ReservedMember::ObjectType,
    );
    assert_empty(ListRelationship::builder(vec![]).object_type(no_types).build(), ReservedMember::ObjectType);
}

#[test]
fn a_relationship_accepts_several_objects_and_object_types() {
    let objects = vec![EntityId::from(iri!("urn:ngsi-ld:T:1")), EntityId::from(iri!("urn:ngsi-ld:T:2"))];
    let types = vec!["T".parse::<EntityTypeName>().unwrap(), "U".parse().unwrap()];
    let relationship = Relationship::builder(objects).object_type(types).build().unwrap();
    assert!(relationship.object().is_many());
    assert_eq!(relationship.object_type().map(|types| types.as_slice().len()), Some(2));
}

#[test]
fn every_builder_reports_an_invalid_sub_attribute_name_and_a_repeated_dataset_id_together() {
    let gps: DatasetId = iri!("urn:ngsi-ld:dataset:gps").into();
    let error = GeoProperty::builder(point())
        .property("1accuracy", 1)
        .property("accuracy", Property::builder(1).dataset_id(gps.clone()))
        .property("accuracy", Property::builder(2).dataset_id(gps))
        .build()
        .unwrap_err();
    let [name, instances] = error.problems() else { panic!("{error}") };
    assert!(matches!(&name.kind, ProblemKind::AttributeName { index: 0, .. }));
    assert!(matches!(
        &instances.kind,
        ProblemKind::Attributes {
            source: AttributesError::Instances {
                source: InstancesError::DuplicateDatasetId { .. }
            },
            ..
        }
    ));
}

#[test]
fn a_sub_attribute_named_location_must_be_a_geo_property() {
    let error = Property::builder(1).property("location", "here").build().unwrap_err();
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
    assert!(Property::builder(1).geo_property("location", point()).build().is_ok());
}

#[test]
fn a_value_validated_by_hand_converts_its_error_into_a_build_error() {
    let error: BuildError = "@id".parse::<VocabTerm>().unwrap_err().into();
    assert!(matches!(
        error.problems(),
        [BuildProblem {
            kind: ProblemKind::Term { .. },
            ..
        }]
    ));
}
