use crate::model::{LanguageKey, LanguageMap, LanguageMapError, LanguageTagError, LanguageTagInput, LanguageText};
use jstrict::Value as Json;
use langtag::LangTagBuf;
use pretty_assertions::assert_eq;
use std::borrow::Cow;

fn parse(json: &str) -> Result<LanguageMap, LanguageMapError> {
    LanguageMap::from_json(json.parse::<Json>().unwrap())
}

#[test]
fn a_language_map_keeps_its_order() {
    let map = parse(r#"{"fr":"Grand Place","nl":"Grote Markt"}"#).unwrap();
    assert_eq!(map.to_string(), r#"{"fr":"Grand Place","nl":"Grote Markt"}"#);
    let keys: Vec<String> = map.iter().map(|(key, _)| key.to_string()).collect();
    assert_eq!(keys, ["fr", "nl"]);
}

#[test]
fn a_language_map_accepts_the_default_key_and_arrays() {
    let map = parse(r#"{"@none":"x","en":["a","b"]}"#).unwrap();
    assert_eq!(map.get(&LanguageKey::Default), Some(&LanguageText::Single("x".to_owned())));
    assert_eq!(map.to_string(), r#"{"@none":"x","en":["a","b"]}"#);
}

#[test]
fn a_language_map_rejects_an_invalid_tag() {
    assert!(matches!(
        parse(r#"{"not a tag":"x"}"#),
        Err(LanguageMapError::InvalidTag {
            source: LanguageTagError::InvalidCharacter { found: ' ', position: 3 }
        })
    ));
}

#[test]
fn a_language_map_rejects_an_empty_object() {
    assert!(matches!(parse("{}"), Err(LanguageMapError::Empty)));
}

#[test]
fn a_language_map_rejects_a_repeated_language_and_non_text_values() {
    assert!(matches!(parse(r#"{"en":"a","en":"b"}"#), Err(LanguageMapError::DuplicateLanguage { .. })));
    assert!(matches!(parse(r#"{"en":1}"#), Err(LanguageMapError::NotText)));
    assert!(matches!(parse(r#"["en"]"#), Err(LanguageMapError::NotAnObject)));
}

#[test]
fn the_ngsi_ld_null_language_map_is_the_null_under_none() {
    assert_eq!(LanguageMap::ngsi_ld_null(), parse(r#"{"@none":"urn:ngsi-ld:null"}"#).unwrap());
}

#[test]
fn a_language_tag_input_borrows_a_parsed_tag_and_parses_text() {
    let french: LangTagBuf = "fr".parse().unwrap();
    assert!(matches!(LanguageTagInput::from(&french).resolve(), Ok(Cow::Borrowed(tag)) if tag == &french));
    assert!(matches!(LanguageTagInput::from("fr").resolve(), Ok(Cow::Owned(tag)) if tag == french));
    assert_eq!(
        LanguageTagInput::from("not a tag").resolve(),
        Err(LanguageTagError::InvalidCharacter { found: ' ', position: 3 })
    );
}
