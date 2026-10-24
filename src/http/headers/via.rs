//! The `Via` header (RFC 9110 section 7.6.3, NGSI-LD clause 6.3.18).

pub(super) mod comment;
pub(super) mod entry;
pub(super) mod received_by;
pub(super) mod received_protocol;

use crate::http::headers::{
    InvalidFieldValue,
    ListFieldError,
    ViaEntry,
    field_list::{FieldList, decode_elements},
};
use http::{HeaderName, HeaderValue, header::VIA};

/// A `Via` header: the hops a message passed through, in the order they were added.
///
/// RFC 9110 section 7.6.3.
///
/// NGSI-LD clause 6.3.18 makes a broker passing a distributed operation onward add a hop naming itself by its
/// `contextSourceAlias`, and a broker receiving one treat the listed hops as context sources already encountered, so
/// that a forwarding loop ends. Decoding reads any valid list, across all its lines, in order; a list with no hop does
/// not decode. The field value is rendered on construction, so encoding cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, header};
/// use ngsi_ld::http::headers::{ReceivedBy, ReceivedProtocol, Via, ViaEntry};
///
/// let via = Via::try_new(ViaEntry::new(
///     "1.1".parse::<ReceivedProtocol>()?,
///     ReceivedBy::new("broker-a".parse()?),
/// ))?
/// .appended(ViaEntry::new(
///     "1.1".parse::<ReceivedProtocol>()?,
///     ReceivedBy::new("broker-b".parse()?).with_port(1026),
/// ))?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(via);
/// assert_eq!(headers[&header::VIA], "1.1 broker-a, 1.1 broker-b:1026");
/// let decoded = headers.typed_get::<Via>();
/// let ports: Option<Vec<Option<u16>>> = decoded.map(|via| {
///     via.entries()
///         .iter()
///         .map(|entry| entry.received_by().port())
///         .collect()
/// });
/// assert_eq!(ports, Some(vec![None, Some(1026)]));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Via(FieldList<ViaEntry>);

impl Via {
    /// A `Via` of the one hop `entry`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which validated hops
    /// never cause.
    pub fn try_new(entry: ViaEntry) -> Result<Via, InvalidFieldValue> {
        FieldList::new(vec![entry]).map(Via)
    }

    /// A `Via` of `entries`, in order.
    ///
    /// # Errors
    ///
    /// Returns [`ListFieldError::Empty`] when `entries` is empty, and [`ListFieldError::FieldValue`] when the
    /// rendering cannot be carried in a header field, which validated hops never cause.
    pub fn try_from_entries(entries: impl IntoIterator<Item = ViaEntry>) -> Result<Via, ListFieldError> {
        FieldList::non_empty(entries.into_iter().collect()).map(Via)
    }

    /// The `Via` with `entry` added as the last hop, as a recipient forwarding the message does.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field, which validated hops
    /// never cause.
    pub fn appended(self, entry: ViaEntry) -> Result<Via, InvalidFieldValue> {
        let mut entries = self.0.into_elements();
        entries.push(entry);
        FieldList::new(entries).map(Via)
    }

    /// The hops, in the order they were added.
    #[must_use]
    pub fn entries(&self) -> &[ViaEntry] {
        self.0.elements()
    }
}

impl headers::Header for Via {
    fn name() -> &'static HeaderName {
        &VIA
    }

    fn decode<'i, I>(values: &mut I) -> Result<Via, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        Via::try_from_entries(decode_elements(values, str::parse::<ViaEntry>)?).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{ListFieldError, Via, ViaEntry, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn entry(text: &str) -> ViaEntry {
        text.parse().unwrap()
    }

    fn encoded(via: &Via) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        via.encode(&mut values);
        values
    }

    #[test]
    fn one_hop_encodes_and_decodes_back() {
        let via = Via::try_new(entry("1.1 broker-a")).unwrap();
        assert_eq!(encoded(&via), vec![HeaderValue::from_static("1.1 broker-a")]);
        assert_eq!(Via::decode(&mut encoded(&via).iter()).ok(), Some(via));
    }

    #[test]
    fn appending_a_hop_keeps_the_earlier_ones_first() {
        let via = Via::try_new(entry("1.1 broker-a")).unwrap().appended(entry("HTTP/2 broker-b (x, y)")).unwrap();
        assert_eq!(encoded(&via), vec![HeaderValue::from_static("1.1 broker-a, HTTP/2 broker-b (x, y)")]);
        assert_eq!(
            Via::try_from_entries([entry("1.1 broker-a"), entry("HTTP/2 broker-b (x, y)")]).ok(),
            Some(via.clone())
        );
        assert_eq!(Via::decode(&mut encoded(&via).iter()).ok(), Some(via));
    }

    #[test]
    fn hops_across_lines_decode_in_order_and_a_comma_in_a_comment_does_not_split() {
        let lines = [
            HeaderValue::from_static("1.0 fred, 1.1 p.example.net (a, b)"),
            HeaderValue::from_static("2 broker-c"),
        ];
        let via = Via::decode(&mut lines.iter()).unwrap();
        let hops: Vec<String> = via.entries().iter().map(ToString::to_string).collect();
        assert_eq!(hops, vec!["1.0 fred", "1.1 p.example.net (a, b)", "2 broker-c"]);
    }

    #[test]
    fn a_malformed_hop_or_an_empty_field_does_not_decode() {
        for raw in ["", " , ", "broker-a", "1.1 broker-a, brokerX", "1.1 broker-a (open", "1.1 [::1]", "1.1 a b"] {
            assert_eq!(decode_line::<Via>(raw), None, "{raw}");
        }
    }

    #[test]
    fn an_empty_list_is_refused_on_construction() {
        assert!(matches!(Via::try_from_entries([]), Err(ListFieldError::Empty)));
    }
}
