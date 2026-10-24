use crate::{
    model::ScopeError,
    query::{
        Expected,
        ParseError,
        Problem,
        scope_query::{self, LevelPattern, ScopeAlternative, ScopePattern, ScopeQuery, builders::ScopeQueryBuilder},
    },
};
use pretty_assertions::assert_eq;

/// Parses a scope query.
fn parse(text: &str) -> ScopeQuery {
    text.parse().unwrap()
}

/// The text of each level of a pattern.
fn levels(pattern: &ScopePattern) -> Vec<String> {
    pattern.levels().iter().map(ToString::to_string).collect()
}

#[test]
fn example_1_one_scope() {
    let query = scope_query::scope("/Madrid").build().unwrap();
    assert!(matches!(query.alternatives(), [ScopeAlternative::Scope(pattern)] if levels(pattern) == ["Madrid"]));
    assert_eq!(query.to_string(), "/Madrid");
    assert_eq!(parse("/Madrid"), query);
}

#[test]
fn example_2_a_scope_and_the_tree_below_it() {
    let query = scope_query::subtree("/Madrid/Gardens").build().unwrap();
    assert!(matches!(query.alternatives(), [ScopeAlternative::Subtree(pattern)] if levels(pattern) == ["Madrid", "Gardens"]));
    assert_eq!(query.to_string(), "/Madrid/Gardens/#");
    assert_eq!(parse("/Madrid/Gardens/#"), query);
}

#[test]
fn example_3_a_wildcard_level() {
    let query = scope_query::scope("/Madrid/+/ParqueNorte").build().unwrap();
    let [ScopeAlternative::Scope(pattern)] = query.alternatives() else {
        panic!("{query:?}");
    };
    assert_eq!(pattern.levels()[1], LevelPattern::AnyLevel);
    assert_eq!(levels(pattern), ["Madrid", "+", "ParqueNorte"]);
    assert_eq!(query.to_string(), "/Madrid/+/ParqueNorte");
}

#[test]
fn example_4_a_conjunction() {
    let query = scope_query::all_of(["/Madrid/Districts", "/CompanyA"]).build().unwrap();
    assert!(matches!(query.alternatives(), [ScopeAlternative::AllOf(conjunction)] if conjunction.patterns().len() == 2));
    assert_eq!(query.to_string(), "(/Madrid/Districts;/CompanyA)");
    assert_eq!(parse("(/Madrid/Districts;/CompanyA)"), query);
}

#[test]
fn example_5_reads_both_or_operators_and_the_scope_without_its_slash() {
    let built = scope_query::all_of(["/Madrid/Districts", "/CompanyA"]).or("/CompanyB").build().unwrap();
    assert_eq!(parse("(/Madrid/Districts;/CompanyA)|CompanyB"), built);
    assert_eq!(parse("(/Madrid/Districts;/CompanyA),CompanyB"), built);
    assert_eq!(built.to_string(), "(/Madrid/Districts;/CompanyA)|/CompanyB");
}

#[test]
fn any_scope() {
    let query = scope_query::any_scope();
    assert!(query.is_any_scope());
    assert_eq!(query.alternatives(), []);
    assert_eq!(query.to_string(), "/#");
    assert_eq!(parse("/#"), query);
    assert!(!parse("/Madrid").is_any_scope());
}

#[test]
fn printed_queries_round_trip() {
    for text in [
        "/a/#|(/b;/c/+)|/d",
        "/Madrid",
        "/Madrid/Gardens/#",
        "/Madrid/+/ParqueNorte",
        "(/Madrid/Districts;/CompanyA)",
        "(/Madrid/Districts;/CompanyA)|/CompanyB",
        "/#",
    ] {
        let query = parse(text);
        assert_eq!(query.to_string(), text);
        assert_eq!(parse(&query.to_string()), query);
    }
}

#[test]
fn queries_outside_the_grammar_are_refused_where_they_go_wrong() {
    let unexpected = [
        ("", Expected::Scope, 0),
        ("/Madrid|", Expected::Scope, 8),
        ("/Madrid;/CompanyA", Expected::Separator, 7),
    ];
    for (text, expected, position) in unexpected {
        let error = text.parse::<ScopeQuery>().unwrap_err();
        assert!(
            matches!(error, ParseError::Unexpected { expected: found, position: at } if found == expected && at == position),
            "{text:?}: {error:?}"
        );
    }
    assert!(matches!(
        "(/Madrid;/CompanyA".parse::<ScopeQuery>(),
        Err(ParseError::UnbalancedParenthesis { position: 0 })
    ));
    let invalid = [
        ("/Madrid//Gardens", 0, ScopeError::EmptyLevel { position: 8 }),
        ("/Madrid/1Gardens", 0, ScopeError::InvalidStart { found: '1', position: 8 }),
        ("(/Madrid/#;/A)", 1, ScopeError::InvalidStart { found: '#', position: 8 }),
        ("/#|/Madrid", 0, ScopeError::EmptyLevel { position: 0 }),
        ("/Mad rid", 0, ScopeError::InvalidCharacter { found: ' ', position: 4 }),
    ];
    for (text, position, reason) in invalid {
        let error = text.parse::<ScopeQuery>().unwrap_err();
        assert!(
            matches!(error, ParseError::InvalidScope { position: at, source } if at == position && source == reason),
            "{text:?}: {error:?}"
        );
    }
}

#[test]
fn the_builder_reports_every_problem() {
    let no_patterns: [&str; 0] = [];
    let error = scope_query::scope("/Madrid//Gardens")
        .or_all_of(no_patterns)
        .or_subtree("+bad")
        .build()
        .unwrap_err();
    assert_eq!(
        error.problems(),
        [
            Problem::InvalidScopePattern {
                source: ScopeError::EmptyLevel { position: 8 },
            },
            Problem::EmptyScopeConjunction,
            Problem::InvalidScopePattern {
                source: ScopeError::InvalidStart { found: '+', position: 0 },
            },
        ]
    );
    let combined = ScopeQueryBuilder::from(scope_query::any_scope()).or("/Madrid").build().unwrap_err();
    assert_eq!(combined.problems(), [Problem::AnyScopeWithAlternatives]);
}

#[test]
fn scope_patterns_alone() {
    assert_eq!("Madrid".parse::<ScopePattern>().unwrap().to_string(), "/Madrid");
    assert_eq!("/+".parse::<ScopePattern>().unwrap().levels(), [LevelPattern::AnyLevel]);
    assert_eq!("/Madrid/".parse::<ScopePattern>(), Err(ScopeError::EmptyLevel { position: 8 }));
}
