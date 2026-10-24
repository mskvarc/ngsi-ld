//! The `Content-Range` header of a partial temporal answer (RFC 9110 section 14.4, NGSI-LD clause 6.3.10).

use crate::{
    http::headers::{
        InvalidFieldValue,
        field_value::{decode_singleton, encode_text},
    },
    model::DateTime,
};
use http::{HeaderName, HeaderValue, header::CONTENT_RANGE};
use std::{iter, num::NonZeroU64};

/// The range unit NGSI-LD gives the time range of a partial temporal answer (clause 6.3.10).
const DATE_TIME_UNIT: &str = "DateTime";

/// The `size` of a temporal [`ContentRange`] (NGSI-LD clause 6.3.10): the `lastN` of a request that paginates
/// backwards, or `*` for one that paginates forwards.
///
/// The enum is exhaustive: clause 6.3.10 allows these two forms only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentRangeSize {
    /// `*`: the request had no `lastN`, so the answer runs forwards, from the least recent instance.
    Unknown,
    /// The `lastN` of the request, which runs the answer backwards, from the most recent instance.
    LastN(NonZeroU64),
}

/// The `Content-Range` header of a 206 Partial Content answer to a temporal request (NGSI-LD clause 6.3.10): the
/// unit `DateTime`, the range the answer covers and its size, written `DateTime <start>-<end>/<size>`.
///
/// Clause 6.3.10 fixes what the bounds are. A request without `lastN` runs forwards: `start` is the `timeAt` of an
/// `after` or `between` request (the least recent instance of the answer for `before`) and `end` the most recent
/// instance of the answer. A request with `lastN` runs backwards: `start` is the `timeAt` of a `before` request, the
/// `endTimeAt` of a `between` request (the most recent instance of the answer for `after`) and `end` the least
/// recent instance of the answer.
///
/// The `headers` crate's `ContentRange` reads the `bytes` unit only, so this one reads the unit NGSI-LD defines.
/// Range units are case-insensitive (RFC 9110 section 14.1), so `datetime` is read too; the header is always
/// written `DateTime`. The field value is rendered once, by [`ContentRange::new`], since
/// [`headers::Header::encode`] cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, HeaderValue, header::CONTENT_RANGE};
/// use ngsi_ld::http::headers::{ContentRange, ContentRangeSize};
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     CONTENT_RANGE,
///     HeaderValue::from_static("DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/*"),
/// );
/// let range = headers.typed_get::<ContentRange>();
/// assert_eq!(
///     range
///         .as_ref()
///         .map(|range| range.end().to_string())
///         .as_deref(),
///     Some("2018-08-01T12:05:00Z")
/// );
/// assert_eq!(
///     range.map(|range| range.size()),
///     Some(ContentRangeSize::Unknown)
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRange {
    /// `range-start`.
    start: DateTime,
    /// `range-end`.
    end: DateTime,
    /// `size`.
    size: ContentRangeSize,
    /// The field value, rendered once.
    value: HeaderValue,
}

impl ContentRange {
    /// The header for the range from `start` to `end`, of `size`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendered range cannot be carried in a header field, which the
    /// ASCII lexical form of a `DateTime` (clause 4.6.3) never causes.
    pub fn new(start: DateTime, end: DateTime, size: ContentRangeSize) -> Result<ContentRange, InvalidFieldValue> {
        let size_text = match size {
            ContentRangeSize::Unknown => "*".to_owned(),
            ContentRangeSize::LastN(count) => count.to_string(),
        };
        let value = encode_text(&format!("{DATE_TIME_UNIT} {start}-{end}/{size_text}"))?;
        Ok(ContentRange { start, end, size, value })
    }

    /// Returns `range-start`.
    #[must_use]
    pub const fn start(&self) -> &DateTime {
        &self.start
    }

    /// Returns `range-end`.
    #[must_use]
    pub const fn end(&self) -> &DateTime {
        &self.end
    }

    /// Returns the size: the `lastN` of a backwards answer, or [`ContentRangeSize::Unknown`].
    #[must_use]
    pub const fn size(&self) -> ContentRangeSize {
        self.size
    }

    /// Reads `DateTime <start>-<end>/<size>`.
    fn read(text: &str) -> Option<ContentRange> {
        let (unit, range) = text.split_once(' ')?;
        if !unit.eq_ignore_ascii_case(DATE_TIME_UNIT) {
            return None;
        }
        let (bounds, size) = range.trim_start_matches(' ').rsplit_once('/')?;
        // A `DateTime` is in UTC and ends with `Z` (clause 4.6.3), so the first `Z-` ends the start; the `-` of the
        // date cannot follow a `Z`.
        let split = bounds.find("Z-")?;
        let start = bounds.get(..=split)?.parse().ok()?;
        let end = bounds.get(split + 2..)?.parse().ok()?;
        let size = match size {
            "*" => ContentRangeSize::Unknown,
            // `NonZeroU64::from_str` also accepts a leading `+`, which is not a digit in the header's grammar.
            digits if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) => ContentRangeSize::LastN(digits.parse().ok()?),
            _ => return None,
        };
        ContentRange::new(start, end, size).ok()
    }
}

impl headers::Header for ContentRange {
    fn name() -> &'static HeaderName {
        &CONTENT_RANGE
    }

    fn decode<'i, I>(values: &mut I) -> Result<ContentRange, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        ContentRange::read(decode_singleton(values)?).ok_or_else(headers::Error::invalid)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // The header owns its rendered value; cloning it is a reference count.
        values.extend(iter::once(self.value.clone()));
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{ContentRange, ContentRangeSize, field_value::tests::decode_line},
        model::DateTime,
    };
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;
    use std::num::NonZeroU64;

    fn time(text: &str) -> DateTime {
        text.parse().unwrap()
    }

    #[test]
    fn a_forwards_range_reads_its_bounds_and_an_unknown_size() {
        let range = decode_line::<ContentRange>("DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/*").unwrap();
        assert_eq!(range.start(), &time("2018-08-01T12:00:00Z"));
        assert_eq!(range.end(), &time("2018-08-01T12:05:00Z"));
        assert_eq!(range.size(), ContentRangeSize::Unknown);
    }

    #[test]
    fn a_backwards_range_reads_its_last_n_and_fractions_of_a_second() {
        let range = decode_line::<ContentRange>("DateTime 2018-08-01T12:07:00.250Z-2018-08-01T12:03:00Z/5").unwrap();
        assert_eq!(range.start().to_string(), "2018-08-01T12:07:00.250Z");
        assert_eq!(range.end(), &time("2018-08-01T12:03:00Z"));
        assert_eq!(range.size(), ContentRangeSize::LastN(NonZeroU64::new(5).unwrap()));
    }

    #[test]
    fn the_unit_is_read_case_insensitively_and_written_as_the_specification_spells_it() {
        let range = decode_line::<ContentRange>("datetime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/2").unwrap();
        let mut encoded = Vec::new();
        range.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/2")]);
    }

    #[test]
    fn a_range_that_is_not_a_date_time_range_is_rejected() {
        for raw in [
            "bytes 0-499/1234",
            "DateTime 2018-08-01T12:00:00Z/*",
            "DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z",
            "DateTime 2018-08-01T12:00:00Z-yesterday/*",
            "DateTime 2018-08-01T12:00:00+01:00-2018-08-01T12:05:00Z/*",
            "DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/0",
            "DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/+5",
            "DateTime 2018-08-01T12:00:00Z-2018-08-01T12:05:00Z/",
            "DateTime",
        ] {
            assert_eq!(decode_line::<ContentRange>(raw), None, "{raw}");
        }
    }

    #[test]
    fn a_built_range_encodes_and_decodes_back() {
        let range = ContentRange::new(
            time("2018-08-01T12:07:00Z"),
            time("2018-08-01T12:03:00Z"),
            ContentRangeSize::LastN(NonZeroU64::new(3).unwrap()),
        )
        .unwrap();
        let mut encoded = Vec::new();
        range.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("DateTime 2018-08-01T12:07:00Z-2018-08-01T12:03:00Z/3")]);
        assert_eq!(ContentRange::decode(&mut encoded.iter()).ok(), Some(range));
    }
}
