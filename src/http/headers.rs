//! Typed NGSI-LD HTTP headers (NGSI-LD clause 6.3).
//!
//! Every header here is a [`headers::Header`] implementation over a validated type, so the request pipeline never
//! reads or writes a header value as text. Each type guarantees that its encoding exists before
//! [`headers::Header::encode`] runs: either it encodes from a static or numeric value, or from a media type without
//! parameters, whose names are restricted names (RFC 6838 section 4.2), none of which can fail, or its constructor
//! renders the value and reports a typed error when the rendering is not a valid field value. Encoding therefore
//! never drops a value.
//!
//! Decoding follows the `headers` crate: a value this crate cannot read is reported as [`headers::Error`], which
//! carries no detail. A header a successful response may carry but does not depend on is read into an
//! [`OptionalHeader`], so a malformed one is reported beside the success instead of
//! replacing it.
//!
//! Besides the NGSI-LD headers, the general HTTP headers a client builds or a mock server reads are typed here too,
//! each with the RFC it follows:
//!
//! - [`Accept`]: weighted media ranges (RFC 9110 section 12.5.1, NGSI-LD clause 6.3.4).
//! - [`AcceptEncoding`]: weighted content codings (RFC 9110 section 12.5.3).
//! - [`AcceptLanguage`]: weighted language ranges (RFC 9110 section 12.5.4,
//!   NGSI-LD clause 4.15).
//! - [`AltSvc`]: alternative services (RFC 7838).
//! - [`Forwarded`]: what proxies learnt of a request (RFC 7239).
//! - [`Via`]: the hops a message passed through (RFC 9110 section 7.6.3, NGSI-LD clause 6.3.18).
//! - [`XForwardedProto`]: the scheme a client used to reach a proxy, the
//!   de-facto form of the `proto` parameter of `Forwarded`.
//!
//! Headers the `headers` crate already types, such as `ETag`, `Vary`, `Allow`, `Retry-After` or `Content-Length`, are
//! used from it directly.
//!
//! # Content negotiation
//!
//! [`Accept`], [`AcceptEncoding`], [`AcceptLanguage`], [`BasicLanguageRange`], [`BasicLanguageRangeError`],
//! [`ContentType`], [`MediaRange`], [`MediaRangeError`], [`ContentCoding`], [`ContentCodingError`], [`CodingRange`],
//! [`Prefer`], [`PreferError`], [`GeoJsonBody`], [`PreferenceApplied`].
//!
//! # NGSI-LD fields
//!
//! [`NgsiLdTenant`], [`NgsiLdResultsCount`], [`NgsiLdEntityMap`], [`NgsiLdSnapshot`], [`NgsiLdWarnings`],
//! [`NgsiLdWarning`], [`WarningCode`], [`SpecVersion`], [`SpecVersionError`], [`ContentRange`], [`ContentRangeSize`],
//! [`Location`].
//!
//! # Links
//!
//! [`Link`], [`LinkValue`], [`RelationType`], [`LinkParameter`], [`LinkParseError`], [`LinkBuildError`].
//!
//! # Proxies and forwarding
//!
//! [`Forwarded`], [`ForwardedElement`], [`ForwardedElementError`], [`ForwardedParameter`], [`ForwardedHost`],
//! [`ForwardedHostError`], [`Node`], [`NodeName`], [`NodePort`], [`NodeError`], [`ObfuscatedIdentifier`],
//! [`ObfuscatedIdentifierError`], [`Via`], [`ViaEntry`], [`ViaEntryError`], [`ReceivedBy`], [`ReceivedByError`],
//! [`ReceivedProtocol`], [`ReceivedProtocolError`], [`ViaComment`], [`ViaCommentError`], [`XForwardedProto`],
//! [`Scheme`], [`SchemeError`].
//!
//! # Alternative services
//!
//! [`AltSvc`], [`Alternative`], [`AlternativeError`], [`AltAuthority`], [`AltAuthorityError`], [`Persistence`].
//!
//! # Field values
//!
//! [`OptionalHeader`], [`InvalidFieldValue`], [`ListFieldError`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::http::headers::{
    accept::Accept,
    accept_encoding::AcceptEncoding,
    accept_language::{
        AcceptLanguage,
        language_range::{BasicLanguageRange, BasicLanguageRangeError},
    },
    alt_svc::{
        AltSvc,
        alternative::{AltAuthority, AltAuthorityError, Alternative, AlternativeError, Persistence},
    },
    content_coding::{CodingRange, ContentCoding, ContentCodingError},
    content_range::{ContentRange, ContentRangeSize},
    content_type::ContentType,
    field_list::ListFieldError,
    field_value::InvalidFieldValue,
    forwarded::{
        Forwarded,
        element::{ForwardedElement, ForwardedElementError, ForwardedParameter},
        host::{ForwardedHost, ForwardedHostError},
        node::{Node, NodeError, NodeName, NodePort, ObfuscatedIdentifier, ObfuscatedIdentifierError},
    },
    link::{
        error::{LinkBuildError, LinkParameter, LinkParseError},
        header::Link,
        relation_type::RelationType,
        value::LinkValue,
    },
    location::Location,
    media_range::{MediaRange, MediaRangeError},
    ngsild_entity_map::NgsiLdEntityMap,
    ngsild_results_count::NgsiLdResultsCount,
    ngsild_snapshot::NgsiLdSnapshot,
    ngsild_tenant::NgsiLdTenant,
    ngsild_warning::{NgsiLdWarning, NgsiLdWarnings, WarningCode},
    optional_header::OptionalHeader,
    prefer::{GeoJsonBody, Prefer, PreferError},
    preference_applied::PreferenceApplied,
    scheme::{Scheme, SchemeError},
    spec_version::{SpecVersion, SpecVersionError},
    via::{
        Via,
        comment::{ViaComment, ViaCommentError},
        entry::{ViaEntry, ViaEntryError},
        received_by::{ReceivedBy, ReceivedByError},
        received_protocol::{ReceivedProtocol, ReceivedProtocolError},
    },
    x_forwarded_proto::XForwardedProto,
};

mod accept;
mod accept_encoding;
mod accept_language;
mod alt_svc;
// Only the loader of remote contexts reads a response from outside NGSI-LD.
#[cfg(feature = "linked-data")]
pub(crate) mod any_content_type;
pub mod builders;
mod content_coding;
mod content_range;
mod content_type;
mod field_list;
mod field_value;
mod forwarded;
mod host_port;
pub(crate) mod link;
mod location;
mod media_range;
mod ngsild_entity_map;
mod ngsild_results_count;
mod ngsild_snapshot;
mod ngsild_tenant;
pub(crate) mod ngsild_warning;
mod optional_header;
mod prefer;
mod preference;
mod preference_applied;
mod quoted_string;
mod scheme;
mod spec_version;
mod text_field;
mod via;
mod weight;
mod x_forwarded_proto;
