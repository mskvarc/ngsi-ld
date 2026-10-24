#[cfg(feature = "linked-data")]
use crate::query::depth_check::assert_maps_to_itself;
use crate::query::{
    Expected,
    ParseError,
    Problem,
    depth_check::assert_walks,
    projection::{self, EntityMember, Projection, ProjectionTerm},
};

#[test]
fn r_01_attributes_are_joined_with_commas() {
    let pick = projection::attributes(["temperature", "humidity"]).build().unwrap();
    assert_eq!(pick.to_string(), "temperature,humidity");
    assert_eq!("temperature,humidity".parse::<Projection>().unwrap(), pick);
}

#[test]
fn r_02_linked_projection_is_braced() {
    let pick = projection::linked("observation", projection::attributes(["temperature", "humidity"]))
        .build()
        .unwrap();
    assert_eq!(pick.to_string(), "observation{temperature,humidity}");
    assert_eq!("observation{temperature,humidity}".parse::<Projection>().unwrap(), pick);
}

#[test]
fn pipe_separator_parses_to_the_same_tree() {
    let pick = projection::attributes(["temperature", "humidity"]).build().unwrap();
    assert_eq!("temperature|humidity".parse::<Projection>().unwrap(), pick);
}

#[test]
fn entity_members_are_their_own_terms() {
    let pick: Projection = "id,type,scope,temperature".parse().unwrap();
    assert!(matches!(
        pick.terms(),
        [
            ProjectionTerm::Member(EntityMember::Id),
            ProjectionTerm::Member(EntityMember::Type),
            ProjectionTerm::Member(EntityMember::Scope),
            ProjectionTerm::Attribute(_)
        ]
    ));
    assert_eq!(pick.to_string(), "id,type,scope,temperature");
    let built = projection::member(EntityMember::Id)
        .attribute("type")
        .member(EntityMember::Scope)
        .attribute("temperature")
        .build()
        .unwrap();
    assert_eq!(built, pick);
}

#[test]
fn nested_linked_projections_round_trip() {
    let text = "observation{sensor{id,humidity},temperature}";
    let pick: Projection = text.parse().unwrap();
    assert_eq!(pick.to_string(), text);
    let built = projection::linked(
        "observation",
        projection::linked("sensor", projection::attributes(["id", "humidity"])).attribute("temperature"),
    )
    .build()
    .unwrap();
    assert_eq!(built, pick);
}

#[test]
fn linked_projection_on_an_entity_member_is_a_problem() {
    let error = projection::linked("id", projection::attributes(["name"])).build().unwrap_err();
    assert!(
        matches!(error.problems(), [Problem::LinkedEntityMember { member: EntityMember::Id }]),
        "{error:?}"
    );
}

#[test]
fn empty_projection_is_a_problem() {
    let error = projection::attributes(Vec::<&str>::new()).build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::EmptyProjection]), "{error:?}");
}

#[test]
fn invalid_names_are_reported() {
    let error = projection::attributes(["temperature", "relative humidity"]).build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::InvalidName { .. }]), "{error:?}");
}

#[test]
fn unclosed_linked_projection_reports_the_missing_brace() {
    let error = "observation{temperature".parse::<Projection>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::ClosingBrace,
                position: 23
            }
        ),
        "{error:?}"
    );
}

#[test]
fn trailing_separator_reports_a_missing_term() {
    let error = "temperature,".parse::<Projection>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::ProjectionTerm,
                position: 12
            }
        ),
        "{error:?}"
    );
}

/// How deep the depth tests nest: far deeper than any thread's stack holds, one frame per level.
const DEPTH: usize = 200_000;

#[test]
fn a_projection_nested_200000_levels_deep_round_trips_and_walks() {
    let text = format!("{}b{}", "a{".repeat(DEPTH), "}".repeat(DEPTH));
    let pick: Projection = text.parse().unwrap();
    assert_eq!(pick.to_string(), text);
    let built = (0..DEPTH)
        .fold(projection::attributes(["b"]), |inner, _| projection::linked("a", inner))
        .build()
        .unwrap();
    assert_eq!(built, pick);
    assert_walks(&pick);
    #[cfg(feature = "linked-data")]
    assert_maps_to_itself(&pick);
}

#[test]
fn an_unclosed_projection_200000_levels_deep_reports_the_missing_brace() {
    let text = format!("{}b", "a{".repeat(DEPTH));
    let error = text.parse::<Projection>().unwrap_err();
    assert!(
        matches!(error, ParseError::Unexpected { expected: Expected::ClosingBrace, position } if position == text.len()),
        "{error}"
    );
}
