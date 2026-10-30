use crate::model::{Date, DateTime, TemporalError, TemporalToken, Time};
#[cfg(feature = "chrono")]
use chrono::{NaiveDate, NaiveTime, Utc};
use jiff::Timestamp;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;
use std::{cmp::Ordering, collections::HashSet};

#[test]
fn date_time_accepts_whole_seconds() {
    assert_eq!("2017-07-29T12:00:04Z".parse::<DateTime>().unwrap().to_string(), "2017-07-29T12:00:04Z");
}

#[test]
fn date_time_keeps_six_fraction_digits() {
    assert_eq!(
        "2017-07-29T12:00:04.123456Z".parse::<DateTime>().unwrap().to_string(),
        "2017-07-29T12:00:04.123456Z"
    );
}

#[test]
fn date_time_keeps_trailing_zero_fraction_digits() {
    assert_eq!("2022-03-14T12:51:02.000Z".parse::<DateTime>().unwrap().to_string(), "2022-03-14T12:51:02.000Z");
}

#[test]
fn date_time_rejects_seven_fraction_digits() {
    assert!(matches!(
        "2017-07-29T12:00:04.1234567Z".parse::<DateTime>(),
        Err(TemporalError::TooManyFractionDigits { position: 20 })
    ));
}

#[test]
fn date_time_rejects_an_offset() {
    assert!(matches!(
        "2017-07-29T12:00:04+01:00".parse::<DateTime>(),
        Err(TemporalError::NotUtc { position: 19 })
    ));
}

#[test]
fn date_time_reads_a_comma_and_writes_a_point() {
    assert_eq!("2017-07-29T12:00:04,5Z".parse::<DateTime>().unwrap().to_string(), "2017-07-29T12:00:04.5Z");
}

#[test]
fn date_time_rejects_a_reduced_representation() {
    assert!(matches!(
        "2017-07-29T12:00Z".parse::<DateTime>(),
        Err(TemporalError::Unexpected {
            expected: TemporalToken::Character(':'),
            position: 16
        })
    ));
}

#[test]
fn date_time_rejects_lower_case_designators_and_trailing_input() {
    assert!(matches!(
        "2017-07-29t12:00:04Z".parse::<DateTime>(),
        Err(TemporalError::Unexpected {
            expected: TemporalToken::Character('T'),
            position: 10
        })
    ));
    assert!(matches!(
        "2017-07-29T12:00:04Zx".parse::<DateTime>(),
        Err(TemporalError::Unexpected {
            expected: TemporalToken::End,
            position: 20
        })
    ));
}

#[test]
fn date_time_rejects_an_impossible_calendar_date() {
    assert!(matches!("2017-02-30T12:00:04Z".parse::<DateTime>(), Err(TemporalError::OutOfRange { .. })));
}

#[test]
fn date_time_from_a_timestamp_uses_the_fewest_exact_digits() {
    let timestamp = Timestamp::new(1_500_000_000, 120_000_000).unwrap();
    assert_eq!(DateTime::try_from(timestamp).unwrap().to_string(), "2017-07-14T02:40:00.12Z");
    let nanos = Timestamp::new(1_500_000_000, 1).unwrap();
    assert!(matches!(DateTime::try_from(nanos), Err(TemporalError::SubMicrosecond)));
}

#[test]
fn now_has_millisecond_precision() {
    assert_eq!(DateTime::now().fraction_digits(), 3);
}

#[test]
fn date_accepts_a_complete_date_and_rejects_a_reduced_one() {
    assert_eq!("2017-07-29".parse::<Date>().unwrap().to_string(), "2017-07-29");
    assert!(matches!(
        "2017-07".parse::<Date>(),
        Err(TemporalError::Unexpected {
            expected: TemporalToken::Character('-'),
            position: 7
        })
    ));
}

#[test]
fn time_accepts_utc_and_rejects_an_offset() {
    assert_eq!("12:00:04Z".parse::<Time>().unwrap().to_string(), "12:00:04Z");
    assert!(matches!("12:00:04+01:00".parse::<Time>(), Err(TemporalError::NotUtc { position: 8 })));
}

#[test]
fn temporal_values_round_trip_through_serde() {
    let value = Json::from("2018-12-04T12:00:00Z");
    let parsed: DateTime = jstrict::from_value(value.clone()).unwrap();
    assert_eq!(jstrict::to_value(parsed).unwrap(), value);
    assert!(jstrict::from_value::<Date>(Json::from("2018-12")).is_err());
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_conversions_round_trip() {
    let value: DateTime = "2018-12-04T12:00:00.25Z".parse().unwrap();
    let converted = chrono::DateTime::<Utc>::from(value);
    assert_eq!(DateTime::try_from(converted).unwrap(), value);
    let date: Date = "2018-12-04".parse().unwrap();
    assert_eq!(Date::try_from(NaiveDate::from(date)).unwrap(), date);
    let time: Time = "12:00:04.5Z".parse().unwrap();
    assert_eq!(Time::try_from(NaiveTime::from(time)).unwrap(), time);
}

#[test]
fn a_verbatim_datetime_writes_its_comma_fraction_back() {
    let verbatim = DateTime::verbatim("2017-07-29T12:00:04,5Z").unwrap();
    assert_eq!(verbatim.to_string(), "2017-07-29T12:00:04,5Z");
    assert_eq!("2017-07-29T12:00:04,5Z".parse::<DateTime>().unwrap().to_string(), "2017-07-29T12:00:04.5Z");
}

#[test]
fn a_verbatim_datetime_equals_the_same_instant_written_with_a_point() {
    let comma = DateTime::verbatim("2017-07-29T12:00:04,5Z").unwrap();
    let point: DateTime = "2017-07-29T12:00:04.5Z".parse().unwrap();
    assert_eq!(comma, point);
    assert_eq!(comma.cmp(&point), Ordering::Equal);
    assert_eq!(HashSet::from([comma, point]).len(), 1);
}

#[test]
fn a_verbatim_time_keeps_a_comma_fraction() {
    let comma = Time::verbatim("12:00:04,250Z").unwrap();
    assert_eq!(comma.to_string(), "12:00:04,250Z");
    assert_eq!(comma, "12:00:04.250Z".parse().unwrap());
    assert!(Time::verbatim("12:00:04,1234567Z").is_err());
    assert!(Time::verbatim("12:00:04,5+01:00").is_err());
}
