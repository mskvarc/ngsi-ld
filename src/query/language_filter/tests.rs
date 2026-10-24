use crate::{
    model::LanguageTagError,
    query::{
        Expected,
        ParseError,
        Problem,
        Quality,
        QualityError,
        language_filter::{self, LanguageFilter, LanguageRange},
    },
};
use pretty_assertions::assert_eq;

/// The quality `thousandths` / 1000.
fn quality(thousandths: u16) -> Quality {
    Quality::try_from_thousandths(thousandths).unwrap()
}

/// Parses `text`, checks it prints back unchanged, and returns the filter.
fn round_trip(text: &str) -> LanguageFilter {
    let filter: LanguageFilter = text.parse().unwrap();
    assert_eq!(filter.to_string(), text);
    filter
}

#[test]
fn l_01_one_language() {
    let filter = round_trip("en");
    assert_eq!(filter, language_filter::language("en").build().unwrap());
    assert!(matches!(filter.preferences(), [preference] if preference.quality().is_none()));
}

#[test]
fn l_02_several_languages_without_ranking() {
    let filter = round_trip("fr-CH,fr");
    assert_eq!(filter, language_filter::language("fr-CH").language("fr").build().unwrap());
}

#[test]
fn l_03_the_wildcard() {
    let filter = round_trip("*");
    assert_eq!(filter, language_filter::any().build().unwrap());
    assert!(matches!(filter.preferences(), [preference] if *preference.value() == LanguageRange::Any));
}

#[test]
fn l_04_quality_values_rank_the_languages() {
    let filter = round_trip("fr-CH,fr;q=0.9,en;q=0.8,*;q=0.5");
    let built = language_filter::language("fr-CH")
        .weighted_language("fr", quality(900))
        .weighted_language("en", quality(800))
        .weighted_any(quality(500))
        .build()
        .unwrap();
    assert_eq!(filter, built);
    let weights: Vec<Option<u16>> = filter
        .preferences()
        .iter()
        .map(|preference| preference.quality().map(Quality::thousandths))
        .collect();
    assert_eq!(weights, [None, Some(900), Some(800), Some(500)]);
}

#[test]
fn l_05_quality_values_print_in_their_shortest_form() {
    let filter: LanguageFilter = "en;Q=0.800,de;q=1.000,nl;q=0".parse().unwrap();
    assert_eq!(filter.to_string(), "en;q=0.8,de;q=1,nl;q=0");
    for (text, thousandths) in [("1", 1000), ("1.", 1000), ("0.005", 5), ("0.85", 850), ("0", 0)] {
        assert_eq!(text.parse::<Quality>().map(Quality::thousandths), Ok(thousandths), "{text}");
    }
}

#[test]
fn l_06_a_quality_above_one_or_with_four_decimals_is_rejected() {
    for (text, error) in [
        ("1.5", QualityError::AboveOne),
        ("1.001", QualityError::AboveOne),
        ("2", QualityError::AboveOne),
        ("0.1234", QualityError::TooManyDecimals),
        ("1.0000", QualityError::TooManyDecimals),
        ("x", QualityError::Syntax),
        ("", QualityError::Syntax),
        (".5", QualityError::Syntax),
        ("0.a", QualityError::Syntax),
        ("00.5", QualityError::Syntax),
        ("01", QualityError::Syntax),
    ] {
        assert_eq!(text.parse::<Quality>(), Err(error), "{text}");
    }
    assert_eq!(Quality::try_from_thousandths(1001), Err(QualityError::AboveOne));
    assert!(matches!(
        "en;q=1.5".parse::<LanguageFilter>(),
        Err(ParseError::InvalidQuality { position: 5, .. })
    ));
}

#[test]
fn l_07_an_invalid_language_tag_is_rejected() {
    assert!(matches!(
        "en,not a tag".parse::<LanguageFilter>(),
        Err(ParseError::InvalidLanguageTag { position: 3, .. })
    ));
    let error = language_filter::language("not a tag")
        .weighted_language("en-", quality(500))
        .build()
        .unwrap_err();
    assert_eq!(
        error.problems(),
        [
            Problem::InvalidLanguageTag {
                source: LanguageTagError::InvalidCharacter { found: ' ', position: 3 }
            },
            Problem::InvalidLanguageTag {
                source: LanguageTagError::Malformed
            }
        ]
    );
}

#[test]
fn l_08_empty_parts_are_rejected() {
    assert!(matches!(
        "".parse::<LanguageFilter>(),
        Err(ParseError::Unexpected {
            expected: Expected::LanguageRange,
            position: 0
        })
    ));
    assert!(matches!(
        "en,".parse::<LanguageFilter>(),
        Err(ParseError::Unexpected {
            expected: Expected::LanguageRange,
            position: 3
        })
    ));
    assert!(matches!(
        "en;x=1".parse::<LanguageFilter>(),
        Err(ParseError::Unexpected {
            expected: Expected::Quality,
            position: 3
        })
    ));
}

#[test]
fn l_07_an_empty_or_malformed_language_tag_is_classified() {
    for (tag, error) in [
        ("", LanguageTagError::Empty),
        ("en-", LanguageTagError::Malformed),
        ("not a tag", LanguageTagError::InvalidCharacter { found: ' ', position: 3 }),
    ] {
        assert_eq!(
            language_filter::language(tag).build().unwrap_err().problems(),
            [Problem::InvalidLanguageTag { source: error }],
            "{tag}"
        );
    }
}

#[test]
fn l_09_a_basic_language_range_that_is_no_language_tag_is_rejected() {
    // Clause 4.15 takes RFC 5646 tags; `en-a-b` is an RFC 4647 basic range but its extension has no 2-8 character
    // subtag.
    assert!(matches!(
        "en-a-b".parse::<LanguageFilter>(),
        Err(ParseError::InvalidLanguageTag { position: 0, .. })
    ));
    assert_eq!(
        language_filter::language("en-a-b").build().unwrap_err().problems(),
        [Problem::InvalidLanguageTag {
            source: LanguageTagError::Malformed
        }]
    );
}
