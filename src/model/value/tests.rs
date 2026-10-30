use crate::model::{CoreDatatype, Datatype, DateTime, NotFinite, Number, NumberError, TypedValue, Value, ValueError, VocabTerm};
use indexmap::IndexMap;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;

fn parse(json: &str) -> Result<Value, ValueError> {
    Value::from_json(json.parse::<Json>().unwrap())
}

#[test]
fn a_number_keeps_its_lexical_form() {
    let value = parse("1.10").unwrap();
    assert_eq!(value.to_string(), "1.10");
}

#[test]
fn a_json_ld_value_object_is_a_typed_value_and_round_trips() {
    let json = r#"{"@type":"DateTime","@value":"2018-12-04T12:00:00Z"}"#;
    let value = parse(json).unwrap();
    assert_eq!(
        value,
        Value::Typed(TypedValue::try_new(CoreDatatype::DateTime, "2018-12-04T12:00:00Z").unwrap())
    );
    assert_eq!(value.to_string(), json);
}

#[test]
fn a_date_time_converts_into_the_typed_value_of_clause_c_6() {
    let observed: DateTime = "2018-12-04T12:00:00Z".parse().unwrap();
    assert_eq!(Value::from(observed).to_string(), r#"{"@type":"DateTime","@value":"2018-12-04T12:00:00Z"}"#);
}

#[test]
fn a_boolean_converts_into_a_boolean_value() {
    assert_eq!(Value::from(true), Value::Boolean(true));
    assert_eq!(Value::from(false).to_string(), "false");
    assert_eq!(Value::from(vec![true, false]).to_string(), "[true,false]");
}

#[test]
fn the_other_variants_convert_from_their_payload() {
    let typed = TypedValue::try_new(CoreDatatype::DateTime, "2018-12-04T12:00:00Z").unwrap();
    assert_eq!(Value::from(typed.clone()), Value::Typed(typed));
    assert_eq!(Value::from(Number::from(80)), Value::Number(Number::from(80)));
    let members = IndexMap::from([("city".to_owned(), Value::from("Berlin"))]);
    assert_eq!(Value::from(members).to_string(), r#"{"city":"Berlin"}"#);
}

#[test]
fn an_object_keeps_its_member_order() {
    let json = r#"{"street":"Pariser Platz","city":"Berlin","country":"Germany"}"#;
    assert_eq!(parse(json).unwrap().to_string(), json);
}

#[test]
fn null_is_rejected_at_any_depth() {
    assert!(matches!(parse("null"), Err(ValueError::Null)));
    assert!(matches!(parse(r#"{"a":[1,null]}"#), Err(ValueError::Null)));
}

#[test]
fn a_repeated_member_is_rejected() {
    assert!(matches!(parse(r#"{"a":1,"a":2}"#), Err(ValueError::DuplicateMember)));
}

#[test]
fn an_object_with_other_keyword_members_is_an_ordinary_object() {
    let value = parse(r#"{"@type":"DateTime","@value":"x","extra":1}"#).unwrap();
    assert!(value.is_object());
}

#[test]
fn numbers_convert_from_integers_and_finite_floats() {
    assert_eq!(Number::from(80).to_string(), "80");
    assert_eq!(Number::try_from(f64::NAN), Err(NotFinite::Nan));
}

#[test]
fn a_parsed_number_keeps_every_digit_of_its_lexical_form() {
    for text in [
        "0.1000000000000000055511151231257827",
        "18446744073709551616",
        "-9223372036854775809",
        "1.10",
        "6.02214076e23",
        "-0",
        "1E-400",
    ] {
        let number: Number = text.parse().unwrap();
        assert_eq!(number.as_str(), text);
        assert_eq!(Value::Number(number).to_string(), text);
    }
}

#[test]
fn a_parsed_number_beyond_the_range_of_an_i64_has_no_i64() {
    let number: Number = "18446744073709551616".parse().unwrap();
    assert_eq!(number.as_i64(), None);
}

#[test]
fn text_that_is_not_a_json_number_is_refused() {
    for text in ["", "01", "1.", ".5", "+1", "1e", "NaN", "Infinity", "0x10", " 1", "1 "] {
        assert_eq!(text.parse::<Number>(), Err(NumberError), "{text:?}");
    }
}

#[test]
fn a_number_that_is_not_finite_is_reported_by_its_class() {
    assert_eq!(Number::try_from(f64::INFINITY), Err(NotFinite::PositiveInfinity));
    assert_eq!(Number::try_from(f64::NEG_INFINITY), Err(NotFinite::NegativeInfinity));
    assert_eq!(NotFinite::Nan.to_string(), "NaN cannot be written as a JSON number");
    assert_eq!(NotFinite::PositiveInfinity.to_string(), "inf cannot be written as a JSON number");
    assert_eq!(NotFinite::NegativeInfinity.to_string(), "-inf cannot be written as a JSON number");
}

#[test]
fn a_core_typed_value_holds_a_lexical_form_of_its_datatype() {
    for (datatype, lexical) in [
        (CoreDatatype::DateTime, "2018-12-04T12:00:00Z"),
        (CoreDatatype::Date, "2018-12-04"),
        (CoreDatatype::Time, "12:00:00Z"),
    ] {
        let typed = TypedValue::try_new(datatype, lexical).unwrap();
        assert_eq!((typed.datatype(), typed.lexical()), (&Datatype::Core(datatype), lexical));
        let error = TypedValue::try_new(datatype, "yesterday").unwrap_err();
        assert_eq!(error.datatype, datatype);
    }
}

#[test]
fn a_datatype_term_naming_a_core_datatype_is_that_datatype() {
    let term: VocabTerm = "Date".parse().unwrap();
    assert_eq!(Datatype::from(term.clone()), Datatype::Core(CoreDatatype::Date));
    assert!(TypedValue::try_new(Datatype::Other(term), "not a date").is_err());
    let other: VocabTerm = "xsd:gYear".parse().unwrap();
    let typed = TypedValue::try_new(other.clone(), "2018").unwrap();
    assert_eq!(typed.datatype(), &Datatype::Other(other));
}

#[test]
fn a_value_object_naming_a_core_datatype_with_an_invalid_lexical_form_is_refused() {
    assert!(matches!(
        parse(r#"{"@type":"DateTime","@value":"yesterday"}"#),
        Err(ValueError::Typed { source }) if source.datatype == CoreDatatype::DateTime
    ));
}

#[test]
fn the_written_out_walks_keep_the_semantics_of_a_derive() {
    let value = parse(r#"{"a":[1,"x",true,{"@type":"Date","@value":"2018-12-04"}],"b":{"c":2}}"#).unwrap();
    let copy = value.clone();
    assert_eq!(copy, value);
    assert_eq!(copy.to_string(), value.to_string());
    // Objects compare as maps, whatever their member order; arrays compare in order.
    assert_eq!(parse(r#"{"a":1,"b":2}"#).unwrap(), parse(r#"{"b":2,"a":1}"#).unwrap());
    assert_ne!(parse("[1,2]").unwrap(), parse("[2,1]").unwrap());
    assert_ne!(parse("1").unwrap(), parse(r#""1""#).unwrap());
    assert_eq!(format!("{:?}", Value::from("x")), r#"Text("x")"#);
    assert_eq!(format!("{:?}", Value::Array(vec![Value::Boolean(true)])), "Array([Boolean(true)])");
}

#[test]
fn a_value_nested_100_000_levels_deep_is_read_written_copied_compared_and_dropped() {
    let depth = 100_000;
    let text = format!("{}1{}", r#"{"a":["#.repeat(depth), "]}".repeat(depth));
    let value = parse(&text).unwrap();
    let copy = value.clone();
    assert_eq!(copy, value);
    assert_eq!(copy.to_string(), text);
    assert!(format!("{copy:?}").starts_with("Object"));
}

#[test]
fn the_json_null_literal_is_a_value_and_round_trips() {
    let literal = r#"{"@type":"@json","@value":null}"#;
    let value = parse(literal).unwrap();
    assert_eq!(value, Value::JsonNull);
    assert!(value.is_json_null());
    assert_eq!(value.to_string(), literal);
    assert_eq!(parse(r#"{"@value":null,"@type":"@json"}"#).unwrap(), Value::JsonNull);
}

#[test]
fn a_bare_null_and_a_null_beside_other_members_stay_refused() {
    assert!(matches!(parse("null"), Err(ValueError::Null)));
    assert!(matches!(parse(r#"{"@type":"@json","@value":null,"other":1}"#), Err(ValueError::Null)));
    assert!(matches!(parse(r#"{"@type":"Other","@value":null}"#), Err(ValueError::Null)));
}

#[test]
fn the_json_null_literal_nests_in_an_array_and_an_object() {
    let literal = r#"{"@type":"@json","@value":null}"#;
    let json = format!(r#"{{"a":[{literal},1]}}"#);
    assert_eq!(parse(&json).unwrap().to_string(), json);
}
