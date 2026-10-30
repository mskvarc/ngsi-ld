use crate::{
    model::EntityTypeName,
    query::{
        Expected,
        ParseError,
        Problem,
        type_selection::{self, SelectableType, SelectableTypeError, TypeSelection, builders::TypeSelectionBuilder},
    },
};

#[test]
fn t_01_or_prints_a_pipe_and_a_comma_parses_to_the_same_tree() {
    let selection = type_selection::entity_type("Building").or("House").build().unwrap();
    assert_eq!(selection.to_string(), "Building|House");
    assert_eq!("Building,House".parse::<TypeSelection>().unwrap(), selection);
    assert_eq!("Building|House".parse::<TypeSelection>().unwrap(), selection);
}

#[test]
fn t_02_conjunction_is_parenthesised() {
    let selection = type_selection::all_of(["Home", "Vehicle"]).or("Motorhome").build().unwrap();
    assert_eq!(selection.to_string(), "(Home;Vehicle)|Motorhome");
    assert_eq!("(Home;Vehicle)|Motorhome".parse::<TypeSelection>().unwrap(), selection);
    assert_eq!("(Home;Vehicle),Motorhome".parse::<TypeSelection>().unwrap(), selection);
}

#[test]
fn single_conjunction_of_example_2_round_trips() {
    let selection: TypeSelection = "(Home;Vehicle)".parse().unwrap();
    assert_eq!(selection, type_selection::all_of(["Home", "Vehicle"]).build().unwrap());
    assert_eq!(selection.to_string(), "(Home;Vehicle)");
}

#[test]
fn uris_and_prefixed_names_are_entity_types() {
    let selection: TypeSelection = "https://example.org/Building|ex:House".parse().unwrap();
    assert_eq!(selection.to_string(), "https://example.org/Building|ex:House");
    assert_eq!(selection.alternatives().count(), 2);
}

#[test]
fn iri_holding_a_language_operator_is_refused() {
    let error = type_selection::entity_type("https://example.org/a,b").build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [Problem::InvalidEntityType {
                source: SelectableTypeError::Separator { found: ',', position: 21 },
                ..
            }]
        ),
        "{error:?}"
    );
    assert!("https://example.org/types(2)".parse::<SelectableType>().is_err());
}

#[test]
fn an_entity_type_name_converts_into_a_selection_builder_owned_or_borrowed() {
    let name: EntityTypeName = "Vehicle".parse().unwrap();
    let borrowed = TypeSelectionBuilder::from(&name).build().unwrap();
    assert_eq!(borrowed.to_string(), "Vehicle");
    assert_eq!(TypeSelectionBuilder::from(name).or("Home").build().unwrap().to_string(), "Vehicle|Home");
}

#[test]
fn an_entity_type_name_holding_an_operator_is_refused_when_the_selection_is_built() {
    let name: EntityTypeName = "https://example.org/a,b".parse().unwrap();
    let error = TypeSelectionBuilder::from(&name).build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::InvalidEntityType { .. }]), "{error:?}");
}

#[test]
fn invalid_names_are_all_reported() {
    let error = type_selection::entity_type("Build ing").or_all_of(["Home", "9Vehicle"]).build().unwrap_err();
    assert_eq!(error.problems().len(), 2, "{error:?}");
}

#[test]
fn empty_conjunction_is_a_problem() {
    let error = type_selection::all_of(Vec::<&str>::new()).build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::EmptyConjunction]), "{error:?}");
}

#[test]
fn unclosed_conjunction_is_unbalanced() {
    let error = "(Home;Vehicle".parse::<TypeSelection>().unwrap_err();
    assert!(matches!(error, ParseError::UnbalancedParenthesis { position: 0 }), "{error:?}");
}

#[test]
fn conjunction_without_parentheses_is_refused() {
    let error = "Home;Vehicle".parse::<TypeSelection>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::Separator,
                position: 4
            }
        ),
        "{error:?}"
    );
}

#[test]
fn trailing_separator_reports_a_missing_entity_type() {
    let error = "Building,".parse::<TypeSelection>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::EntityType,
                position: 9
            }
        ),
        "{error:?}"
    );
}
