use crate::model::{AttributeName, EntityTypeName, Name, NameError, ReservedMember, Term, VocabTerm};
use pretty_assertions::assert_eq;

#[test]
fn attribute_name_accepts_a_term() {
    let name: AttributeName = "brandName".parse().unwrap();
    assert_eq!(name.to_string(), "brandName");
}

#[test]
fn attribute_name_accepts_a_prefixed_term() {
    let name: AttributeName = "ex:brandName".parse().unwrap();
    assert_eq!(name.to_string(), "ex:brandName");
}

#[test]
fn attribute_name_accepts_digits_and_underscore_after_the_first_letter() {
    let name: AttributeName = "temperature_2".parse().unwrap();
    assert_eq!(name.to_string(), "temperature_2");
}

#[test]
fn attribute_name_rejects_a_leading_digit() {
    assert_eq!("1speed".parse::<AttributeName>(), Err(NameError::InvalidStart { found: '1', position: 0 }));
}

#[test]
fn attribute_name_rejects_a_hyphen() {
    assert_eq!(
        "brand-name".parse::<AttributeName>(),
        Err(NameError::InvalidCharacter { found: '-', position: 5 })
    );
}

#[test]
fn attribute_name_accepts_an_absolute_iri() {
    let name: AttributeName = "https://example.org/brandName".parse().unwrap();
    assert_eq!(name.to_string(), "https://example.org/brandName");
}

#[test]
fn attribute_name_rejects_non_reified_core_members() {
    for (input, member) in [
        ("datasetId", ReservedMember::DatasetId),
        ("instanceId", ReservedMember::InstanceId),
        ("observedAt", ReservedMember::ObservedAt),
    ] {
        assert_eq!(input.parse::<AttributeName>(), Err(NameError::Reserved { member }), "{input}");
    }
}

#[test]
fn attribute_name_rejects_entity_members() {
    for (input, member) in [
        ("id", ReservedMember::Id),
        ("type", ReservedMember::Type),
        ("scope", ReservedMember::Scope),
        ("@context", ReservedMember::Context),
    ] {
        assert_eq!(input.parse::<AttributeName>(), Err(NameError::Reserved { member }), "{input}");
    }
}

#[test]
fn entity_type_name_accepts_a_term_and_an_iri() {
    let short: EntityTypeName = "Vehicle".parse().unwrap();
    let long: EntityTypeName = "https://example.org/Vehicle".parse().unwrap();
    assert_eq!(short.to_string(), "Vehicle");
    assert_eq!(long.to_string(), "https://example.org/Vehicle");
}

#[test]
fn every_name_returns_the_text_it_was_built_from_as_a_str() {
    let attribute: AttributeName = "ex:brandName".parse().unwrap();
    let entity_type: EntityTypeName = "https://example.org/Vehicle".parse().unwrap();
    let name: Name = "Vehicle".parse().unwrap();
    let term: Term = "Vehicle".parse().unwrap();
    assert_eq!(attribute.as_str(), "ex:brandName");
    assert_eq!(entity_type.as_str(), "https://example.org/Vehicle");
    assert_eq!(name.as_str(), "Vehicle");
    assert_eq!(term.as_str(), "Vehicle");
}

#[test]
fn name_accepts_unicode_letters_and_numbers() {
    let name: Name = "température²".parse().unwrap();
    assert_eq!(name.to_string(), "température²");
}

#[test]
fn name_rejects_an_empty_string_and_an_empty_local_part() {
    assert_eq!("".parse::<Name>(), Err(NameError::Empty));
    assert_eq!("ex:".parse::<Name>(), Err(NameError::Empty));
}

#[test]
fn name_reports_the_position_of_an_invalid_start_after_the_prefix() {
    assert_eq!("ex:1speed".parse::<Name>(), Err(NameError::InvalidStart { found: '1', position: 3 }));
}

#[test]
fn term_rejects_a_prefix() {
    assert_eq!("ex:speed".parse::<Term>(), Err(NameError::InvalidCharacter { found: ':', position: 2 }));
}

#[test]
fn vocab_term_accepts_hyphens_and_rejects_keywords_and_whitespace() {
    assert_eq!("non-commercial".parse::<VocabTerm>().unwrap().to_string(), "non-commercial");
    assert_eq!("@id".parse::<VocabTerm>(), Err(NameError::InvalidTermCharacter { found: '@', position: 0 }));
    assert_eq!("a b".parse::<VocabTerm>(), Err(NameError::InvalidTermCharacter { found: ' ', position: 1 }));
}

#[test]
fn vocab_term_checks_every_conversion_from_text() {
    assert_eq!(VocabTerm::try_new(""), Err(NameError::Empty));
    assert_eq!(VocabTerm::try_from("a\tb"), Err(NameError::InvalidTermCharacter { found: '\t', position: 1 }));
    assert_eq!(
        VocabTerm::try_from("@type".to_owned()),
        Err(NameError::InvalidTermCharacter { found: '@', position: 0 })
    );
    let term = VocabTerm::try_new("ex:term").unwrap();
    assert_eq!(term.as_ref(), "ex:term");
    assert_eq!(term.into_inner(), "ex:term");
}

#[test]
fn the_null_vocab_term_is_a_valid_term() {
    assert_eq!(Ok(VocabTerm::ngsi_ld_null()), "urn:ngsi-ld:null".parse::<VocabTerm>());
}

#[test]
fn vocab_terms_round_trip_through_serde_and_refuse_an_invalid_term() {
    let term: VocabTerm = jstrict::from_value(jstrict::json!("non-commercial")).unwrap();
    assert_eq!(jstrict::to_value(&term).unwrap(), jstrict::json!("non-commercial"));
    assert!(jstrict::from_value::<VocabTerm>(jstrict::json!("@id")).is_err());
}

#[test]
fn names_round_trip_through_serde() {
    let name: AttributeName = jstrict::from_value(jstrict::json!("brandName")).unwrap();
    assert_eq!(jstrict::to_value(&name).unwrap(), jstrict::json!("brandName"));
    assert!(jstrict::from_value::<AttributeName>(jstrict::json!("datasetId")).is_err());
}

#[test]
fn every_kind_of_entity_type_name_converts_into_the_term_with_the_same_text() {
    for text in ["Vehicle", "ex:Vehicle", "Fahrzeug2", "https://example.org/Vehicle", "urn:ngsi-ld:Vehicle"] {
        let name: EntityTypeName = text.parse().unwrap();
        let expected: VocabTerm = text.parse().unwrap();
        assert_eq!(VocabTerm::from(&name), expected, "{text}");
        assert_eq!(VocabTerm::from(name), expected, "{text}");
    }
}
