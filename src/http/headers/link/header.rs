// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The `Link` header itself: a list of link-values (RFC 8288 section 3).
//!
//! This file is derived from the disabled `Link` implementation in <https://github.com/hyperium/headers> and
//! includes local modifications.

use crate::{
    http::headers::{InvalidFieldValue, LinkBuildError, LinkParseError, LinkValue, RelationType, field_value::encode_text, link::quoting::Elements},
    model::context::core_context_reference,
};
use http::{HeaderName, HeaderValue, header::LINK};
use iri_rs::IriRefBuf;
use std::{fmt, iter, str::FromStr};

/// A `Link` header: one or more link-values (RFC 8288, NGSI-LD clauses 6.3.5 and 6.3.10).
///
/// The field value is rendered whenever a `Link` is built or decoded, and kept: a target holding non-ASCII
/// characters cannot be carried verbatim, and [`headers::Header::encode`] cannot fail. Several `Link` field lines
/// decode as one list.
///
/// ```
/// use http::header;
/// use headers::HeaderMapExt;
/// use http::HeaderMap;
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::http::headers::Link;
///
/// let context = IriRefBuf::new("https://example.org/context.jsonld")?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(Link::json_ld_context(context)?);
/// assert_eq!(
///     headers[&header::LINK],
///     r#"<https://example.org/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json""#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{}", ValueList(values))]
pub struct Link {
    /// The link-values, in the order they were written; never empty.
    values: Vec<LinkValue>,
    /// The link-values rendered as one field value.
    encoded: HeaderValue,
}

/// A list of link-values written comma-separated, as one `Link` field value.
struct ValueList<'a>(&'a [LinkValue]);

// A list separator is written between elements only, which no derive can express.
impl fmt::Display for ValueList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((first, rest)) = self.0.split_first() {
            write!(f, "{first}")?;
            rest.iter().try_for_each(|value| write!(f, ", {value}"))?;
        }
        Ok(())
    }
}

impl Link {
    /// A `Link` header carrying one link-value.
    ///
    /// # Errors
    ///
    /// Returns [`LinkBuildError::Unencodable`] when the rendering cannot be carried in a header field.
    pub fn new(value: LinkValue) -> Result<Link, LinkBuildError> {
        Link::from_values(vec![value])
    }

    /// A `Link` header carrying `values`, in order.
    ///
    /// # Errors
    ///
    /// Returns [`LinkBuildError::Empty`] when `values` is empty, and [`LinkBuildError::Unencodable`] when the
    /// rendering cannot be carried in a header field.
    pub fn from_values(values: Vec<LinkValue>) -> Result<Link, LinkBuildError> {
        if values.is_empty() {
            return Err(LinkBuildError::Empty);
        }
        Ok(Link::encode_values(values)?)
    }

    /// The JSON-LD context link naming `context` (NGSI-LD clause 6.3.5):
    /// `<context>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json"`.
    ///
    /// # Errors
    ///
    /// Returns [`LinkBuildError::Unencodable`] when the rendering cannot be carried in a header field.
    pub fn json_ld_context(context: IriRefBuf) -> Result<Link, LinkBuildError> {
        Link::new(LinkValue::json_ld_context(context))
    }

    /// The JSON-LD context link naming the core context (clause 4.3.4): the `Link` an `application/json` answer
    /// carries when the request named no context of its own (clause 6.3.6).
    ///
    /// ```
    /// use ngsi_ld::{http::headers::Link, model::CORE_CONTEXT};
    ///
    /// let link = Link::core_context()?;
    /// assert!(link.context_urls().eq([CORE_CONTEXT.as_str()]));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`LinkBuildError::Unencodable`] when the rendering cannot be carried in a header field.
    pub fn core_context() -> Result<Link, LinkBuildError> {
        Link::json_ld_context(core_context_reference())
    }

    /// Renders a non-empty list of link-values.
    fn encode_values(values: Vec<LinkValue>) -> Result<Link, InvalidFieldValue> {
        let encoded = encode_text(&ValueList(&values).to_string())?;
        Ok(Link { values, encoded })
    }

    /// The link-values, in the order they were written.
    #[must_use]
    pub fn values(&self) -> &[LinkValue] {
        &self.values
    }

    /// The targets of every JSON-LD context link.
    pub fn context_urls(&self) -> impl Iterator<Item = &IriRefBuf> {
        self.values.iter().filter(|value| value.is_context_link()).map(LinkValue::target)
    }

    /// The link-values whose `rel` names `rel`.
    pub fn find_by_rel<'a>(&'a self, rel: &'a RelationType) -> impl Iterator<Item = &'a LinkValue> {
        self.values.iter().filter(move |value| value.has_rel(rel))
    }
}

/// Reads the link-values of one field line, skipping the empty list elements RFC 9110 section 5.6.1 tolerates.
fn parse_values(raw: &str) -> Result<Vec<LinkValue>, LinkParseError> {
    Elements::new(raw, b',').filter(|element| !element.trim().is_empty()).map(str::parse).collect()
}

/// What one `Link` field line yields when every readable link-value is kept.
#[derive(Debug, Default)]
pub(crate) struct LineValues {
    /// The link-values that were read, in order.
    pub(crate) values: Vec<LinkValue>,
    /// How many link-values could not be read.
    pub(crate) unread: usize,
}

/// Reads one field line leniently: a link-value that does not parse is counted and skipped instead of failing the
/// line, so one malformed link does not hide the others (a response reads its links this way, NGSI-LD clause
/// 6.3.10). A line that is not visible ASCII counts as one unread link-value.
pub(crate) fn read_line(line: &HeaderValue) -> LineValues {
    let Ok(text) = line.to_str() else {
        return LineValues { values: Vec::new(), unread: 1 };
    };
    Elements::new(text, b',')
        .filter(|element| !element.trim().is_empty())
        .fold(LineValues::default(), |mut read, element| {
            match element.parse() {
                Ok(value) => read.values.push(value),
                Err(_) => read.unread += 1,
            }
            read
        })
}

// The link-values are `,`-separated and at least one is required, which a derive cannot express.
impl FromStr for Link {
    type Err = LinkParseError;

    fn from_str(raw: &str) -> Result<Link, LinkParseError> {
        let values = parse_values(raw)?;
        if values.is_empty() {
            return Err(LinkParseError::Empty);
        }
        Ok(Link::encode_values(values)?)
    }
}

impl headers::Header for Link {
    fn name() -> &'static HeaderName {
        &LINK
    }

    fn decode<'i, I>(values: &mut I) -> Result<Link, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let mut link_values = Vec::new();
        for value in values {
            let text = value.to_str().map_err(|_| headers::Error::invalid())?;
            link_values.extend(parse_values(text).map_err(|_| headers::Error::invalid())?);
        }
        Link::from_values(link_values).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // `encode` borrows the header; cloning a `HeaderValue` only bumps the reference count of its shared bytes.
        values.extend(iter::once(self.encoded.clone()));
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{Link, LinkBuildError, LinkParseError, LinkValue, RelationType, field_value::tests::decode_line, link::header::read_line},
        model::CORE_CONTEXT,
    };
    use headers::{Header, HeaderMapExt};
    use http::{HeaderMap, HeaderValue};
    use iri_rs::IriRefBuf;
    use mediatype::MediaTypeBuf;
    use pretty_assertions::assert_eq;

    fn targets<'a>(values: impl Iterator<Item = &'a LinkValue>) -> Vec<&'a str> {
        values.map(|value| value.target().as_str()).collect()
    }

    #[test]
    fn the_core_context_link_names_the_core_context_as_a_json_ld_context() {
        let link = Link::core_context().unwrap();
        let [value] = link.values() else { panic!("one link-value") };
        assert!(value.is_context_link());
        assert_eq!(value.target().as_str(), CORE_CONTEXT.as_str());
        let mut headers = HeaderMap::new();
        // The map takes a header of its own; the original is kept to compare the decoded one with.
        headers.typed_insert(link.clone());
        assert_eq!(headers.typed_get::<Link>(), Some(link));
    }

    #[test]
    fn the_json_ld_context_link_renders_exactly_as_clause_6_3_5_shows() {
        let link = Link::json_ld_context(IriRefBuf::new("https://example.org/context.jsonld").unwrap()).unwrap();
        let expected = r#"<https://example.org/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json""#;
        let mut encoded = Vec::new();
        link.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static(expected)]);
        assert_eq!(link.to_string(), expected);
        assert_eq!(Link::decode(&mut encoded.iter()).ok(), Some(link));
    }

    #[test]
    fn the_annex_c_pagination_link_decodes_into_its_next_target_and_type() {
        let link =
            decode_line::<Link>(r#"</ngsi-ld/v1/entities/?type=Vehicle&format=simplified&limit=2&offset=2>; rel="next"; type="application/ld+json""#).unwrap();
        assert_eq!(
            targets(link.find_by_rel(&RelationType::Next)),
            vec!["/ngsi-ld/v1/entities/?type=Vehicle&format=simplified&limit=2&offset=2"]
        );
        assert_eq!(link.values()[0].media_type().map(MediaTypeBuf::as_str), Some("application/ld+json"));
        assert_eq!(link.context_urls().count(), 0);
    }

    #[test]
    fn a_pagination_target_with_a_raw_q_query_stays_one_link_value() {
        let link = decode_line::<Link>(
            r#"</ngsi-ld/v1/entities?type=Vehicle&q=speed>50;brandName!="Mercedes">; rel="next", </ngsi-ld/v1/entities?type=Vehicle&offset=0>; rel="prev""#,
        )
        .unwrap();
        assert_eq!(
            targets(link.find_by_rel(&RelationType::Next)),
            vec!["/ngsi-ld/v1/entities?type=Vehicle&q=speed%3E50;brandName!=%22Mercedes%22"]
        );
        assert_eq!(
            targets(link.find_by_rel(&RelationType::Prev)),
            vec!["/ngsi-ld/v1/entities?type=Vehicle&offset=0"]
        );
    }

    #[test]
    fn a_comma_inside_a_target_does_not_start_a_new_link_value() {
        let link = decode_line::<Link>(r#"</ngsi-ld/v1/entities?type=Vehicle&attrs=speed,brandName&limit=2>; rel="next""#).unwrap();
        assert_eq!(
            targets(link.values().iter()),
            vec!["/ngsi-ld/v1/entities?type=Vehicle&attrs=speed,brandName&limit=2"]
        );
        assert_eq!(link.values()[0].rels(), [RelationType::Next]);
    }

    #[test]
    fn several_link_lines_and_comma_separated_values_concatenate_in_order() {
        let lines = [
            HeaderValue::from_static(r#"</page/2>; rel="next", </page/0>; rel="prev""#),
            HeaderValue::from_static("<https://example.org/ctx.jsonld>; rel=\"http://www.w3.org/ns/json-ld#context\""),
        ];
        let link = Link::decode(&mut lines.iter()).unwrap();
        assert_eq!(targets(link.values().iter()), vec!["/page/2", "/page/0", "https://example.org/ctx.jsonld"]);
        assert_eq!(
            link.context_urls().map(IriRefBuf::as_str).collect::<Vec<_>>(),
            vec!["https://example.org/ctx.jsonld"]
        );
    }

    #[test]
    fn empty_list_elements_are_skipped_but_an_empty_header_is_rejected() {
        assert_eq!(decode_line::<Link>(r#", </a>; rel="next", ,"#).map(|link| link.values().len()), Some(1));
        assert_eq!(decode_line::<Link>(""), None);
        assert_eq!(decode_line::<Link>(" , "), None);
        assert!(matches!("".parse::<Link>(), Err(LinkParseError::Empty)));
    }

    #[test]
    fn a_malformed_link_value_rejects_the_header() {
        assert_eq!(decode_line::<Link>(r#"http://example.org/; rel="next""#), None);
        assert_eq!(decode_line::<Link>(r#"</a>; rel="next", garbage"#), None);
    }

    #[test]
    fn a_lenient_read_keeps_every_readable_value_and_counts_the_others() {
        let read = read_line(&HeaderValue::from_static(
            r#"</page/2>; rel="next", garbage, </page/0>; rel="prev", </a>; rel="%zz""#,
        ));
        assert_eq!(targets(read.values.iter()), vec!["/page/2", "/page/0"]);
        assert_eq!(read.unread, 2);
        let opaque = HeaderValue::from_bytes(b"</caf\xc3\xa9>").unwrap();
        assert_eq!(read_line(&opaque).unread, 1);
    }

    #[test]
    fn values_render_comma_separated_and_parse_back() {
        let link = Link::from_values(vec![
            LinkValue::builder(IriRefBuf::new("/next").unwrap()).rel(RelationType::Next).build(),
            LinkValue::builder(IriRefBuf::new("/prev").unwrap()).rel(RelationType::Prev).build(),
        ])
        .unwrap();
        assert_eq!(link.to_string(), r#"</next>; rel="next", </prev>; rel="prev""#);
        assert_eq!(link.to_string().parse::<Link>().ok(), Some(link));
    }

    #[test]
    fn a_link_without_values_or_with_a_non_ascii_target_cannot_be_built() {
        assert!(matches!(Link::from_values(Vec::new()), Err(LinkBuildError::Empty)));
        let non_ascii = LinkValue::builder(IriRefBuf::new("/entities/caf\u{e9}").unwrap())
            .rel(RelationType::Next)
            .build();
        assert!(matches!(Link::new(non_ascii), Err(LinkBuildError::Unencodable(_))));
    }

    #[test]
    fn a_non_ascii_target_read_from_text_is_percent_encoded() {
        let link = "</entities/caf\u{e9}>; rel=\"next\"".parse::<Link>().unwrap();
        assert_eq!(targets(link.values().iter()), vec!["/entities/caf%C3%A9"]);
    }
}
