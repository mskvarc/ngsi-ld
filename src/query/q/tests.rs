#[cfg(feature = "linked-data")]
use crate::query::depth_check::assert_maps_to_itself;
use crate::{
    model::{Date, DateTime, EntityId, NameError, Time, Uri},
    query::{
        Expected,
        ParseError,
        Problem,
        RegularExpressionError,
        depth_check::assert_walks,
        q::{
            self,
            AttributePath,
            ComparableValue,
            Comparison,
            EqualityValue,
            Link,
            LinkedPath,
            Pattern,
            PatternError,
            Query,
            QueryTerm,
            UnquotedUri,
            ValuePath,
            builders::QueryBuilder,
        },
    },
};
use iri_rs::iri;

/// Builds a query the test expects to be valid.
fn build(builder: QueryBuilder) -> Query {
    builder.build().unwrap()
}

/// Parses a query the test expects to be valid.
fn parse(text: &str) -> Query {
    text.parse().unwrap()
}

/// Queries of every construct of the grammar, each with the string it prints.
fn printed_rows() -> Vec<(Query, &'static str)> {
    let downtown = EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1"));
    let christmas: DateTime = "2017-12-24T12:00:00Z".parse().unwrap();
    vec![
        (build(q::attr("temperature").exists()), "temperature"),
        (build(q::attr("temperature").eq(20)), "temperature==20"),
        (build(q::attr("brandName").ne("Mercedes")), r#"brandName!="Mercedes""#),
        (build(q::attr("isParked").eq(downtown)), r#"isParked=="urn:ngsi-ld:OffStreetParking:Downtown1""#),
        (
            build(q::attr("speed").gt(50).and(q::attr("brandName").ne("Mercedes"))),
            r#"speed>50;brandName!="Mercedes""#,
        ),
        (
            build(q::attr("speed").gt(50).or(q::attr("rpm").gt(3000)).and(q::attr("brandName").eq("Mercedes"))),
            r#"(speed>50|rpm>3000);brandName=="Mercedes""#,
        ),
        (
            build(q::attr("temperature").ge(20).and(q::attr("temperature").le(25)).or(q::attr("capacity").le(10))),
            "temperature>=20;temperature<=25|capacity<=10",
        ),
        (
            build(q::attr("temperature").sub_attr("observedAt").ge(christmas)),
            "temperature.observedAt>=2017-12-24T12:00:00Z",
        ),
        (build(q::attr("address").member("city").eq("Berlin")), r#"address[city]=="Berlin""#),
        (build(q::attr("sensor").linked("humidity").eq(40)), "sensor{humidity}==40"),
        (
            build(q::attr("sensor").linked_typed(["Device"], "humidity").eq(40)),
            "sensor{Device:humidity}==40",
        ),
        (build(q::attr("temperature").within(10, 20)), "temperature==10..20"),
        (build(q::attr("color").any_of(["black", "red"])), r#"color=="black","red""#),
        (build(q::attr("name").matches("^Mer(c|z)edes$")), "name~=^Mer(c|z)edes$"),
    ]
}

#[test]
fn q_01_existence_prints_the_attribute_alone() {
    assert_eq!(build(q::attr("temperature").exists()).to_string(), "temperature");
}

#[test]
fn q_02_equality_with_a_number() {
    assert_eq!(build(q::attr("temperature").eq(20)).to_string(), "temperature==20");
}

#[test]
fn q_03_inequality_with_a_quoted_string() {
    assert_eq!(build(q::attr("brandName").ne("Mercedes")).to_string(), r#"brandName!="Mercedes""#);
}

#[test]
fn q_04_entity_id_is_compared_as_a_quoted_string() {
    let downtown = EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1"));
    assert_eq!(
        build(q::attr("isParked").eq(downtown)).to_string(),
        r#"isParked=="urn:ngsi-ld:OffStreetParking:Downtown1""#
    );
}

#[test]
fn q_05_and_joins_terms_with_a_semicolon() {
    let query = build(q::attr("speed").gt(50).and(q::attr("brandName").ne("Mercedes")));
    assert_eq!(query.to_string(), r#"speed>50;brandName!="Mercedes""#);
}

#[test]
fn q_06_or_inside_and_is_parenthesised() {
    let query = build(q::attr("speed").gt(50).or(q::attr("rpm").gt(3000)).and(q::attr("brandName").eq("Mercedes")));
    assert_eq!(query.to_string(), r#"(speed>50|rpm>3000);brandName=="Mercedes""#);
}

#[test]
fn q_07_and_inside_or_needs_no_parentheses() {
    let query = build(q::attr("temperature").ge(20).and(q::attr("temperature").le(25)).or(q::attr("capacity").le(10)));
    assert_eq!(query.to_string(), "temperature>=20;temperature<=25|capacity<=10");
}

#[test]
fn q_08_sub_attribute_compared_with_a_date_time() {
    let christmas: DateTime = "2017-12-24T12:00:00Z".parse().unwrap();
    let query = build(q::attr("temperature").sub_attr("observedAt").ge(christmas));
    assert_eq!(query.to_string(), "temperature.observedAt>=2017-12-24T12:00:00Z");
}

#[test]
fn q_09_member_path_into_a_compound_value() {
    assert_eq!(build(q::attr("address").member("city").eq("Berlin")).to_string(), r#"address[city]=="Berlin""#);
}

#[test]
fn q_10_linked_entity_path() {
    assert_eq!(build(q::attr("sensor").linked("humidity").eq(40)).to_string(), "sensor{humidity}==40");
}

#[test]
fn q_11_linked_entity_path_with_a_type_hint() {
    assert_eq!(
        build(q::attr("sensor").linked_typed(["Device"], "humidity").eq(40)).to_string(),
        "sensor{Device:humidity}==40"
    );
}

#[test]
fn q_12_range() {
    assert_eq!(build(q::attr("temperature").within(10, 20)).to_string(), "temperature==10..20");
}

#[test]
fn q_13_value_list() {
    assert_eq!(build(q::attr("color").any_of(["black", "red"])).to_string(), r#"color=="black","red""#);
}

#[test]
fn q_14_pattern_is_printed_verbatim() {
    assert_eq!(build(q::attr("name").matches("^Mer(c|z)edes$")).to_string(), "name~=^Mer(c|z)edes$");
}

#[test]
fn q_15_every_printed_row_parses_back_to_its_tree() {
    for (query, printed) in printed_rows() {
        assert_eq!(query.to_string(), printed);
        assert_eq!(parse(printed), query, "{printed}");
    }
}

#[test]
fn q_16_trailing_and_reports_a_missing_query_term_at_its_offset() {
    let error = "speed>50;".parse::<Query>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::QueryTerm,
                position: 9
            }
        ),
        "{error:?}"
    );
}

#[test]
fn q_17_unclosed_parenthesis_is_unbalanced() {
    let error = "(speed>50".parse::<Query>().unwrap_err();
    assert!(matches!(error, ParseError::UnbalancedParenthesis { position: 0 }), "{error:?}");
}

#[test]
fn q_18_quotes_in_a_string_are_escaped() {
    let query = build(q::attr("brandName").eq(r#"say "hi""#));
    assert_eq!(query.to_string(), r#"brandName=="say \"hi\"""#);
    assert_eq!(parse(r#"brandName=="say \"hi\"""#), query);
}

#[test]
fn invalid_names_are_reported_not_panicked() {
    let error = q::attr("speed limit")
        .gt(50)
        .and(q::attr("brandName").sub_attr("9x").exists())
        .build()
        .unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [
                Problem::InvalidName {
                    source: NameError::InvalidCharacter { found: ' ', position: 5 }
                },
                Problem::InvalidName { .. }
            ]
        ),
        "{error:?}"
    );
}

#[test]
fn not_matches_prints_and_parses() {
    let query = build(q::attr("name").not_matches("^Mer"));
    assert_eq!(query.to_string(), "name!~=^Mer");
    assert_eq!(parse("name!~=^Mer"), query);
}

#[test]
fn any_language_member_path_parses_and_prints() {
    let query = build(q::attr("color").member("*").eq("red"));
    assert_eq!(query.to_string(), r#"color[*]=="red""#);
    assert_eq!(parse(r#"color[*]=="red""#), query);
}

#[test]
fn language_tag_member_path_parses() {
    let query = parse(r#"marque[en-GB]=="x""#);
    assert_eq!(query, build(q::attr("marque").member("en-GB").eq("x")));
    assert_eq!(query.to_string(), r#"marque[en-GB]=="x""#);
}

#[test]
fn dotted_member_path_of_example_10_round_trips() {
    let query = build(q::attr("sensor").sub_attr("rawdata").member("airquality").member("particulate").eq(40));
    assert_eq!(query.to_string(), "sensor.rawdata[airquality.particulate]==40");
    assert_eq!(parse("sensor.rawdata[airquality.particulate]==40"), query);
}

#[test]
fn time_and_date_values_round_trip() {
    let noon: Time = "12:00:00Z".parse().unwrap();
    let day: Date = "2018-12-04".parse().unwrap();
    let query = build(q::attr("opensAt").eq(noon).and(q::attr("openedOn").lt(day)));
    assert_eq!(query.to_string(), "opensAt==12:00:00Z;openedOn<2018-12-04");
    assert_eq!(parse("opensAt==12:00:00Z;openedOn<2018-12-04"), query);
}

#[test]
fn booleans_round_trip() {
    let query = build(q::attr("isParked").eq(true).or(q::attr("isMoving").ne(false)));
    assert_eq!(query.to_string(), "isParked==true|isMoving!=false");
    assert_eq!(parse("isParked==true|isMoving!=false"), query);
}

#[test]
fn unquoted_uri_parses_and_prints() {
    let query = parse("color==http://example.org/red;speed>5");
    let Query::And(operands) = &query else { panic!("{query:?}") };
    let Some(Query::Term(term)) = operands.as_slice().first() else {
        panic!("{query:?}")
    };
    let uri: Uri = "http://example.org/red".parse().unwrap();
    assert_eq!(
        term.comparison,
        Some(Comparison::Equal(EqualityValue::Uri(UnquotedUri::try_from(uri).unwrap())))
    );
    assert_eq!(query.to_string(), "color==http://example.org/red;speed>5");
}

#[test]
fn example_6_drops_the_redundant_outer_parentheses() {
    let query = parse(r#"((speed>50|rpm>3000);brandName=="Mercedes")"#);
    assert_eq!(
        query,
        build(q::attr("speed").gt(50).or(q::attr("rpm").gt(3000)).and(q::attr("brandName").eq("Mercedes")))
    );
    assert_eq!(query.to_string(), r#"(speed>50|rpm>3000);brandName=="Mercedes""#);
}

#[test]
fn example_7_parses_into_and_under_or() {
    let query = parse("(temperature>=20;temperature<=25)|capacity<=10");
    assert_eq!(
        query,
        build(q::attr("temperature").ge(20).and(q::attr("temperature").le(25)).or(q::attr("capacity").le(10)))
    );
    assert_eq!(query.to_string(), "temperature>=20;temperature<=25|capacity<=10");
}

#[test]
fn nested_groups_print_only_the_parentheses_they_need() {
    let inner = q::attr("a").exists().or(q::attr("b").exists()).and(q::attr("c").exists());
    let query = build(inner.or(q::attr("d").exists()).and(q::attr("e").exists()));
    assert_eq!(query.to_string(), "((a|b);c|d);e");
    assert_eq!(parse("((a|b);c|d);e"), query);
}

#[test]
fn ranges_of_negative_numbers_and_dates_round_trip() {
    let query = build(q::attr("level").within(-5, 5));
    assert_eq!(query.to_string(), "level==-5..5");
    assert_eq!(parse("level==-5..5"), query);
    let dates = parse("day==2018-01-01..2018-12-31");
    assert_eq!(dates.to_string(), "day==2018-01-01..2018-12-31");
}

#[test]
fn decimal_numbers_keep_their_lexical_form() {
    let query = parse("ratio>1.50e3");
    assert_eq!(query.to_string(), "ratio>1.50e3");
}

#[test]
fn invalid_date_time_is_a_temporal_error() {
    let error = "observed>=2017-13-24T12:00:00Z".parse::<Query>().unwrap_err();
    assert!(matches!(error, ParseError::InvalidTemporal { position: 10, .. }), "{error:?}");
}

#[test]
fn unquoted_text_is_not_a_value() {
    let error = "brandName==Mercedes".parse::<Query>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::Value,
                position: 11
            }
        ),
        "{error:?}"
    );
}

#[test]
fn unterminated_string_reports_the_missing_quote() {
    let error = r#"brandName=="Mercedes"#.parse::<Query>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::ClosingQuote,
                position: 20
            }
        ),
        "{error:?}"
    );
}

#[test]
fn raw_control_character_in_a_string_is_refused() {
    let error = "name==\"a\tb\"".parse::<Query>().unwrap_err();
    assert!(
        matches!(&error, ParseError::InvalidString { position: 6, source } if source.position() == 2),
        "{error:?}"
    );
}

#[test]
fn stray_closing_parenthesis_is_unbalanced() {
    let error = "speed>50)".parse::<Query>().unwrap_err();
    assert!(matches!(error, ParseError::UnbalancedParenthesis { position: 8 }), "{error:?}");
}

#[test]
fn link_from_a_dotted_path_is_a_problem() {
    let error = q::attr("a").sub_attr("b").linked("c").exists().build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::LinkFromCompoundPath { .. }]), "{error:?}");
}

#[test]
fn sub_attribute_after_a_member_is_a_problem() {
    let error = q::attr("address").member("city").sub_attr("observedAt").exists().build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::SubAttributeAfterMember { .. }]), "{error:?}");
}

#[test]
fn empty_value_list_is_a_problem_and_a_single_value_is_an_equality() {
    let error = q::attr("color").any_of(Vec::<&str>::new()).build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::EmptyValueList]), "{error:?}");
    assert_eq!(build(q::attr("color").any_of(["red"])), build(q::attr("color").eq("red")));
}

#[test]
fn pattern_refuses_what_a_query_would_split() {
    assert_eq!("Mercedes|BMW".parse::<Pattern>(), Err(PatternError::Delimiter { found: '|', position: 8 }));
    assert_eq!("(a".parse::<Pattern>(), Err(PatternError::Structure(RegularExpressionError::UnclosedGroup)));
    assert_eq!("[a".parse::<Pattern>(), Err(PatternError::Structure(RegularExpressionError::UnclosedBracket)));
    assert_eq!("a\\".parse::<Pattern>(), Err(PatternError::Structure(RegularExpressionError::DanglingEscape)));
    assert_eq!("".parse::<Pattern>(), Err(PatternError::Structure(RegularExpressionError::Empty)));
    assert!("[;|)]x".parse::<Pattern>().is_ok());
    assert!("[[:alpha:]]+".parse::<Pattern>().is_ok());
    assert!("a\\;b".parse::<Pattern>().is_ok());
}

#[test]
fn pattern_inside_a_group_round_trips() {
    let query = build(q::attr("name").matches("^(a|b)[)]$").or(q::attr("speed").gt(5)).and(q::attr("x").exists()));
    assert_eq!(query.to_string(), "(name~=^(a|b)[)]$|speed>5);x");
    assert_eq!(parse("(name~=^(a|b)[)]$|speed>5);x"), query);
}

#[test]
fn ordering_operators_take_only_comparable_values() {
    let error = "speed>true".parse::<Query>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::ComparableValue,
                position: 6
            }
        ),
        "{error:?}"
    );
    assert_eq!(build(q::attr("speed").gt(ComparableValue::from(5))).to_string(), "speed>5");
}

#[test]
fn a_pattern_starting_with_an_operator_character_round_trips() {
    for pattern in ["=x", "==x", "~y", ">5", "<a", "!z"] {
        for query in [build(q::attr("name").matches(pattern)), build(q::attr("name").not_matches(pattern))] {
            let printed = query.to_string();
            assert_eq!(printed.parse::<Query>().unwrap(), query, "{printed}");
        }
    }
}

#[test]
fn an_unknown_operator_is_reported_where_it_starts() {
    let error = "speed=>5".parse::<Query>().unwrap_err();
    assert_eq!(error.position(), 5);
}

#[test]
fn a_bracket_holding_its_own_bracket_or_a_class_prints_and_reads_back_the_same() {
    for text in ["[]a]", "[[:alpha:]]"] {
        let pattern: Pattern = text.parse().unwrap();
        assert_eq!(pattern.to_string(), text);
        assert_eq!(pattern.to_string().parse::<Pattern>(), Ok(pattern));
        let query = build(q::attr("name").matches(text));
        assert_eq!(parse(&query.to_string()), query, "{text}");
    }
}

/// How deep the depth tests nest: far deeper than any thread's stack holds, one frame per level.
const DEPTH: usize = 200_000;

/// Checks every walk of a deep query, with the mapping of names when JSON-LD is compiled in.
fn assert_deep_query_walks(query: &Query) {
    assert_walks(query);
    #[cfg(feature = "linked-data")]
    assert_maps_to_itself(query);
}

#[test]
fn a_group_nested_200000_levels_deep_reads_as_the_term_it_holds() {
    let text = format!("{}a==1{}", "(".repeat(DEPTH), ")".repeat(DEPTH));
    let query = parse(&text);
    assert_eq!(query, build(q::attr("a").eq(1)));
    assert_eq!(query.to_string(), "a==1");
}

#[test]
fn an_unclosed_group_200000_levels_deep_is_unbalanced() {
    let text = format!("{}a==1", "(".repeat(DEPTH));
    let error = text.parse::<Query>().unwrap_err();
    assert!(
        matches!(error, ParseError::UnbalancedParenthesis { position } if position == DEPTH - 1),
        "{error}"
    );
}

#[test]
fn and_and_or_alternating_200000_levels_deep_round_trip_and_walk() {
    // Each group holds an OR whose second operand is an AND ending with the next group: two levels per group.
    let groups = DEPTH / 2;
    let text = format!("{}a==1{}", "a==1;(a==1|".repeat(groups), ")".repeat(groups));
    let query = parse(&text);
    assert_eq!(query.to_string(), text);
    let built = (0..groups).fold(build(q::attr("a").eq(1)), |inner, _| {
        build(q::attr("a").eq(1)).and(build(q::attr("a").eq(1)).or(inner))
    });
    assert_eq!(built, query);
    assert_deep_query_walks(&query);
}

#[test]
fn a_path_through_200000_relationships_round_trips_and_walks() {
    let text = format!("{}b{}==1", "a{".repeat(DEPTH), "}".repeat(DEPTH));
    let query = parse(&text);
    assert_eq!(query.to_string(), text);
    let Query::Term(QueryTerm {
        path: AttributePath::Linked(path),
        ..
    }) = &query
    else {
        panic!("not a linked path")
    };
    assert_eq!(path.links().len(), DEPTH);
    let built = (0..DEPTH).fold(q::attr("a"), |path, _| path.linked("a"));
    assert_eq!(
        built.eq(1).build().unwrap().to_string(),
        format!("{}a{}==1", "a{".repeat(DEPTH), "}".repeat(DEPTH))
    );
    assert_deep_query_walks(&query);
}

#[test]
fn a_linked_path_built_around_a_linked_path_flattens_its_links() {
    let humidity = AttributePath::from(ValuePath::new("humidity".parse().unwrap()));
    let inner = LinkedPath::new(Link::new("sensor".parse().unwrap()).with_entity_types(["Device".parse().unwrap()]), humidity);
    let outer = LinkedPath::new(Link::new("building".parse().unwrap()), AttributePath::Linked(inner));
    assert_eq!(outer.to_string(), "building{sensor{Device:humidity}}");
    assert_eq!(outer.links().len(), 2);
    assert_eq!(outer.target().to_string(), "humidity");
    assert_eq!(
        parse("building{sensor{Device:humidity}}"),
        Query::Term(QueryTerm {
            path: AttributePath::Linked(outer),
            comparison: None
        })
    );
}

#[test]
fn an_unclosed_linked_path_reports_the_missing_brace() {
    let error = "a{b{c}".parse::<Query>().unwrap_err();
    assert!(
        matches!(
            error,
            ParseError::Unexpected {
                expected: Expected::ClosingBrace,
                position: 6
            }
        ),
        "{error:?}"
    );
}
