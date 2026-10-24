//! The `Forwarded` header (RFC 7239).

pub(super) mod element;
pub(super) mod host;
pub(super) mod node;

use crate::http::headers::{
    ForwardedElement,
    InvalidFieldValue,
    ListFieldError,
    field_list::{FieldList, decode_elements},
};
use http::{HeaderName, HeaderValue, header::FORWARDED};

/// A `Forwarded` header (RFC 7239): what each proxy on the way learnt of a request, one element per proxy, from the
/// client outwards.
///
/// Decoding reads any valid list, across all its lines, in order. It refuses a repeated parameter (RFC 7239 section
/// 4) and an element with no parameter, and it refuses an extension parameter (section 5.5) too: this crate has no
/// type to keep one in, and dropping it would change what a proxy said. The field value is rendered on construction,
/// so encoding cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, header};
/// use ngsi_ld::http::headers::{Forwarded, ForwardedElement, Node};
/// use std::net::Ipv4Addr;
///
/// let client = ForwardedElement::builder()
///     .forwarded_for(Ipv4Addr::new(192, 0, 2, 43))
///     .proto("https".parse()?)
///     .build()?;
/// let proxy = ForwardedElement::builder()
///     .forwarded_for(Node::unknown())
///     .build()?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(Forwarded::new(client)?.appended(proxy)?);
/// assert_eq!(
///     headers[&header::FORWARDED],
///     "for=192.0.2.43;proto=https, for=unknown"
/// );
/// let decoded = headers.typed_get::<Forwarded>();
/// assert_eq!(
///     decoded.as_ref().map(|forwarded| forwarded.elements().len()),
///     Some(2)
/// );
/// let last_for = decoded
///     .as_ref()
///     .and_then(|forwarded| forwarded.elements().last())
///     .and_then(ForwardedElement::forwarded_for);
/// assert_eq!(last_for, Some(&Node::unknown()));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Forwarded(FieldList<ForwardedElement>);

impl Forwarded {
    /// A `Forwarded` of the one `element`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which validated
    /// elements never cause.
    pub fn new(element: ForwardedElement) -> Result<Forwarded, InvalidFieldValue> {
        FieldList::new(vec![element]).map(Forwarded)
    }

    /// A `Forwarded` of `elements`, from the client outwards.
    ///
    /// # Errors
    ///
    /// Returns [`ListFieldError::Empty`] when `elements` is empty, and [`ListFieldError::FieldValue`] when the
    /// rendering cannot be carried in a header field, which validated elements never cause.
    pub fn from_elements(elements: impl IntoIterator<Item = ForwardedElement>) -> Result<Forwarded, ListFieldError> {
        FieldList::non_empty(elements.into_iter().collect()).map(Forwarded)
    }

    /// The `Forwarded` with `element` added last, as a proxy passing the request on does (RFC 7239 section 4).
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which validated
    /// elements never cause.
    pub fn appended(self, element: ForwardedElement) -> Result<Forwarded, InvalidFieldValue> {
        let mut elements = self.0.into_elements();
        elements.push(element);
        FieldList::new(elements).map(Forwarded)
    }

    /// The elements, from the client outwards.
    #[must_use]
    pub fn elements(&self) -> &[ForwardedElement] {
        self.0.elements()
    }
}

impl headers::Header for Forwarded {
    fn name() -> &'static HeaderName {
        &FORWARDED
    }

    fn decode<'i, I>(values: &mut I) -> Result<Forwarded, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        Forwarded::from_elements(decode_elements(values, str::parse::<ForwardedElement>)?).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{Forwarded, ForwardedElement, ListFieldError, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn element(text: &str) -> ForwardedElement {
        text.parse().unwrap()
    }

    fn encoded(forwarded: &Forwarded) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        forwarded.encode(&mut values);
        values
    }

    #[test]
    fn the_examples_of_rfc_7239_decode() {
        for raw in [
            "for=\"_gazonk\"",
            "For=\"[2001:db8:cafe::17]:4711\"",
            "for=192.0.2.60;proto=http;by=203.0.113.43",
            "for=192.0.2.43, for=198.51.100.17",
        ] {
            assert!(decode_line::<Forwarded>(raw).is_some(), "{raw}");
        }
    }

    #[test]
    fn elements_encode_in_order_and_decode_back() {
        let forwarded = Forwarded::new(element("for=192.0.2.43"))
            .unwrap()
            .appended(element("for=\"[2001:db8::1]\";proto=https"))
            .unwrap();
        assert_eq!(
            encoded(&forwarded),
            vec![HeaderValue::from_static("for=192.0.2.43, for=\"[2001:db8::1]\";proto=https")]
        );
        assert_eq!(
            Forwarded::from_elements([element("for=192.0.2.43"), element("proto=https;for=\"[2001:db8::1]\"")]).ok(),
            Some(forwarded.clone())
        );
        assert_eq!(Forwarded::decode(&mut encoded(&forwarded).iter()).ok(), Some(forwarded));
    }

    #[test]
    fn elements_across_lines_decode_in_order() {
        let lines = [HeaderValue::from_static("for=192.0.2.43"), HeaderValue::from_static("for=unknown, by=_p")];
        let forwarded = Forwarded::decode(&mut lines.iter()).unwrap();
        let rendered: Vec<String> = forwarded.elements().iter().map(ToString::to_string).collect();
        assert_eq!(rendered, vec!["for=192.0.2.43", "for=unknown", "by=_p"]);
    }

    #[test]
    fn a_malformed_element_or_an_empty_field_does_not_decode() {
        for raw in ["", " , ", ";", "for", "secret=1", "for=a;for=b", "for=\"192.0.2.43", "for=[::1]", "proto=1http"] {
            assert_eq!(decode_line::<Forwarded>(raw), None, "{raw}");
        }
        assert!(matches!(Forwarded::from_elements([]), Err(ListFieldError::Empty)));
    }
}
