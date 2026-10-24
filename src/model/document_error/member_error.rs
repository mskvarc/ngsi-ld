//! The problems every reader of the crate can meet in a member: a value of the wrong JSON type, a missing,
//! repeated, unknown or empty member, and a value that is not a valid NGSI-LD Value, name, identifier, URL, scope,
//! timestamp, geometry, duration, context source alias or `@context`.

use crate::model::{ContextError, ContextSourceAliasError, DurationError, NameError, ScopeError, TemporalError, UriError, ValueError, geometry::GeometryError};

/// What is wrong with a member, whatever the document; both
/// [`DocumentErrorKind`](crate::model::DocumentErrorKind) and
/// [`ReadErrorKind`](crate::subscription::ReadErrorKind) hold it.
///
/// Readers of future versions of the specification may meet new problems, so the enum is `#[non_exhaustive]`.
///
/// ```
/// use ngsi_ld::model::{DocumentErrorKind, Entity, JsonType, Lossless, MemberError, Output};
///
/// let error = Entity::<Output>::parse(
///     r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":3}"#,
///     Lossless::Normalized,
/// )
/// .err();
/// assert!(matches!(
///     error.map(|error| error.kind),
///     Some(DocumentErrorKind::Member(MemberError::WrongShape {
///         member: "type",
///         expected: JsonType::String
///     }))
/// ));
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MemberError {
    /// The text is not JSON.
    #[error("the document is not valid JSON")]
    NotJson,
    /// A JSON object was expected.
    #[error("expected a JSON object")]
    NotAnObject,
    /// A JSON array was expected.
    #[error("expected a JSON array")]
    NotAnArray,
    /// A member has the wrong JSON type.
    #[error("`{member}` must be {expected}")]
    WrongShape {
        /// The member.
        member: &'static str,
        /// What its value must be.
        expected: JsonType,
    },
    /// A required member is missing.
    #[error("missing member `{member}`")]
    MissingMember {
        /// The member.
        member: &'static str,
    },
    /// A member appears twice.
    #[error("member `{member}` appears twice")]
    DuplicateMember {
        /// The member, as written.
        member: String,
    },
    /// A member is not defined for this document, or only a broker writes it and the document is a request.
    #[error("member `{member}` is not allowed here")]
    UnexpectedMember {
        /// The member, as written.
        member: String,
    },
    /// A list or text the specification forbids to be empty is empty, such as the array of a member that holds one
    /// value or several.
    #[error("the member cannot be empty")]
    Empty,
    /// A member's value is not one of the tokens the specification allows, such as a `format` or a trigger.
    #[error("`{found}` is not a valid value of `{member}`")]
    InvalidToken {
        /// The member.
        member: &'static str,
        /// The value, as written.
        found: String,
    },
    /// A member whose value is any NGSI-LD Value (clause 3.1), such as the `value` of a Property or a Context Source
    /// Property of a registration (clause 4.9), holds `null` or an invalid value.
    #[error("invalid value")]
    Value {
        /// Why.
        #[source]
        source: ValueError,
    },
    /// A name or term is invalid.
    #[error("invalid name")]
    Name {
        /// Why.
        #[source]
        source: NameError,
    },
    /// An identifier is not a URI.
    #[error("invalid identifier")]
    Identifier {
        /// Why.
        #[source]
        source: UriError,
    },
    /// A dereferenceable URI is not a URL.
    #[error("invalid URL")]
    Url {
        /// Why.
        #[source]
        source: url::ParseError,
    },
    /// A scope is invalid.
    #[error("invalid scope")]
    Scope {
        /// Why.
        #[source]
        source: ScopeError,
    },
    /// A timestamp is invalid.
    #[error("invalid timestamp")]
    Temporal {
        /// Why.
        #[source]
        source: Box<TemporalError>,
    },
    /// A geometry is invalid.
    #[error("invalid geometry")]
    Geometry {
        /// Why.
        #[source]
        source: GeometryError,
    },
    /// A duration is not an ISO 8601 duration (clause 4.5.19.1).
    #[error("invalid duration")]
    Duration {
        /// Why.
        #[source]
        source: DurationError,
    },
    /// A context source alias is not an IETF RFC 7230 token (Tables 5.2.9-1 and 5.2.40-1).
    #[error("invalid context source alias")]
    ContextSourceAlias {
        /// Why.
        #[source]
        source: ContextSourceAliasError,
    },
    /// An `@context` is invalid.
    #[error("invalid @context")]
    Context {
        /// Why.
        #[source]
        source: ContextError,
    },
}

/// The JSON type a member's value must have.
///
/// A future reader may require another type, so the enum is `#[non_exhaustive]`.
///
/// ```
/// use ngsi_ld::model::JsonType;
///
/// assert_eq!(
///     JsonType::Count.to_string(),
///     "a whole number of zero or more"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display)]
#[non_exhaustive]
pub enum JsonType {
    /// A string.
    #[strum(serialize = "a string")]
    String,
    /// A boolean.
    #[strum(serialize = "a boolean")]
    Boolean,
    /// A number.
    #[strum(serialize = "a number")]
    Number,
    /// A number that is a whole number of zero or more: a count.
    #[strum(serialize = "a whole number of zero or more")]
    Count,
    /// A whole number.
    #[strum(serialize = "a whole number")]
    Integer,
}
