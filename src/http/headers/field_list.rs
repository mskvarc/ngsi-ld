//! The comma-separated list a header field value holds (RFC 9110 section 5.6.1), read and written.

use crate::http::headers::{InvalidFieldValue, field_value::encode_text, quoted_string::QuotedScan};
use http::HeaderValue;
use std::fmt;

/// Why a header holding a list of elements could not be built.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ListFieldError {
    /// No element was given, and the header needs at least one.
    #[error("the header needs at least one element")]
    Empty,
    /// The rendered elements cannot be carried in a header field.
    #[error("the elements cannot be sent in a header")]
    FieldValue(#[from] InvalidFieldValue),
}

/// A quoted string or a comment left open at the end of a field value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unterminated;

/// Where the scan of a field value stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scan {
    /// Outside any quoted string or comment: a delimiter splits here.
    Plain,
    /// Inside a quoted string (RFC 9110 section 5.6.4).
    Quoted(QuotedScan),
    /// Inside a comment (RFC 9110 section 5.6.5), nested this deep.
    Comment(usize),
    /// Right after a backslash inside a comment nested this deep.
    CommentPair(usize),
}

impl Scan {
    /// The state after `byte`, or `None` where a delimiter splits.
    fn next(self, byte: u8, delimiter: u8) -> Option<Scan> {
        Some(match (self, byte) {
            (Scan::Plain, found) if found == delimiter => return None,
            (Scan::Plain, b'"') => Scan::Quoted(QuotedScan::Text),
            (Scan::Plain, b'(') => Scan::Comment(1),
            (Scan::Quoted(quoted), found) => quoted.next(found).map_or(Scan::Plain, Scan::Quoted),
            (Scan::Comment(1), b')') => Scan::Plain,
            (Scan::Comment(depth), b')') => Scan::Comment(depth - 1),
            (Scan::Comment(depth), b'(') => Scan::Comment(depth + 1),
            (Scan::Comment(depth), b'\\') => Scan::CommentPair(depth),
            (Scan::CommentPair(depth), _) => Scan::Comment(depth),
            (unchanged @ (Scan::Plain | Scan::Comment(_)), _) => unchanged,
        })
    }
}

/// Splits `text` on every `delimiter` outside a quoted string or a comment, keeping each piece as written.
///
/// A quoted string or a comment left open makes the text unreadable.
pub(crate) fn split(text: &str, delimiter: u8) -> Result<Vec<&str>, Unterminated> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut scan = Scan::Plain;
    for (index, byte) in text.bytes().enumerate() {
        if let Some(next) = scan.next(byte, delimiter) {
            scan = next;
        } else {
            // The delimiter is ASCII, so both slices start and end on character boundaries.
            pieces.push(&text[start..index]);
            start = index + 1;
        }
    }
    if scan != Scan::Plain {
        return Err(Unterminated);
    }
    pieces.push(&text[start..]);
    Ok(pieces)
}

/// Trims the optional whitespace (`OWS`, RFC 9110 section 5.6.3) around `text`.
pub(crate) fn trim_ows(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

/// Reads every element of a list field, across all its lines, in order, with `parse`.
///
/// Empty elements are skipped, as RFC 9110 section 5.6.1 requires of a recipient; any element `parse` refuses, and
/// any line that is not visible ASCII, makes the field unreadable. A field with no line at all is absent, not empty,
/// and is unreadable too.
pub(crate) fn decode_elements<'i, I, T, E>(values: &mut I, mut parse: impl FnMut(&'i str) -> Result<T, E>) -> Result<Vec<T>, headers::Error>
where
    I: Iterator<Item = &'i HeaderValue>,
{
    let mut values = values.peekable();
    if values.peek().is_none() {
        return Err(headers::Error::invalid());
    }
    let mut elements = Vec::new();
    for value in values {
        let line = value.to_str().map_err(|_| headers::Error::invalid())?;
        let elements_of_line = split(line, b',').map_err(|Unterminated| headers::Error::invalid())?;
        for element in elements_of_line.into_iter().map(trim_ows).filter(|element| !element.is_empty()) {
            elements.push(parse(element).map_err(|_| headers::Error::invalid())?);
        }
    }
    Ok(elements)
}

/// The elements of a list field and their rendering as one field line, `, `-separated.
///
/// A list is built only when its rendering is a field value, so encoding, which renders it again, never drops it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FieldList<T> {
    /// The elements, in order.
    elements: Vec<T>,
}

impl<T: fmt::Display> FieldList<T> {
    /// The list of `elements`, which may be empty.
    pub(crate) fn new(elements: Vec<T>) -> Result<FieldList<T>, InvalidFieldValue> {
        let list = FieldList { elements };
        encode_text(&list.render())?;
        Ok(list)
    }

    /// The list of `elements`, whose rendering is a field value by construction, such as media types without
    /// parameters, whose names are restricted names (RFC 6838 section 4.2).
    pub(crate) const fn known(elements: Vec<T>) -> FieldList<T> {
        FieldList { elements }
    }

    /// The list of `elements`, which must not be empty.
    pub(crate) fn non_empty(elements: Vec<T>) -> Result<FieldList<T>, ListFieldError> {
        if elements.is_empty() {
            return Err(ListFieldError::Empty);
        }
        Ok(FieldList::new(elements)?)
    }

    /// The elements rendered `, `-separated.
    fn render(&self) -> String {
        let rendered: Vec<String> = self.elements.iter().map(ToString::to_string).collect();
        rendered.join(", ")
    }

    /// Writes the field line.
    pub(crate) fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // Every list was built from a rendering that is a field value, so this rendering of it is one too.
        values.extend(encode_text(&self.render()).ok());
    }
}

impl<T> FieldList<T> {
    /// The elements, in order.
    pub(crate) fn elements(&self) -> &[T] {
        &self.elements
    }

    /// Takes the elements, in order.
    pub(crate) fn into_elements(self) -> Vec<T> {
        self.elements
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{
        ListFieldError,
        field_list::{FieldList, decode_elements, split},
    };
    use http::HeaderValue;
    use pretty_assertions::assert_eq;
    use std::iter;

    #[test]
    fn a_delimiter_inside_a_quoted_string_or_a_comment_does_not_split() {
        assert_eq!(split(r#"a, "b, c", d"#, b',').ok(), Some(vec!["a", r#" "b, c""#, " d"]));
        assert_eq!(split("1.1 p (a, (b, c)), 1.1 q", b',').ok(), Some(vec!["1.1 p (a, (b, c))", " 1.1 q"]));
        assert_eq!(split(r#"a; "x;\";y"; b"#, b';').ok(), Some(vec!["a", r#" "x;\";y""#, " b"]));
        assert_eq!(split(r"(a\), b), c", b',').ok(), Some(vec![r"(a\), b)", " c"]));
    }

    #[test]
    fn an_open_quoted_string_or_comment_is_rejected() {
        for text in ["\"a, b", "(a, b", "(a (b)", r#""a\""#] {
            assert!(split(text, b',').is_err(), "{text}");
        }
    }

    #[test]
    fn every_line_is_read_in_order_and_empty_elements_are_skipped() {
        let lines = [HeaderValue::from_static(" a ,, b"), HeaderValue::from_static(", c,")];
        let elements = decode_elements(&mut lines.iter(), |element| Ok::<_, ()>(element.to_owned()));
        assert_eq!(elements.ok(), Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]));
    }

    #[test]
    fn an_element_the_parser_refuses_or_a_non_ascii_line_fails_the_field() {
        let lines = [HeaderValue::from_static("a, b")];
        assert!(decode_elements(&mut lines.iter(), |element| if element == "b" { Err(()) } else { Ok(()) }).is_err());
        let opaque = [HeaderValue::from_bytes(b"caf\xe9").unwrap()];
        assert!(decode_elements(&mut opaque.iter(), |_| Ok::<_, ()>(())).is_err());
        assert!(decode_elements(&mut iter::empty(), |_| Ok::<_, ()>(())).is_err());
    }

    #[test]
    fn a_list_renders_its_elements_comma_separated() {
        let list = FieldList::non_empty(vec!["a", "b"]).unwrap();
        let mut encoded = Vec::new();
        list.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("a, b")]);
        assert_eq!(list.elements(), &["a", "b"]);
        assert_eq!(list.into_elements(), vec!["a", "b"]);
        let mut empty = Vec::new();
        FieldList::<&str>::new(Vec::new()).unwrap().encode(&mut empty);
        assert_eq!(empty, vec![HeaderValue::from_static("")]);
    }

    #[test]
    fn a_list_that_must_not_be_empty_or_cannot_be_carried_is_refused() {
        assert!(matches!(FieldList::<&str>::non_empty(Vec::new()), Err(ListFieldError::Empty)));
        assert!(matches!(FieldList::non_empty(vec!["caf\u{e9}"]), Err(ListFieldError::FieldValue(_))));
    }
}
