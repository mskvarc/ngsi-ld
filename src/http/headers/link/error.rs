// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The ways reading or building a `Link` header can fail.
//!
//! This file supports the adapted `Link` implementation derived from <https://github.com/hyperium/headers> and is
//! distributed under the MIT License.

use crate::http::headers::InvalidFieldValue;
use iri_rs::InvalidIri;
use mediatype::MediaTypeError;

/// A link parameter this crate reads (RFC 8288 section 3.4).
///
/// Non-exhaustive: further parameters may be read in later versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
#[non_exhaustive]
pub enum LinkParameter {
    /// `rel`: the relation types.
    Rel,
    /// `type`: a hint on the media type of the target.
    Type,
    /// `profile`: the profile of the target.
    Profile,
}

/// Why a string is not a `Link` header value.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LinkParseError {
    /// The header holds no link-value.
    #[error("a Link header must carry at least one link-value")]
    Empty,
    /// A link-value does not start with a target enclosed in angle brackets.
    #[error("a link-value must start with a target enclosed in angle brackets")]
    UnbracketedTarget,
    /// The target is not an IRI reference, even after the characters an IRI cannot hold were percent-encoded.
    #[error("the link target is not an IRI reference")]
    InvalidTarget(#[source] InvalidIri<String>),
    /// A parameter has a value but no name.
    #[error("a link parameter has no name")]
    ParameterWithoutName,
    /// A parameter this crate reads has no value.
    #[error("the {0} link parameter has no value")]
    MissingValue(LinkParameter),
    /// A quoted parameter value is not closed, or is followed by more text.
    #[error("a quoted link parameter value is malformed")]
    MalformedQuotedString,
    /// The `rel` parameter names a relation type that is neither a registered name nor an IRI reference.
    #[error("the rel link parameter names an invalid relation type")]
    InvalidRelationType(#[source] InvalidIri<String>),
    /// The `type` parameter is not a media type.
    #[error("the type link parameter is not a media type")]
    InvalidMediaType(#[source] MediaTypeError),
    /// The `profile` parameter is not an absolute IRI.
    #[error("the profile link parameter is not an absolute IRI")]
    InvalidProfile(#[source] InvalidIri<String>),
    /// The parsed header cannot be written back into a header field.
    #[error(transparent)]
    Unencodable(#[from] InvalidFieldValue),
}

/// Why a `Link` header cannot be built.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LinkBuildError {
    /// No link-value was given.
    #[error("a Link header must carry at least one link-value")]
    Empty,
    /// The rendered header cannot be carried in a header field.
    #[error(transparent)]
    Unencodable(#[from] InvalidFieldValue),
}
