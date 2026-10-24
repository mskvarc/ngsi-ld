//! What is wrong with a document of the data model.

use crate::model::{AttributeKind, AttributesError, GeoJsonType, InstancesError, LanguageMapError, MemberError, NullMember, discovery::DiscoveryType};
use cefact_units::UnknownCode;

/// What is wrong with an entity, an attribute or another document of the data model (clauses 4.5 to 4.8 and 5.2).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentErrorKind {
    /// A problem every reader can meet: a member of the wrong JSON type, a missing, repeated, unknown or empty
    /// member, or an invalid NGSI-LD Value, name, identifier, URL, scope, timestamp, geometry, duration, context
    /// source alias or `@context`.
    #[error(transparent)]
    Member(#[from] MemberError),
    /// In normalized representation, an attribute has no `type` member.
    #[error("an attribute in normalized representation needs a `type` member")]
    MissingType,
    /// The `type` member names no attribute kind.
    #[error("unknown attribute type `{found}`")]
    UnknownAttributeType {
        /// The `type` as written.
        found: String,
    },
    /// A concise attribute has members of several kinds, so its kind cannot be inferred.
    #[error("the attribute kind cannot be inferred: it has both `{first}` and `{second}`")]
    AmbiguousKind {
        /// One kind-defining member.
        first: &'static str,
        /// Another kind-defining member.
        second: &'static str,
    },
    /// A language map is invalid.
    #[error("invalid language map")]
    LanguageMap {
        /// Why.
        #[source]
        source: LanguageMapError,
    },
    /// A `lang` is neither `@none` nor a BCP 47 language tag.
    #[error("invalid language tag")]
    LanguageTag {
        /// Why.
        #[source]
        source: LanguageMapError,
    },
    /// A unit code is not a UN/CEFACT common code.
    #[error("invalid unit code")]
    UnitCode {
        /// Why.
        #[source]
        source: UnknownCode,
    },
    /// The `json` of a `JsonProperty` is neither an object nor an array of objects.
    #[error("the json of a JsonProperty must be an object or an array of objects")]
    JsonShape,
    /// Instances break the dataset-id rules of clause 4.5.5.
    #[error("invalid instances")]
    Instances {
        /// Why.
        #[source]
        source: InstancesError,
    },
    /// An attribute cannot stand under its name: `location`, `observationSpace` and `operationSpace` are
    /// `GeoProperty` attributes (clause 5.2.4).
    #[error("the attribute cannot stand under its name")]
    Attributes {
        /// Why.
        #[source]
        source: AttributesError,
    },
    /// A request document holds the NGSI-LD Null, which clause 5.5.4 allows only in fragments.
    #[error("a request cannot hold the NGSI-LD Null in {member}")]
    NgsiLdNull {
        /// Where the Null is.
        member: NullMember,
    },
    /// An `ngsildproof` is an attribute of another kind than `Property`.
    #[error("an ngsildproof must be a Property, found a {found}")]
    ProofNotAProperty {
        /// The kind it was read as.
        found: AttributeKind,
    },
    /// An instance of the temporal evolution of a scope is not a Property (clause 4.5.6).
    #[error("the evolution of a scope is a Property, found type `{found}`")]
    ScopeNotAProperty {
        /// The `type` as written.
        found: String,
    },
    /// An element of a simplified or aggregated temporal attribute is not an array of the length its
    /// representation requires: a payload and a time (clause 4.5.9), or a value, a start and an end
    /// (clause 4.5.19).
    #[error("expected an array of {expected} elements, found {found}")]
    TemporalEntry {
        /// The length the representation requires.
        expected: usize,
        /// The length found.
        found: usize,
    },
    /// An aggregated temporal attribute is of a kind clause 4.5.19 does not aggregate.
    #[error("a {found} has no aggregated temporal representation")]
    AggregatedKind {
        /// The kind found.
        found: AttributeKind,
    },
    /// The value of an `ngsildproof` is not a JSON object.
    #[error("the value of an ngsildproof must be a JSON object")]
    ProofNotAnObject,
    /// A `GeoJSON` object has another `type` than the one expected.
    #[error("expected a GeoJSON {expected}, found type `{found}`")]
    GeoJsonType {
        /// The `GeoJSON` type expected.
        expected: GeoJsonType,
        /// The `type` as written.
        found: String,
    },
    /// A discovery document has another `type` than the one expected (clauses 4.5.10 to 4.5.15).
    #[error("expected a {expected} document, found type `{found}`")]
    DiscoveryType {
        /// The type expected.
        expected: DiscoveryType,
        /// The `type` as written.
        found: String,
    },
}
