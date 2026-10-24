// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! One link-value of a `Link` header (RFC 8288 section 3).
//!
//! This file is derived from the disabled `Link` implementation in <https://github.com/hyperium/headers> and
//! includes local modifications.

use crate::{
    http::headers::{
        LinkParameter,
        LinkParseError,
        RelationType,
        link::quoting::Elements,
        quoted_string::{MalformedQuotedString, parameter_value, write_quoted},
    },
    model::APPLICATION_LD_JSON,
};
use iri_rs::{IriBuf, IriRefBuf};
use mediatype::MediaTypeBuf;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use std::{fmt, str::FromStr};

/// The ASCII characters an IRI cannot hold anywhere (RFC 3987 section 2.2); a target written with them verbatim is
/// read with them percent-encoded.
const NOT_IN_IRI: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'\\')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// The ASCII characters an IRI query or fragment cannot hold. `[` and `]` are allowed only in the host, but a `q`
/// query writes them verbatim for compound value paths (NGSI-LD clause 4.9).
const NOT_IN_IRI_QUERY: &AsciiSet = &NOT_IN_IRI.add(b'[').add(b']');

/// One link-value: a target and the parameters NGSI-LD reads from it (RFC 8288 section 3).
///
/// The target is an IRI reference because pagination links are relative (`</ngsi-ld/v1/entities?offset=2>`), while
/// context links are absolute. Of the parameters, `rel`, `type` and `profile` are kept; every other one is ignored.
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::{
///     http::headers::{LinkValue, RelationType},
///     model::APPLICATION_JSON,
/// };
///
/// let next = LinkValue::builder(IriRefBuf::new(
///     "/ngsi-ld/v1/entities?type=Vehicle&offset=2",
/// )?)
/// .rel(RelationType::Next)
/// .media_type(APPLICATION_JSON)
/// .build();
/// assert_eq!(
///     next.to_string(),
///     r#"</ngsi-ld/v1/entities?type=Vehicle&offset=2>; rel="next"; type="application/json""#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, bon::Builder)]
pub struct LinkValue {
    /// The link target.
    #[builder(start_fn)]
    target: IriRefBuf,
    /// The relation types of `rel`, in order; empty when the parameter is absent.
    #[builder(field)]
    rels: Vec<RelationType>,
    /// The `type` parameter: a hint on the media type of the target.
    #[builder(into)]
    media_type: Option<MediaTypeBuf>,
    /// The `profile` parameter.
    #[builder(into)]
    profile: Option<IriBuf>,
}

impl<S: link_value_builder::State> LinkValueBuilder<S> {
    /// Adds a relation type to the `rel` parameter.
    pub fn rel(mut self, rel: RelationType) -> Self {
        self.rels.push(rel);
        self
    }
}

impl LinkValue {
    /// The JSON-LD context link-value naming `context` (NGSI-LD clause 6.3.5): the relation
    /// `http://www.w3.org/ns/json-ld#context` and the type `application/ld+json`.
    #[must_use]
    pub fn json_ld_context(context: IriRefBuf) -> LinkValue {
        LinkValue::context_value(context, None)
    }

    /// The JSON-LD context link-value naming `context` with `profile` as its `profile` parameter (clause 6.3.21).
    pub(crate) fn profiled_json_ld_context(context: IriRefBuf, profile: IriBuf) -> LinkValue {
        LinkValue::context_value(context, Some(profile))
    }

    /// The JSON-LD context link-value naming `context`, with `profile` when there is one.
    fn context_value(context: IriRefBuf, profile: Option<IriBuf>) -> LinkValue {
        LinkValue::builder(context)
            .rel(RelationType::JsonLdContext)
            .media_type(APPLICATION_LD_JSON)
            .maybe_profile(profile)
            .build()
    }

    /// The link target.
    #[must_use]
    pub const fn target(&self) -> &IriRefBuf {
        &self.target
    }

    /// The relation types of the `rel` parameter, in order; empty when it is absent.
    #[must_use]
    pub fn rels(&self) -> &[RelationType] {
        &self.rels
    }

    /// The `type` parameter, if present.
    #[must_use]
    pub const fn media_type(&self) -> Option<&MediaTypeBuf> {
        self.media_type.as_ref()
    }

    /// The `profile` parameter, if present.
    #[must_use]
    pub const fn profile(&self) -> Option<&IriBuf> {
        self.profile.as_ref()
    }

    /// Whether the `rel` parameter names `rel`.
    #[must_use]
    pub fn has_rel(&self, rel: &RelationType) -> bool {
        self.rels.contains(rel)
    }

    /// Whether this is a JSON-LD context link.
    #[must_use]
    pub fn is_context_link(&self) -> bool {
        self.rels.iter().any(RelationType::is_json_ld_context)
    }

    /// Applies one `name=value` parameter.
    ///
    /// A parameter already set keeps its first value: RFC 8288 section 3.3 makes the first `rel` win, and the same
    /// rule is applied to `type` and `profile`. A parameter this crate does not read is ignored (section 3.4.1).
    fn apply_parameter(&mut self, parameter: &str) -> Result<(), LinkParseError> {
        let (name, raw_value) = parameter.split_once('=').map_or((parameter, None), |(name, value)| (name, Some(value)));
        let name = name.trim();
        if name.is_empty() {
            return Err(LinkParseError::ParameterWithoutName);
        }
        let Ok(known) = name.parse::<LinkParameter>() else {
            return Ok(());
        };
        let raw_value = raw_value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(LinkParseError::MissingValue(known))?;
        let value = parameter_value(raw_value).map_err(|MalformedQuotedString| LinkParseError::MalformedQuotedString)?;
        match known {
            LinkParameter::Rel if self.rels.is_empty() => {
                self.rels = value
                    .split_ascii_whitespace()
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .map_err(LinkParseError::InvalidRelationType)?;
                if self.rels.is_empty() {
                    return Err(LinkParseError::MissingValue(known));
                }
            }
            LinkParameter::Type if self.media_type.is_none() => {
                self.media_type = Some(value.parse().map_err(LinkParseError::InvalidMediaType)?);
            }
            LinkParameter::Profile if self.profile.is_none() => {
                self.profile = Some(IriBuf::new(value).map_err(LinkParseError::InvalidProfile)?);
            }
            LinkParameter::Rel | LinkParameter::Type | LinkParameter::Profile => {}
        }
        Ok(())
    }
}

/// Reads a target, percent-encoding the characters an IRI cannot hold so that a `q` query written verbatim still
/// makes a usable reference.
fn parse_target(raw: &str) -> Result<IriRefBuf, LinkParseError> {
    let target: String = match raw.split_once('?') {
        Some((before_query, query)) => utf8_percent_encode(before_query, NOT_IN_IRI)
            .chain(["?"])
            .chain(utf8_percent_encode(query, NOT_IN_IRI_QUERY))
            .collect(),
        None => utf8_percent_encode(raw, NOT_IN_IRI).collect(),
    };
    IriRefBuf::new(target).map_err(LinkParseError::InvalidTarget)
}

// A bracketed target followed by `;`-separated parameters, which a derive cannot express.
impl FromStr for LinkValue {
    type Err = LinkParseError;

    /// Reads one link-value: a `<...>` target, then `;`-separated parameters. Blank parameters are skipped.
    fn from_str(element: &str) -> Result<LinkValue, LinkParseError> {
        let mut parts = Elements::new(element, b';');
        let target = parts
            .next()
            .map(str::trim)
            .and_then(|target| target.strip_prefix('<')?.strip_suffix('>'))
            .ok_or(LinkParseError::UnbracketedTarget)?;
        let mut value = LinkValue {
            target: parse_target(target.trim())?,
            rels: Vec::new(),
            media_type: None,
            profile: None,
        };
        for parameter in parts.filter(|parameter| !parameter.trim().is_empty()) {
            value.apply_parameter(parameter)?;
        }
        Ok(value)
    }
}

// The parameters are written only when present, and the `type` value is escaped, which no derive can express.
impl fmt::Display for LinkValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.target)?;
        // Relation types and IRIs cannot hold `"` or `\`, so they need no escaping inside the quotes.
        if let Some((first, rest)) = self.rels.split_first() {
            write!(f, "; rel=\"{first}")?;
            rest.iter().try_for_each(|rel| write!(f, " {rel}"))?;
            f.write_str("\"")?;
        }
        if let Some(media_type) = &self.media_type {
            f.write_str("; type=")?;
            write_quoted(f, media_type.as_str())?;
        }
        if let Some(profile) = &self.profile {
            write!(f, "; profile=\"{profile}\"")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{LinkParseError, LinkValue, RelationType},
        model::APPLICATION_JSON,
    };
    use iri_rs::{IriBuf, IriRefBuf};
    use mediatype::MediaTypeBuf;
    use pretty_assertions::assert_eq;

    fn target(raw: &str) -> IriRefBuf {
        IriRefBuf::new(raw).unwrap()
    }

    #[test]
    fn a_json_ld_context_value_carries_the_context_relation_and_media_type() {
        let value = LinkValue::json_ld_context(target("https://example.org/context.jsonld"));
        assert_eq!(
            value.to_string(),
            r#"<https://example.org/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json""#
        );
        assert!(value.is_context_link());
    }

    #[test]
    fn a_relation_the_value_carries_is_reported_and_others_are_not() {
        let value = LinkValue::builder(target("/page")).rel(RelationType::Next).build();
        assert!(value.has_rel(&RelationType::Next));
        assert!(!value.has_rel(&RelationType::Prev));
        assert!(!value.is_context_link());
    }

    #[test]
    fn every_parameter_renders_and_parses_back() {
        let value = LinkValue::builder(target("https://example.org/context.jsonld"))
            .rel(RelationType::JsonLdContext)
            .rel(RelationType::Alternate)
            .media_type(APPLICATION_JSON)
            .profile(IriBuf::new("https://uri.etsi.org/ngsi-ld/profile/1.9").unwrap())
            .build();
        let rendered = value.to_string();
        assert_eq!(
            rendered,
            r#"<https://example.org/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context alternate"; type="application/json"; profile="https://uri.etsi.org/ngsi-ld/profile/1.9""#
        );
        assert_eq!(rendered.parse::<LinkValue>().ok(), Some(value));
    }

    #[test]
    fn a_media_type_with_a_quoted_parameter_is_escaped_and_reads_back() {
        let value = LinkValue::builder(target("/a"))
            .media_type("text/plain; note=\"a b\"".parse::<MediaTypeBuf>().unwrap())
            .build();
        assert_eq!(value.to_string(), r#"</a>; type="text/plain; note=\"a b\"""#);
        assert_eq!(value.to_string().parse::<LinkValue>().ok(), Some(value));
    }

    #[test]
    fn the_first_occurrence_of_a_parameter_wins() {
        let value = r#"</page>; rel="next"; rel="prev"; type="application/json"; type="text/html""#.parse::<LinkValue>().unwrap();
        assert_eq!(value.rels(), [RelationType::Next]);
        assert_eq!(value.media_type().map(MediaTypeBuf::as_str), Some("application/json"));
    }

    #[test]
    fn unknown_parameters_are_ignored_and_names_are_case_insensitive() {
        let value = r#"</page>; title="Chapter 2"; hreflang=en; REL=next; anchor"#.parse::<LinkValue>().unwrap();
        assert_eq!(value.rels(), [RelationType::Next]);
        assert_eq!(value.to_string(), r#"</page>; rel="next""#);
    }

    #[test]
    fn a_target_with_a_raw_query_is_percent_encoded_into_an_iri_reference() {
        let value = r#"</ngsi-ld/v1/entities?type=Vehicle&q=speed>50;brandName!="Mercedes"|address[city]=="Bonn">; rel="next""#
            .parse::<LinkValue>()
            .unwrap();
        assert_eq!(
            value.target().as_str(),
            "/ngsi-ld/v1/entities?type=Vehicle&q=speed%3E50;brandName!=%22Mercedes%22%7Caddress%5Bcity%5D==%22Bonn%22"
        );
        assert_eq!(value.rels(), [RelationType::Next]);
    }

    #[test]
    fn a_bracketed_host_is_kept() {
        assert_eq!(
            "<http://[::1]/entities?a=[b]>"
                .parse::<LinkValue>()
                .map(|value| value.target().as_str().to_owned())
                .ok(),
            Some("http://[::1]/entities?a=%5Bb%5D".to_owned())
        );
    }

    #[test]
    fn a_value_without_a_bracketed_target_is_rejected() {
        assert!(matches!(
            r#"http://example.org/; rel="next""#.parse::<LinkValue>(),
            Err(LinkParseError::UnbracketedTarget)
        ));
        assert!(matches!("</a; rel=next".parse::<LinkValue>(), Err(LinkParseError::UnbracketedTarget)));
    }

    #[test]
    fn a_malformed_known_parameter_is_rejected() {
        assert!(matches!(r#"</a>; ="next""#.parse::<LinkValue>(), Err(LinkParseError::ParameterWithoutName)));
        assert!(matches!("</a>; rel=".parse::<LinkValue>(), Err(LinkParseError::MissingValue(_))));
        assert!(matches!("</a>; rel=\"  \"".parse::<LinkValue>(), Err(LinkParseError::MissingValue(_))));
        assert!(matches!("</a>; type".parse::<LinkValue>(), Err(LinkParseError::MissingValue(_))));
        assert!(matches!(r#"</a>; rel="next"#.parse::<LinkValue>(), Err(LinkParseError::MalformedQuotedString)));
        assert!(matches!(
            r#"</a>; type="not a media type""#.parse::<LinkValue>(),
            Err(LinkParseError::InvalidMediaType(_))
        ));
        assert!(matches!(
            r#"</a>; profile="/relative""#.parse::<LinkValue>(),
            Err(LinkParseError::InvalidProfile(_))
        ));
        assert!(matches!(r#"</a>; rel="%zz""#.parse::<LinkValue>(), Err(LinkParseError::InvalidRelationType(_))));
        assert!(matches!(
            "<http://exa mple.org/%zz>".parse::<LinkValue>(),
            Err(LinkParseError::InvalidTarget(_))
        ));
    }
}
