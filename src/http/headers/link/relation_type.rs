// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! Link relation types (RFC 8288 section 2.1).
//!
//! This file is derived from the disabled `Link` implementation in <https://github.com/hyperium/headers> and
//! includes local modifications.

use iri_rs::{InvalidIri, Iri, IriRefBuf, iri};
use std::str::FromStr;

/// The JSON-LD context relation (JSON-LD 1.1 section 6.1).
const JSON_LD_CONTEXT: Iri<&'static str> = iri!("http://www.w3.org/ns/json-ld#context");

/// A link relation type (RFC 8288 section 2.1).
///
/// The relation types NGSI-LD reads have named variants; every other registered name or extension IRI is carried
/// verbatim in [`RelationType::ExtRelType`]. Registered names and the JSON-LD context relation are recognised
/// whatever their case (RFC 8288 sections 2.1.1 and 2.1.2). Non-exhaustive: further relation types may get named
/// variants in later versions.
///
/// ```
/// use ngsi_ld::http::headers::RelationType;
///
/// assert_eq!(
///     "NEXT".parse::<RelationType>().ok(),
///     Some(RelationType::Next)
/// );
/// assert_eq!(
///     RelationType::JsonLdContext.to_string(),
///     "http://www.w3.org/ns/json-ld#context"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum RelationType {
    /// `next`: the next page of a paginated result (NGSI-LD clause 6.3.10).
    #[display("next")]
    Next,
    /// `prev`: the previous page of a paginated result (NGSI-LD clause 6.3.10).
    #[display("prev")]
    Prev,
    /// `first`: the first page of a paginated result.
    #[display("first")]
    First,
    /// `last`: the last page of a paginated result.
    #[display("last")]
    Last,
    /// `profile`: the target names a profile of the representation (RFC 6906).
    #[display("profile")]
    Profile,
    /// `alternate`: the target is an alternate representation of the resource (RFC 8288 section 6.2.2).
    #[display("alternate")]
    Alternate,
    /// `http://www.w3.org/ns/json-ld#context`: the target is the JSON-LD context of the document (NGSI-LD clause
    /// 6.3.5).
    #[display("{JSON_LD_CONTEXT}")]
    JsonLdContext,
    /// Any other relation type: a registered name or an extension IRI, as written.
    #[display("{_0}")]
    ExtRelType(IriRefBuf),
}

impl RelationType {
    /// Whether this is the JSON-LD context relation.
    #[must_use]
    pub const fn is_json_ld_context(&self) -> bool {
        matches!(self, RelationType::JsonLdContext)
    }
}

// Named relation types compare case-insensitively and everything else falls back to an IRI reference, which
// `strum::EnumString` cannot express.
impl FromStr for RelationType {
    type Err = InvalidIri<String>;

    fn from_str(raw: &str) -> Result<RelationType, InvalidIri<String>> {
        let named = [
            RelationType::Next,
            RelationType::Prev,
            RelationType::First,
            RelationType::Last,
            RelationType::Profile,
            RelationType::Alternate,
            RelationType::JsonLdContext,
        ]
        .into_iter()
        .find(|relation| relation.to_string().eq_ignore_ascii_case(raw));
        match named {
            Some(relation) => Ok(relation),
            None => IriRefBuf::new(raw).map(RelationType::ExtRelType),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::RelationType;
    use iri_rs::IriRefBuf;
    use pretty_assertions::assert_eq;

    #[test]
    fn each_named_relation_renders_and_parses_back_whatever_its_case() {
        for (relation, token) in [
            (RelationType::Next, "next"),
            (RelationType::Prev, "prev"),
            (RelationType::First, "first"),
            (RelationType::Last, "last"),
            (RelationType::Profile, "profile"),
            (RelationType::Alternate, "alternate"),
        ] {
            assert_eq!(relation.to_string(), token);
            assert_eq!(token.parse::<RelationType>().ok(), Some(relation.clone()));
            assert_eq!(token.to_uppercase().parse::<RelationType>().ok(), Some(relation));
        }
    }

    #[test]
    fn the_json_ld_context_relation_is_recognised_whatever_its_case() {
        for spelling in ["http://www.w3.org/ns/json-ld#context", "HTTP://WWW.W3.ORG/NS/JSON-LD#CONTEXT"] {
            assert_eq!(spelling.parse::<RelationType>().ok(), Some(RelationType::JsonLdContext), "{spelling}");
        }
        assert!(RelationType::JsonLdContext.is_json_ld_context());
        assert!(!RelationType::Next.is_json_ld_context());
    }

    #[test]
    fn an_unnamed_relation_is_carried_verbatim() {
        let relation = "https://example.org/rel/describes".parse::<RelationType>().unwrap();
        assert_eq!(relation, RelationType::ExtRelType(IriRefBuf::new("https://example.org/rel/describes").unwrap()));
        assert_eq!(relation.to_string(), "https://example.org/rel/describes");
        assert!(!"http://www.w3.org/ns/json-ld#contexts".parse::<RelationType>().unwrap().is_json_ld_context());
    }

    #[test]
    fn a_relation_that_is_not_an_iri_reference_is_rejected() {
        assert!("a b".parse::<RelationType>().is_err());
    }
}
