//! Conversions between the clause 4.6.3 types and `chrono` (behind the `chrono` feature). No derive converts between
//! two crates' representations of a time, so each conversion is written out.

use crate::model::{Date, DateTime, TemporalError, Time};
use chrono::{NaiveDate, NaiveTime, Utc};
use jiff::{Timestamp, civil};

// Written out: see the module documentation.
impl From<DateTime> for chrono::DateTime<Utc> {
    fn from(value: DateTime) -> Self {
        let timestamp = value.timestamp();
        // Every instant `jiff` holds lies within chrono's range, so the fallback is never taken.
        Self::from_timestamp(timestamp.as_second(), timestamp.subsec_nanosecond().unsigned_abs()).unwrap_or_default()
    }
}

/// Fails when the value has sub-microsecond precision or lies outside the range of four-digit years.
impl TryFrom<chrono::DateTime<Utc>> for DateTime {
    type Error = TemporalError;

    fn try_from(value: chrono::DateTime<Utc>) -> Result<Self, Self::Error> {
        let nanosecond = i32::try_from(value.timestamp_subsec_nanos()).map_err(|_| TemporalError::ComponentOutOfRange)?;
        Self::try_from(Timestamp::new(value.timestamp(), nanosecond)?)
    }
}

// Written out: see the module documentation.
impl From<Date> for NaiveDate {
    fn from(value: Date) -> Self {
        let date = civil::Date::from(value);
        // A clause 4.6.3 date has a four-digit year, which chrono always represents.
        Self::from_ymd_opt(i32::from(date.year()), date.month().unsigned_abs().into(), date.day().unsigned_abs().into()).unwrap_or_default()
    }
}

/// Fails when the year does not have four digits.
impl TryFrom<NaiveDate> for Date {
    type Error = TemporalError;

    fn try_from(value: NaiveDate) -> Result<Self, Self::Error> {
        use chrono::Datelike;
        let narrow = |component: u32| i8::try_from(component).map_err(|_| TemporalError::ComponentOutOfRange);
        let year = i16::try_from(value.year()).map_err(|_| TemporalError::ComponentOutOfRange)?;
        Ok(Self::from(civil::Date::new(year, narrow(value.month())?, narrow(value.day())?)?))
    }
}

// Written out: see the module documentation.
impl From<Time> for NaiveTime {
    fn from(value: Time) -> Self {
        let time = value.time();
        let component = |value: i8| u32::from(value.unsigned_abs());
        // A civil time is always a valid time of day.
        Self::from_hms_nano_opt(
            component(time.hour()),
            component(time.minute()),
            component(time.second()),
            time.subsec_nanosecond().unsigned_abs(),
        )
        .unwrap_or_default()
    }
}

/// Fails on sub-microsecond precision and on leap seconds, which clause 4.6.3 cannot express.
impl TryFrom<NaiveTime> for Time {
    type Error = TemporalError;

    fn try_from(value: NaiveTime) -> Result<Self, Self::Error> {
        use chrono::Timelike;
        let narrow = |component: u32| i8::try_from(component).map_err(|_| TemporalError::ComponentOutOfRange);
        let nanosecond = i32::try_from(value.nanosecond()).map_err(|_| TemporalError::ComponentOutOfRange)?;
        Self::try_from(civil::Time::new(
            narrow(value.hour())?,
            narrow(value.minute())?,
            narrow(value.second())?,
            nanosecond,
        )?)
    }
}
