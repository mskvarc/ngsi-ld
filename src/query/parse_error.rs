//! Why a string is not a valid query, type selection, projection, language filter, entity ordering, geo-relationship
//! or scope query (NGSI-LD clauses 4.9, 4.10, 4.15, 4.17, 4.19, 4.21 and 4.23).

use crate::{
    model::{InvalidJson, NameError, ScopeError, TemporalError},
    query::{
        QualityError,
        geoquery::DistanceError,
        q::{MemberError, PatternError},
        type_selection::SelectableTypeError,
    },
};
use langtag::InvalidLangTag;

/// What a query-language parser expected where it found something else.
///
/// The enum is `#[non_exhaustive]`: the languages grow with the specification, and so does the list of things
/// a parser can expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum Expected {
    /// A query term or a parenthesised group of query terms.
    #[display("a query term")]
    QueryTerm,
    /// An attribute name (the `AttrName` production of clause 4.9).
    #[display("an attribute name")]
    AttributeName,
    /// The name of a member of a compound value.
    #[display("a member name")]
    Member,
    /// The `]` closing a member path.
    #[display("`]`")]
    ClosingBracket,
    /// The `}` closing a linked entity path or a linked projection.
    #[display("`}}`")]
    ClosingBrace,
    /// The `"` closing a string.
    #[display("a closing `\"`")]
    ClosingQuote,
    /// A comparison operator.
    #[display("an operator")]
    Operator,
    /// A value: a number, a string, a date, a time, a boolean, a list, a range or a URI.
    #[display("a value")]
    Value,
    /// A value that can be ordered: a number, a string, a date or a time.
    #[display("a number, a string, a date or a time")]
    ComparableValue,
    /// A regular expression.
    #[display("a regular expression")]
    Pattern,
    /// A logical operator or the end of the input.
    #[display("`;`, `|` or the end of the input")]
    LogicalOperator,
    /// An entity type name.
    #[display("an entity type")]
    EntityType,
    /// An attribute name or one of the entity members `id`, `type` and `scope`.
    #[display("an attribute, `id`, `type` or `scope`")]
    ProjectionTerm,
    /// A list separator or the end of the input.
    #[display("`,`, `|` or the end of the input")]
    Separator,
    /// A language tag or the wildcard `*` (clause 4.15).
    #[display("a language tag or `*`")]
    LanguageRange,
    /// `q=` followed by a quality value (IETF RFC 9110 section 12.4.2).
    #[display("`q=` and a quality value")]
    Quality,
    /// The separator of language preferences, the weight of one, or the end of the input.
    #[display("`,`, `;` or the end of the input")]
    LanguageSeparator,
    /// An attribute path or one of the entity members `id`, `type` and `scope` (clause 4.23.3).
    #[display("an attribute path, `id`, `type` or `scope`")]
    OrderingKey,
    /// A sort direction (clause 4.23.3).
    #[display("`asc`, `desc`, `dist-asc` or `dist-desc`")]
    SortDirection,
    /// The separator of ordering terms, the direction of one, or the end of the input.
    #[display("`,`, `;` or the end of the input")]
    OrderingSeparator,
    /// A geo-relationship (clause 4.10).
    #[display("`near`, `within`, `contains`, `intersects`, `equals`, `disjoint` or `overlaps`")]
    GeoRelation,
    /// The distance modifier of a `near` relation (clause 4.10).
    #[display("`;maxDistance==` or `;minDistance==`")]
    DistanceModifier,
    /// A number greater than zero (the `PositiveNumber` of clause 4.10).
    #[display("a positive number")]
    PositiveNumber,
    /// The end of the input.
    #[display("the end of the input")]
    EndOfInput,
    /// A scope, or a parenthesised conjunction of scopes (clause 4.19).
    #[display("a scope or a parenthesised conjunction of scopes")]
    Scope,
}

/// Why a string is not a valid expression of a query language.
///
/// The expression is a query, type selection, projection, language filter, entity ordering, geo-relationship or scope
/// query.
///
/// Every variant carries the byte offset in the input where the problem starts.
// `langtag`'s error carries no `Clone` and `TemporalError` no equality, so neither can the error that chains them.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input does not follow the grammar.
    #[error("expected {expected} at byte {position}")]
    Unexpected {
        /// What the grammar allows at that position.
        expected: Expected,
        /// The byte offset in the input.
        position: usize,
    },
    /// A parenthesis is opened and never closed, or closed without being opened.
    #[error("unbalanced parenthesis at byte {position}")]
    UnbalancedParenthesis {
        /// The byte offset of the parenthesis.
        position: usize,
    },
    /// A name is not a valid attribute or entity type name.
    #[error("invalid name at byte {position}")]
    InvalidName {
        /// The byte offset where the name starts; the position in the source is relative to it.
        position: usize,
        /// Why the name is invalid.
        #[source]
        source: NameError,
    },
    /// An entity type cannot be written in a type selection.
    #[error("invalid entity type at byte {position}")]
    InvalidEntityType {
        /// The byte offset where the entity type starts; the position in the source is relative to it.
        position: usize,
        /// Why the entity type is invalid.
        #[source]
        source: SelectableTypeError,
    },
    /// A member of a member path is invalid.
    #[error("invalid member name at byte {position}")]
    InvalidMember {
        /// The byte offset where the member starts; the position in the source is relative to it.
        position: usize,
        /// Why the member is invalid.
        #[source]
        source: MemberError,
    },
    /// A regular expression cannot be delimited in a query.
    #[error("invalid regular expression at byte {position}")]
    InvalidPattern {
        /// The byte offset where the regular expression starts; the position in the source is relative to it.
        position: usize,
        /// Why the regular expression is invalid.
        #[source]
        source: PatternError,
    },
    /// A value has the shape of a `DateTime`, a Date or a Time but is not a valid one.
    #[error("invalid date or time at byte {position}")]
    InvalidTemporal {
        /// The byte offset where the value starts; the position in the source is relative to it.
        position: usize,
        /// Why the value is invalid.
        #[source]
        source: TemporalError,
    },
    /// A quoted string is not a valid JSON string (IETF RFC 8259, section 7).
    #[error("invalid string at byte {position}")]
    InvalidString {
        /// The byte offset of the opening quote; the position in the source is relative to it.
        position: usize,
        /// Why the string is invalid.
        #[source]
        source: InvalidJson,
    },
    /// A language range is neither `*` nor a language tag (IETF RFC 5646).
    #[error("invalid language tag at byte {position}")]
    InvalidLanguageTag {
        /// The byte offset where the tag starts.
        position: usize,
        /// The tag parser's error.
        #[source]
        source: InvalidLangTag<String>,
    },
    /// A weight is not a quality value (IETF RFC 9110 section 12.4.2).
    #[error("invalid quality value at byte {position}")]
    InvalidQuality {
        /// The byte offset where the quality value starts.
        position: usize,
        /// Why the value is invalid.
        #[source]
        source: QualityError,
    },
    /// The distance of a `near` relation is not a number greater than zero (clause 4.10).
    #[error("invalid distance at byte {position}")]
    InvalidDistance {
        /// The byte offset where the distance starts.
        position: usize,
        /// Why the distance is invalid.
        #[source]
        source: DistanceError,
    },
    /// A scope of a scope query is invalid (clauses 4.18 and 4.19).
    #[error("invalid scope at byte {position}")]
    InvalidScope {
        /// The byte offset where the scope starts; the position in the source is relative to it.
        position: usize,
        /// Why the scope is invalid.
        #[source]
        source: ScopeError,
    },
}

impl ParseError {
    /// Returns the byte offset in the input where the problem starts.
    #[must_use]
    pub const fn position(&self) -> usize {
        match self {
            Self::Unexpected { position, .. }
            | Self::UnbalancedParenthesis { position }
            | Self::InvalidName { position, .. }
            | Self::InvalidEntityType { position, .. }
            | Self::InvalidMember { position, .. }
            | Self::InvalidPattern { position, .. }
            | Self::InvalidTemporal { position, .. }
            | Self::InvalidString { position, .. }
            | Self::InvalidLanguageTag { position, .. }
            | Self::InvalidQuality { position, .. }
            | Self::InvalidDistance { position, .. }
            | Self::InvalidScope { position, .. } => *position,
        }
    }
}
