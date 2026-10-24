//! Every rejection the entity builder promises, reported together by `build()`.

use crate::model::{AttributesError, BuildProblem, Entity, EntityTypeName, NullMember, ProblemKind, Property, ReservedMember};
use iri_rs::iri;

#[test]
fn an_invalid_id_type_and_scope_are_reported_together() {
    let error = Entity::builder()
        .id("not a uri")
        .entity_type("1Vehicle")
        .scope("/Madrid//Centro")
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [
            BuildProblem {
                kind: ProblemKind::Identifier { .. },
                ..
            },
            BuildProblem {
                kind: ProblemKind::Term { .. },
                ..
            },
            BuildProblem {
                kind: ProblemKind::Scope { .. },
                ..
            },
        ]
    ));
}

#[test]
fn an_empty_list_of_types_or_scopes_is_reported() {
    let no_scopes: [&str; 0] = [];
    let error = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type(Vec::<EntityTypeName>::new())
        .scope(no_scopes)
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [
            BuildProblem {
                kind: ProblemKind::EmptyList { member: ReservedMember::Type },
                ..
            },
            BuildProblem {
                kind: ProblemKind::EmptyList { member: ReservedMember::Scope },
                ..
            },
        ]
    ));
}

#[test]
fn several_types_and_scopes_are_kept_in_order() {
    let entity = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type(["Building", "House"])
        .scope(["/Madrid", "/CompanyA"])
        .build()
        .unwrap();
    assert_eq!(entity.entity_type.iter().map(ToString::to_string).collect::<Vec<_>>(), ["Building", "House"]);
    assert_eq!(entity.scope.map(|scope| scope.iter().count()), Some(2));
}

#[test]
fn every_name_clause_5_2_4_reserves_for_geometries_refuses_another_kind() {
    for name in ["location", "observationSpace", "operationSpace"] {
        let error = Entity::builder()
            .id(iri!("urn:ngsi-ld:T:1"))
            .entity_type("T")
            .property(name, "here")
            .build()
            .unwrap_err();
        assert!(
            matches!(
                error.problems(),
                [BuildProblem {
                    kind: ProblemKind::Attributes {
                        source: AttributesError::GeoPropertyRequired,
                        ..
                    },
                    ..
                }]
            ),
            "{name}: {error}"
        );
    }
}

#[test]
fn a_null_attribute_is_reported_under_its_name() {
    let error = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .property("speed", Property::builder("urn:ngsi-ld:null"))
        .build()
        .unwrap_err();
    let [problem] = error.problems() else { panic!("{error}") };
    assert_eq!(problem.within().map(ToString::to_string).collect::<Vec<_>>(), ["speed"]);
    assert!(matches!(problem.kind, ProblemKind::NgsiLdNull { member: NullMember::Value }));
}
