//! The `NGSILD-Results-Count` header (NGSI-LD clause 6.3.13).

use crate::http::headers::field_value::decode_singleton;
use http::{HeaderName, HeaderValue};
use std::iter;

/// The header name.
static NGSILD_RESULTS_COUNT: HeaderName = HeaderName::from_static("ngsild-results-count");

/// The `NGSILD-Results-Count` header: the total number of results of a query.
///
/// It counts the matching results of a paginated query (NGSI-LD clause 6.3.13).
///
/// Encoding goes through `HeaderValue::from(u64)`, which cannot fail. Decoding accepts ASCII digits only, so a sign
/// or a fraction is rejected.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::HeaderMap;
/// use ngsi_ld::http::headers::NgsiLdResultsCount;
///
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(NgsiLdResultsCount::new(42));
/// assert_eq!(
///     headers
///         .typed_get::<NgsiLdResultsCount>()
///         .map(NgsiLdResultsCount::count),
///     Some(42)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NgsiLdResultsCount(u64);

impl NgsiLdResultsCount {
    /// A header reporting `count` results.
    #[must_use]
    pub const fn new(count: u64) -> NgsiLdResultsCount {
        NgsiLdResultsCount(count)
    }

    /// The total number of matching results.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.0
    }
}

impl headers::Header for NgsiLdResultsCount {
    fn name() -> &'static HeaderName {
        &NGSILD_RESULTS_COUNT
    }

    fn decode<'i, I>(values: &mut I) -> Result<NgsiLdResultsCount, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let digits = decode_singleton(values)?;
        // `u64::from_str` also accepts a leading `+`, which is not an integer in the header's grammar.
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(headers::Error::invalid());
        }
        digits.parse().map(NgsiLdResultsCount).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(iter::once(HeaderValue::from(self.0)));
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{NgsiLdResultsCount, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn decode(raw: &'static str) -> Option<u64> {
        decode_line::<NgsiLdResultsCount>(raw).map(NgsiLdResultsCount::count)
    }

    #[test]
    fn a_count_of_forty_two_decodes() {
        assert_eq!(decode("42"), Some(42));
        assert_eq!(decode("0"), Some(0));
        assert_eq!(decode("18446744073709551615"), Some(u64::MAX));
    }

    #[test]
    fn a_count_that_is_not_a_non_negative_integer_is_rejected() {
        for raw in ["abc", "-1", "+1", "4.2", "", "18446744073709551616"] {
            assert_eq!(decode(raw), None, "{raw}");
        }
    }

    #[test]
    fn a_count_encodes_as_digits_and_decodes_back() {
        let mut encoded = Vec::new();
        NgsiLdResultsCount::new(100).encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("100")]);
        assert_eq!(NgsiLdResultsCount::decode(&mut encoded.iter()).ok(), Some(NgsiLdResultsCount::new(100)));
    }
}
