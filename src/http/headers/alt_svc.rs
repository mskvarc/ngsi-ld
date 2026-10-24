//! The `Alt-Svc` header (RFC 7838).

pub(super) mod alternative;

use crate::http::headers::{
    Alternative,
    InvalidFieldValue,
    ListFieldError,
    field_list::{FieldList, decode_elements},
};
use http::{HeaderName, HeaderValue, header::ALT_SVC};
use std::{convert::Infallible, iter};

/// The value withdrawing every alternative of an origin (RFC 7838 section 3); it is case-sensitive.
const CLEAR: &str = "clear";

/// What an `Alt-Svc` header says.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AltSvcValue {
    /// `clear`.
    Clear,
    /// The alternatives, in order of preference.
    Alternatives(FieldList<Alternative>),
}

/// An `Alt-Svc` header (RFC 7838 section 3): the alternative services an origin is also reached at, in order of
/// preference, or `clear` to withdraw every alternative advertised before.
///
/// Decoding reads `clear` alone or any valid list of alternatives, across all its lines, in order. The field value
/// is rendered on construction, so encoding cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, header};
/// use ngsi_ld::http::headers::{AltAuthority, AltSvc, Alternative};
/// use std::time::Duration;
///
/// let h3 = Alternative::builder()
///     .protocol("h3".parse()?)
///     .authority(AltAuthority::same_host(443))
///     .max_age(Duration::from_secs(3600))
///     .build();
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(AltSvc::try_new(h3)?);
/// assert_eq!(headers[&header::ALT_SVC], "h3=\":443\"; ma=3600");
/// headers.typed_insert(AltSvc::clear());
/// assert_eq!(headers[&header::ALT_SVC], "clear");
/// assert_eq!(
///     headers
///         .typed_get::<AltSvc>()
///         .map(|header| header.is_clear()),
///     Some(true)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AltSvc(AltSvcValue);

impl AltSvc {
    /// `clear`: every alternative advertised before is withdrawn.
    #[must_use]
    pub const fn clear() -> AltSvc {
        AltSvc(AltSvcValue::Clear)
    }

    /// An `Alt-Svc` advertising the one `alternative`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which validated
    /// alternatives never cause.
    pub fn try_new(alternative: Alternative) -> Result<AltSvc, InvalidFieldValue> {
        FieldList::new(vec![alternative]).map(|list| AltSvc(AltSvcValue::Alternatives(list)))
    }

    /// An `Alt-Svc` advertising `alternatives`, in order of preference.
    ///
    /// # Errors
    ///
    /// Returns [`ListFieldError::Empty`] when `alternatives` is empty, and [`ListFieldError::FieldValue`] when the
    /// rendering cannot be carried in a header field, which validated alternatives never cause.
    pub fn try_from_alternatives(alternatives: impl IntoIterator<Item = Alternative>) -> Result<AltSvc, ListFieldError> {
        FieldList::non_empty(alternatives.into_iter().collect()).map(|list| AltSvc(AltSvcValue::Alternatives(list)))
    }

    /// Whether the header is `clear`.
    #[must_use]
    pub const fn is_clear(&self) -> bool {
        matches!(self.0, AltSvcValue::Clear)
    }

    /// The alternatives, in order of preference; none for `clear`.
    #[must_use]
    pub fn alternatives(&self) -> &[Alternative] {
        match &self.0 {
            AltSvcValue::Clear => &[],
            AltSvcValue::Alternatives(list) => list.elements(),
        }
    }
}

impl headers::Header for AltSvc {
    fn name() -> &'static HeaderName {
        &ALT_SVC
    }

    fn decode<'i, I>(values: &mut I) -> Result<AltSvc, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let elements = decode_elements(values, Ok::<&str, Infallible>)?;
        if elements == [CLEAR] {
            return Ok(AltSvc::clear());
        }
        let alternatives = elements
            .into_iter()
            .map(str::parse)
            .collect::<Result<Vec<Alternative>, _>>()
            .map_err(|_| headers::Error::invalid())?;
        AltSvc::try_from_alternatives(alternatives).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        match &self.0 {
            AltSvcValue::Clear => values.extend(iter::once(HeaderValue::from_static(CLEAR))),
            AltSvcValue::Alternatives(list) => list.encode(values),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{AltSvc, Alternative, ListFieldError, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn alternative(text: &str) -> Alternative {
        text.parse().unwrap()
    }

    fn encoded(header: &AltSvc) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        header.encode(&mut values);
        values
    }

    #[test]
    fn clear_encodes_and_decodes_back() {
        assert_eq!(encoded(&AltSvc::clear()), vec![HeaderValue::from_static("clear")]);
        assert_eq!(decode_line::<AltSvc>("clear"), Some(AltSvc::clear()));
        assert_eq!(AltSvc::clear().alternatives(), []);
    }

    #[test]
    fn alternatives_encode_in_order_and_decode_back() {
        let header = AltSvc::try_from_alternatives([alternative("h3=\":443\"; ma=86400"), alternative("h2=\"alt.example.org:443\"")]).unwrap();
        assert_eq!(
            encoded(&header),
            vec![HeaderValue::from_static("h3=\":443\"; ma=86400, h2=\"alt.example.org:443\"")]
        );
        assert!(!header.is_clear());
        assert_eq!(header.alternatives().len(), 2);
        assert_eq!(AltSvc::decode(&mut encoded(&header).iter()).ok(), Some(header));
        assert_eq!(AltSvc::try_new(alternative("h3=\":443\"")).ok(), decode_line::<AltSvc>("h3=\":443\""));
    }

    #[test]
    fn alternatives_across_lines_decode_in_order() {
        let lines = [HeaderValue::from_static("h3=\":443\""), HeaderValue::from_static("h2=\":8443\";persist=1")];
        let header = AltSvc::decode(&mut lines.iter()).unwrap();
        let rendered: Vec<String> = header.alternatives().iter().map(ToString::to_string).collect();
        assert_eq!(rendered, vec!["h3=\":443\"", "h2=\":8443\"; persist=1"]);
    }

    #[test]
    fn clear_beside_alternatives_a_misspelt_clear_or_an_empty_field_does_not_decode() {
        for raw in ["", " , ", "Clear", "clear, h3=\":443\"", "h3=\":443\", clear", "h3", "h3=\":443\"; ma=x"] {
            assert_eq!(decode_line::<AltSvc>(raw), None, "{raw}");
        }
        assert!(matches!(AltSvc::try_from_alternatives([]), Err(ListFieldError::Empty)));
    }
}
