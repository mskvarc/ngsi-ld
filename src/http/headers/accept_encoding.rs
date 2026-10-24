//! The `Accept-Encoding` header (RFC 9110 section 12.5.3).

use crate::{
    http::headers::{
        CodingRange,
        InvalidFieldValue,
        field_list::{FieldList, decode_elements},
        weight::split_weight,
    },
    query::Weighted,
};
use http::{HeaderName, HeaderValue, header::ACCEPT_ENCODING};

/// An `Accept-Encoding` header (RFC 9110 section 12.5.3): the content codings a client takes a response in, in
/// order, each with an optional weight.
///
/// The list may be empty: RFC 9110 section 12.5.3 gives an empty field the meaning that no coding is wanted. Decoding
/// reads any valid list, across all its lines, keeping its order and weights. The field value is rendered on
/// construction, so encoding cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, header};
/// use ngsi_ld::{
///     http::headers::{AcceptEncoding, CodingRange, ContentCoding},
///     query::{Quality, Weighted},
/// };
///
/// let header = AcceptEncoding::try_new([
///     Weighted::new("gzip".parse::<ContentCoding>()?),
///     Weighted::new(CodingRange::Identity).with_quality(Quality::try_from_thousandths(500)?),
///     Weighted::new(CodingRange::Any).with_quality(Quality::try_from_thousandths(0)?),
/// ])?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(header);
/// assert_eq!(
///     headers[&header::ACCEPT_ENCODING],
///     "gzip, identity;q=0.5, *;q=0"
/// );
/// let decoded = headers.typed_get::<AcceptEncoding>();
/// assert_eq!(
///     decoded.as_ref().map(|header| header.codings().len()),
///     Some(3)
/// );
/// assert_eq!(
///     decoded.and_then(|header| header.codings().last().and_then(Weighted::quality)),
///     Some(Quality::try_from_thousandths(0)?)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AcceptEncoding(FieldList<Weighted<CodingRange>>);

impl AcceptEncoding {
    /// An `Accept-Encoding` listing `codings`, in order; no coding asks for none.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which content codings
    /// never cause.
    pub fn try_new(codings: impl IntoIterator<Item = Weighted<CodingRange>>) -> Result<AcceptEncoding, InvalidFieldValue> {
        FieldList::new(codings.into_iter().collect()).map(AcceptEncoding)
    }

    /// The codings, in order, with their weights.
    #[must_use]
    pub fn codings(&self) -> &[Weighted<CodingRange>] {
        self.0.elements()
    }
}

/// Reads one element: a coding range with an optional weight.
fn parse_coding(element: &str) -> Result<Weighted<CodingRange>, headers::Error> {
    let (coding, quality) = split_weight(element)?;
    let coding = coding.parse().map_err(|_| headers::Error::invalid())?;
    Ok(Weighted::from_parts(coding, quality))
}

impl headers::Header for AcceptEncoding {
    fn name() -> &'static HeaderName {
        &ACCEPT_ENCODING
    }

    fn decode<'i, I>(values: &mut I) -> Result<AcceptEncoding, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        AcceptEncoding::try_new(decode_elements(values, parse_coding)?).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{AcceptEncoding, CodingRange, field_value::tests::decode_line},
        query::{Quality, Weighted},
    };
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn encoded(header: &AcceptEncoding) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        header.encode(&mut values);
        values
    }

    #[test]
    fn codings_encode_in_order_and_decode_back() {
        let header = AcceptEncoding::try_new([
            Weighted::new(CodingRange::Coding("br".parse().unwrap())).with_quality(Quality::try_from_thousandths(1000).unwrap()),
            Weighted::new(CodingRange::Coding("gzip".parse().unwrap())).with_quality(Quality::try_from_thousandths(800).unwrap()),
            Weighted::new(CodingRange::Identity),
        ])
        .unwrap();
        assert_eq!(encoded(&header), vec![HeaderValue::from_static("br;q=1, gzip;q=0.8, identity")]);
        assert_eq!(AcceptEncoding::decode(&mut encoded(&header).iter()).ok(), Some(header));
    }

    #[test]
    fn an_empty_field_asks_for_no_coding() {
        let header = decode_line::<AcceptEncoding>("").unwrap();
        assert_eq!(header.codings(), []);
        assert_eq!(encoded(&header), vec![HeaderValue::from_static("")]);
        assert_eq!(AcceptEncoding::try_new([]).ok(), Some(header));
    }

    #[test]
    fn a_list_across_lines_decodes_in_order_whatever_the_case() {
        let lines = [HeaderValue::from_static("GZIP ;q=0.5"), HeaderValue::from_static("Identity, *;q=0")];
        let header = AcceptEncoding::decode(&mut lines.iter()).unwrap();
        let rendered: Vec<String> = header.codings().iter().map(ToString::to_string).collect();
        assert_eq!(rendered, vec!["gzip;q=0.5", "identity", "*;q=0"]);
    }

    #[test]
    fn a_coding_that_is_no_token_or_a_malformed_weight_does_not_decode() {
        for raw in ["g zip", "gzip;q=2", "gzip;level=1", "gzip;q=0.5;q=0.4", "\"gzip\""] {
            assert_eq!(decode_line::<AcceptEncoding>(raw), None, "{raw}");
        }
    }
}
