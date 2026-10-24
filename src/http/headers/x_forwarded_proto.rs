//! The `X-Forwarded-Proto` header: the de-facto predecessor of the `proto` parameter of `Forwarded` (RFC 7239
//! section 5.4).

use crate::http::headers::{
    InvalidFieldValue,
    ListFieldError,
    Scheme,
    field_list::{FieldList, decode_elements},
};
use http::{HeaderName, HeaderValue};

/// The header name.
static X_FORWARDED_PROTO: HeaderName = HeaderName::from_static("x-forwarded-proto");

/// An `X-Forwarded-Proto` header: the scheme a client used to reach a proxy, which the proxy passes on.
///
/// No RFC defines the header; RFC 7239 section 5.4 standardises its content as the `proto` parameter of
/// `Forwarded`. Proxies that append rather than replace write a comma-separated list, ordered from the client
/// outwards as `Forwarded` orders its elements, so the header holds a non-empty list of schemes. Decoding reads any
/// valid list, across all its lines, in order. The field value is rendered on construction, so encoding cannot fail.
///
/// ```
/// use headers::{Header, HeaderMapExt};
/// use http::HeaderMap;
/// use ngsi_ld::http::headers::{Scheme, XForwardedProto};
///
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(XForwardedProto::new("https".parse::<Scheme>()?)?);
/// assert_eq!(headers[XForwardedProto::name()], "https");
/// assert_eq!(
///     headers
///         .typed_get::<XForwardedProto>()
///         .map(|header| header.schemes().len()),
///     Some(1)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct XForwardedProto(FieldList<Scheme>);

impl XForwardedProto {
    /// An `X-Forwarded-Proto` naming `scheme`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which a scheme never
    /// causes.
    pub fn new(scheme: Scheme) -> Result<XForwardedProto, InvalidFieldValue> {
        FieldList::new(vec![scheme]).map(XForwardedProto)
    }

    /// An `X-Forwarded-Proto` listing `schemes`, from the client outwards.
    ///
    /// # Errors
    ///
    /// Returns [`ListFieldError::Empty`] when `schemes` is empty, and [`ListFieldError::FieldValue`] when the
    /// rendering cannot be carried in a header field, which schemes never cause.
    pub fn from_schemes(schemes: impl IntoIterator<Item = Scheme>) -> Result<XForwardedProto, ListFieldError> {
        FieldList::non_empty(schemes.into_iter().collect()).map(XForwardedProto)
    }

    /// The schemes, from the client outwards.
    #[must_use]
    pub fn schemes(&self) -> &[Scheme] {
        self.0.elements()
    }
}

impl headers::Header for XForwardedProto {
    fn name() -> &'static HeaderName {
        &X_FORWARDED_PROTO
    }

    fn decode<'i, I>(values: &mut I) -> Result<XForwardedProto, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        XForwardedProto::from_schemes(decode_elements(values, str::parse::<Scheme>)?).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{ListFieldError, Scheme, XForwardedProto, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn scheme(text: &str) -> Scheme {
        text.parse().unwrap()
    }

    fn encoded(header: &XForwardedProto) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        header.encode(&mut values);
        values
    }

    #[test]
    fn each_scheme_encodes_and_decodes_back() {
        for text in ["http", "https", "ws"] {
            let header = XForwardedProto::new(scheme(text)).unwrap();
            assert_eq!(encoded(&header), vec![HeaderValue::from_str(text).unwrap()]);
            assert_eq!(XForwardedProto::decode(&mut encoded(&header).iter()).ok(), Some(header));
        }
    }

    #[test]
    fn a_chain_of_schemes_keeps_its_order() {
        let header = XForwardedProto::from_schemes([scheme("https"), scheme("http")]).unwrap();
        assert_eq!(encoded(&header), vec![HeaderValue::from_static("https, http")]);
        assert_eq!(decode_line::<XForwardedProto>(" HTTPS ,http"), Some(header));
    }

    #[test]
    fn a_value_that_is_no_scheme_or_an_empty_field_does_not_decode() {
        for raw in ["", " , ", "1http", "https:", "\"https\"", "https http"] {
            assert_eq!(decode_line::<XForwardedProto>(raw), None, "{raw}");
        }
        assert!(matches!(XForwardedProto::from_schemes([]), Err(ListFieldError::Empty)));
    }
}
