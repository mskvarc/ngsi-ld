use crate::model::RawJson;
use pretty_assertions::assert_eq;
use std::hash::{BuildHasher, RandomState};

fn raw(text: &str) -> RawJson {
    text.parse().unwrap()
}

#[test]
fn raw_json_compares_in_member_order_and_hashes_consistently() {
    let state = RandomState::new();
    let json = raw(r#"{"a":[1,{"b":null}],"c":"x"}"#);
    let copy = json.clone();
    assert_eq!(copy, json);
    assert_eq!(state.hash_one(&copy), state.hash_one(&json));
    // Raw JSON is carried verbatim, so member order and number spelling count.
    assert_ne!(raw(r#"{"a":1,"b":2}"#), raw(r#"{"b":2,"a":1}"#));
    assert_ne!(raw("1.0"), raw("1"));
    assert_ne!(raw("[1]"), raw(r#"{"0":1}"#));
    assert_eq!(format!("{json:?}"), r#"RawJson({"a":[1,{"b":null}],"c":"x"})"#);
}

#[test]
fn raw_json_nested_200_000_levels_deep_is_copied_compared_and_hashed() {
    let depth = 100_000;
    let text = format!("{}1{}", r#"{"a":["#.repeat(depth), "]}".repeat(depth));
    let json = raw(&text);
    let copy = json.clone();
    assert_eq!(copy, json);
    let state = RandomState::new();
    assert_eq!(state.hash_one(&copy), state.hash_one(&json));
    assert_eq!(copy.to_string(), text);
}
