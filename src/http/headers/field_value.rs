//! The conversion between typed values and HTTP field values (RFC 9110 section 5.5).

use http::{HeaderValue, header::InvalidHeaderValue};

/// A rendered header value that an HTTP field cannot carry.
///
/// RFC 9110 section 5.5 limits what a sender generates to visible ASCII, spaces and tabs; a value holding anything
/// else, such as a non-ASCII character in an IRI, is refused when the typed header is constructed rather than when it
/// is encoded, because encoding cannot fail.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidFieldValue {
    /// The value holds a non-ASCII character, which RFC 9110 section 5.5 forbids a sender to generate.
    #[error("the value holds a non-ASCII character at byte {position}")]
    NonAscii {
        /// The byte offset of the character.
        position: usize,
    },
    /// The value holds a control character.
    #[error("the value holds a character an HTTP header field cannot carry")]
    Rejected(#[from] InvalidHeaderValue),
}

/// Whether `text` is made of the characters a sender generates in a field value: visible ASCII, spaces and tabs (RFC
/// 9110 section 5.5, without the obsolete `obs-text`).
pub(crate) fn is_field_text(text: &str) -> bool {
    text.bytes().all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
}

/// Renders `text` as a field value.
///
/// `HeaderValue::from_str` accepts non-ASCII bytes as obsolete text, which a peer's `to_str` then refuses, so they
/// are rejected here first.
pub(crate) fn encode_text(text: &str) -> Result<HeaderValue, InvalidFieldValue> {
    if let Some(position) = text.bytes().position(|byte| !byte.is_ascii()) {
        return Err(InvalidFieldValue::NonAscii { position });
    }
    Ok(HeaderValue::from_str(text)?)
}

/// Reads the one value of a singleton field (RFC 9110 section 5.5), without surrounding whitespace.
///
/// A singleton field sent more than once is invalid, and so is a value that is not visible ASCII.
pub(crate) fn decode_singleton<'i, I>(values: &mut I) -> Result<&'i str, headers::Error>
where
    I: Iterator<Item = &'i HeaderValue>,
{
    let value = values.next().ok_or_else(headers::Error::invalid)?;
    if values.next().is_some() {
        return Err(headers::Error::invalid());
    }
    value.to_str().map(|text| text.trim_matches([' ', '\t'])).map_err(|_| headers::Error::invalid())
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::http::headers::{
        InvalidFieldValue,
        field_value::{decode_singleton, encode_text, is_field_text},
    };
    use http::HeaderValue;
    use pretty_assertions::assert_eq;
    use std::iter;

    /// Decodes the typed header `H` from the one field line `raw`, `None` when it does not decode.
    pub(crate) fn decode_line<H: headers::Header>(raw: &'static str) -> Option<H> {
        H::decode(&mut iter::once(&HeaderValue::from_static(raw))).ok()
    }

    #[test]
    fn visible_ascii_text_encodes() {
        assert_eq!(
            encode_text("urn:ngsi-ld:Vehicle:A4567").ok(),
            Some(HeaderValue::from_static("urn:ngsi-ld:Vehicle:A4567"))
        );
    }

    #[test]
    fn non_ascii_and_control_characters_are_refused() {
        assert!(matches!(encode_text("caf\u{e9}"), Err(InvalidFieldValue::NonAscii { position: 3 })));
        assert!(matches!(encode_text("a\nb"), Err(InvalidFieldValue::Rejected(_))));
    }

    #[test]
    fn field_text_is_visible_ascii_spaces_and_tabs() {
        assert!(is_field_text("text/plain; a=\"b c\"\t~"));
        assert!(is_field_text(""));
        for text in ["a\nb", "a\rb", "\u{7f}", "caf\u{e9}", "\0"] {
            assert!(!is_field_text(text), "{text}");
        }
    }

    #[test]
    fn a_singleton_field_yields_its_trimmed_value() {
        let value = HeaderValue::from_static(" 42 ");
        assert_eq!(decode_singleton(&mut [&value].into_iter()).ok(), Some("42"));
    }

    #[test]
    fn a_singleton_field_sent_twice_or_not_at_all_is_rejected() {
        let value = HeaderValue::from_static("42");
        assert!(decode_singleton(&mut [&value, &value].into_iter()).is_err());
        assert!(decode_singleton(&mut iter::empty()).is_err());
    }
}
