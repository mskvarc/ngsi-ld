//! The `Accept-Language` header (RFC 9110 section 12.5.4).

pub(super) mod language_range;

use crate::{
    http::headers::{
        BasicLanguageRange,
        ListFieldError,
        field_list::{FieldList, decode_elements},
        weight::split_weight,
    },
    query::{Weighted, language_filter::LanguageFilter},
};
use http::{HeaderName, HeaderValue, header::ACCEPT_LANGUAGE};

/// An `Accept-Language` header: the natural languages a client prefers.
///
/// RFC 9110 section 12.5.4 defines the header. The languages are listed in order, each a basic language range (RFC 4647
/// section 2.1) with an optional weight.
///
/// The NGSI-LD Language Filter (clause 4.15) writes the syntax of this header in its `lang` parameter, with one
/// difference: clause 4.15 takes BCP 47 language tags (RFC 5646), where this header takes any basic language range.
/// Every tag is a basic range, so a filter converts into the header unchanged; a header range that is no tag, such as
/// `en-a-b`, decodes here but has no filter counterpart. Decoding reads any valid list, across all its lines, keeping
/// its order and weights; a list with no element does not decode. The field value is rendered on construction, so
/// encoding cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, header};
/// use ngsi_ld::{
///     http::headers::AcceptLanguage,
///     query::{Quality, language_filter},
/// };
///
/// let filter = language_filter::language("fr-CH")
///     .language("fr")
///     .weighted_language("en", Quality::try_from_thousandths(800)?)
///     .weighted_any(Quality::try_from_thousandths(500)?)
///     .build()?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(AcceptLanguage::try_from(filter)?);
/// assert_eq!(
///     headers[&header::ACCEPT_LANGUAGE],
///     "fr-CH, fr, en;q=0.8, *;q=0.5"
/// );
/// assert_eq!(
///     headers
///         .typed_get::<AcceptLanguage>()
///         .map(|header| header.preferences().len()),
///     Some(4)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AcceptLanguage(FieldList<Weighted<BasicLanguageRange>>);

impl AcceptLanguage {
    /// An `Accept-Language` listing `preferences`, in order.
    ///
    /// # Errors
    ///
    /// Returns [`ListFieldError::Empty`] when `preferences` is empty, and [`ListFieldError::FieldValue`] when the
    /// rendering cannot be carried in a header field, which language ranges never cause.
    pub fn try_new(preferences: impl IntoIterator<Item = Weighted<BasicLanguageRange>>) -> Result<AcceptLanguage, ListFieldError> {
        FieldList::non_empty(preferences.into_iter().collect()).map(AcceptLanguage)
    }

    /// The preferences, in order.
    #[must_use]
    pub fn preferences(&self) -> &[Weighted<BasicLanguageRange>] {
        self.0.elements()
    }
}

/// The preferences of a Language Filter (NGSI-LD clause 4.15), in order.
impl TryFrom<LanguageFilter> for AcceptLanguage {
    type Error = ListFieldError;

    fn try_from(filter: LanguageFilter) -> Result<AcceptLanguage, ListFieldError> {
        AcceptLanguage::try_new(filter.into_preferences().into_iter().map(|preference| preference.map(BasicLanguageRange::from)))
    }
}

/// Reads one element: a basic language range with an optional weight.
fn parse_preference(element: &str) -> Result<Weighted<BasicLanguageRange>, headers::Error> {
    let (range, quality) = split_weight(element)?;
    let range = range.parse().map_err(|_| headers::Error::invalid())?;
    Ok(Weighted::from_parts(range, quality))
}

impl headers::Header for AcceptLanguage {
    fn name() -> &'static HeaderName {
        &ACCEPT_LANGUAGE
    }

    fn decode<'i, I>(values: &mut I) -> Result<AcceptLanguage, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        AcceptLanguage::try_new(decode_elements(values, parse_preference)?).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{AcceptLanguage, BasicLanguageRange, ListFieldError, field_value::tests::decode_line},
        query::{Quality, Weighted, language_filter::LanguageFilter},
    };
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn encoded(header: &AcceptLanguage) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        header.encode(&mut values);
        values
    }

    fn rendered(header: &AcceptLanguage) -> Vec<String> {
        header.preferences().iter().map(ToString::to_string).collect()
    }

    #[test]
    fn preferences_encode_in_order_and_decode_back() {
        let header = AcceptLanguage::try_new([
            Weighted::new("de-AT".parse::<BasicLanguageRange>().unwrap()),
            Weighted::new(BasicLanguageRange::any()).with_quality(Quality::try_from_thousandths(100).unwrap()),
        ])
        .unwrap();
        assert_eq!(encoded(&header), vec![HeaderValue::from_static("de-AT, *;q=0.1")]);
        assert_eq!(AcceptLanguage::decode(&mut encoded(&header).iter()).ok(), Some(header));
    }

    #[test]
    fn a_language_filter_converts_unchanged() {
        let filter: LanguageFilter = "fr-CH,fr;q=0.9,en;q=0.8,*;q=0.5".parse().unwrap();
        let header = AcceptLanguage::try_from(filter).unwrap();
        assert_eq!(encoded(&header), vec![HeaderValue::from_static("fr-CH, fr;q=0.9, en;q=0.8, *;q=0.5")]);
    }

    #[test]
    fn a_list_with_whitespace_across_lines_decodes_in_order() {
        let lines = [HeaderValue::from_static("da ,en-GB ; q=0.8"), HeaderValue::from_static("en;Q=0.7, *;q=0")];
        let header = AcceptLanguage::decode(&mut lines.iter()).unwrap();
        assert_eq!(rendered(&header), vec!["da", "en-GB;q=0.8", "en;q=0.7", "*;q=0"]);
    }

    #[test]
    fn a_basic_language_range_that_is_no_language_tag_decodes() {
        let header = decode_line::<AcceptLanguage>("en-a-b;q=0.5, x-y").unwrap();
        assert_eq!(rendered(&header), vec!["en-a-b;q=0.5", "x-y"]);
        assert_eq!(AcceptLanguage::decode(&mut encoded(&header).iter()).ok(), Some(header));
    }

    #[test]
    fn an_invalid_range_weight_or_parameter_or_an_empty_list_does_not_decode() {
        for raw in ["", " , ", "en_US", "fr;q=1.1", "fr;level=1", "fr;q=0.5;q=0.4", "*-CH", "fr en", "en-123456789"] {
            assert_eq!(decode_line::<AcceptLanguage>(raw), None, "{raw}");
        }
    }

    #[test]
    fn an_empty_list_is_refused_on_construction() {
        assert!(matches!(AcceptLanguage::try_new([]), Err(ListFieldError::Empty)));
    }
}
