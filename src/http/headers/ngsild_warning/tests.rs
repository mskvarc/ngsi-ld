use crate::http::headers::{InvalidFieldValue, NgsiLdWarning, NgsiLdWarnings, OptionalHeader, WarningCode, ngsild_warning::combine};
use headers::Header;
use http::HeaderValue;
use pretty_assertions::assert_eq;
use std::iter;

fn decode(lines: &[&'static str]) -> Option<Vec<(WarningCode, String)>> {
    let values: Vec<HeaderValue> = lines.iter().copied().map(HeaderValue::from_static).collect();
    NgsiLdWarnings::decode(&mut values.iter())
        .ok()
        .map(|warnings| warnings.iter().map(|warning| (warning.code(), warning.text().to_owned())).collect())
}

fn encoded(warnings: &NgsiLdWarnings) -> Vec<HeaderValue> {
    let mut lines = Vec::new();
    warnings.encode(&mut lines);
    lines
}

#[test]
fn wn_01_a_warning_as_clause_6_3_17_writes_it_decodes_and_encodes_back() {
    assert_eq!(
        decode(&["110 - Response is Stale"]),
        Some(vec![(WarningCode::ResponseIsStale, "- Response is Stale".to_owned())])
    );
    let line = HeaderValue::from_static("110 - Response is Stale");
    let warnings = NgsiLdWarnings::decode(&mut [&line].into_iter()).unwrap();
    assert_eq!(encoded(&warnings), vec![line]);
}

#[test]
fn wn_02_warnings_on_several_lines_are_kept_in_order() {
    assert_eq!(
        decode(&["110 - Response is Stale", "199 - Miscellaneous Warning"]),
        Some(vec![
            (WarningCode::ResponseIsStale, "- Response is Stale".to_owned()),
            (WarningCode::MiscellaneousWarning, "- Miscellaneous Warning".to_owned()),
        ])
    );
}

#[test]
fn wn_03_a_comma_starts_a_new_warning_only_before_a_code() {
    assert_eq!(
        decode(&["110 - Response is Stale, 299 - Miscellaneous Persistent Warning"]),
        Some(vec![
            (WarningCode::ResponseIsStale, "- Response is Stale".to_owned()),
            (WarningCode::MiscellaneousPersistentWarning, "- Miscellaneous Persistent Warning".to_owned()),
        ])
    );
    assert_eq!(
        decode(&["199 - timeout, retry later"]),
        Some(vec![(WarningCode::MiscellaneousWarning, "- timeout, retry later".to_owned())])
    );
    assert_eq!(decode(&["199 - waited 300 ms, 111 - bad payload"]).map(|warnings| warnings.len()), Some(2));
}

#[test]
fn wn_04_an_agent_and_quoted_text_or_the_code_alone_are_read() {
    assert_eq!(
        decode(&["110 broker-1 \"Response is Stale\""]),
        Some(vec![(WarningCode::ResponseIsStale, "broker-1 \"Response is Stale\"".to_owned())])
    );
    assert_eq!(decode(&[" 111 "]), Some(vec![(WarningCode::RevalidationFailed, String::new())]));
}

#[test]
fn wn_05_a_field_holding_an_unreadable_warning_does_not_decode() {
    for line in ["", "abc", "1100 - x", "112 - Unknown", "11 - short", "110- x", "110 - stale, 112 - unknown"] {
        assert_eq!(decode(&[line]), None, "{line}");
    }
    assert_eq!(decode(&["110 - Response is Stale", "abc"]), None);
    assert!(NgsiLdWarnings::decode(&mut iter::empty()).is_err());
}

#[test]
fn wn_06_each_code_encodes_as_clause_6_3_17_writes_it() {
    let warnings = NgsiLdWarnings::new(NgsiLdWarning::new(WarningCode::ResponseIsStale))
        .and(NgsiLdWarning::new(WarningCode::RevalidationFailed))
        .and(NgsiLdWarning::new(WarningCode::MiscellaneousWarning))
        .and(NgsiLdWarning::new(WarningCode::MiscellaneousPersistentWarning));
    assert_eq!(
        encoded(&warnings),
        vec![
            HeaderValue::from_static("110 - Response is Stale"),
            HeaderValue::from_static("111 - Revalidation Failed"),
            HeaderValue::from_static("199 - Miscellaneous Warning"),
            HeaderValue::from_static("299 - Miscellaneous Persistent Warning"),
        ]
    );
    assert_eq!(NgsiLdWarnings::decode(&mut encoded(&warnings).iter()).ok(), Some(warnings));
    assert_eq!(
        NgsiLdWarning::try_with_text(WarningCode::MiscellaneousWarning, "broker-1 \"took 5 s\"")
            .map(|warning| warning.text().to_owned())
            .ok(),
        Some("broker-1 \"took 5 s\"".to_owned())
    );
    assert!(matches!(
        NgsiLdWarning::try_with_text(WarningCode::MiscellaneousWarning, "caf\u{e9}"),
        Err(InvalidFieldValue::NonAscii { position: 7 })
    ));
}

#[test]
fn wn_07_the_warnings_of_two_answers_are_combined_without_losing_a_line() {
    let stale = || NgsiLdWarnings::new(NgsiLdWarning::new(WarningCode::ResponseIsStale));
    let failed = || NgsiLdWarnings::new(NgsiLdWarning::new(WarningCode::RevalidationFailed));
    let garbage = || OptionalHeader::<NgsiLdWarnings>::Malformed(vec![HeaderValue::from_static("garbage")]);
    assert_eq!(combine(OptionalHeader::Absent, OptionalHeader::Absent), OptionalHeader::Absent);
    assert_eq!(combine(OptionalHeader::Absent, OptionalHeader::Valid(stale())), OptionalHeader::Valid(stale()));
    assert_eq!(combine(OptionalHeader::Valid(stale()), OptionalHeader::Absent), OptionalHeader::Valid(stale()));
    assert_eq!(
        combine(OptionalHeader::Valid(stale()), OptionalHeader::Valid(failed())),
        OptionalHeader::Valid(stale().and(NgsiLdWarning::new(WarningCode::RevalidationFailed)))
    );
    assert_eq!(
        combine(OptionalHeader::Valid(stale()), garbage()),
        OptionalHeader::Malformed(vec![HeaderValue::from_static("110 - Response is Stale"), HeaderValue::from_static("garbage")])
    );
    assert_eq!(
        combine(garbage(), OptionalHeader::Valid(failed())),
        OptionalHeader::Malformed(vec![HeaderValue::from_static("garbage"), HeaderValue::from_static("111 - Revalidation Failed")])
    );
    assert_eq!(
        combine(garbage(), garbage()),
        OptionalHeader::Malformed(vec![HeaderValue::from_static("garbage"), HeaderValue::from_static("garbage")])
    );
}
