//! The data model.
//!
//! It holds names, identifiers, values, attributes, entities and their representations (NGSI-LD clauses 4.5 to 4.8 and
//! 5.2).
//!
//! Nothing in this module knows about HTTP or any other transport; it is usable on its own by a broker, a
//! notification receiver or a data pipeline.
//!
//! A document a broker returns is read in the representation it was asked for, and written in any other:
//!
//! ```
//! use ngsi_ld::model::{Entity, Lossless, Output, Representation};
//!
//! let concise = r#"{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": 80}"#;
//! let vehicle = Entity::<Output>::parse(concise, Lossless::Concise)?;
//! assert_eq!(
//!     vehicle.render(Representation::Normalized).to_string(),
//!     concat!(
//!         r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","#,
//!         r#""speed":{"type":"Property","value":80}}"#
//!     )
//! );
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Entities
//!
//! [`Entity`], [`EntityDirection`], [`Completeness`], [`Complete`], [`Projected`], [`EntityFragment`], [`ScopeChange`],
//! [`SimplifiedEntity`], [`SimplifiedValue`], [`SimplifiedFragment`], [`SimplifiedChange`],
//! [`SimplifiedAttributeChange`], [`Feature`], [`FeatureCollection`], [`GeoJsonType`], [`IdDraft`], [`TypeDraft`],
//! [`ScopeDraft`].
//!
//! # Attributes
//!
//! Each kind's `output_builder` ([`Property::output_builder`] and its counterparts) builds the attributes a broker or a
//! context source returns (clause 5.2.2): a client never builds them for a request; a mock broker or a context source
//! under test does, to answer the way the specification shapes an answer.
//!
//! A lone attribute sent as a JSON-LD document, the body of Replace Attribute or Partial Attribute Update with its
//! `@context`, is an [`AttributeDocument`].
//!
//! [`Attribute`], [`AttributeKind`], [`AttributeKindIter`], [`AttributeDraft`], [`AttributeSlot`], [`AttributeDocument`], [`Property`],
//! [`PropertyOutputMembers`], [`Relationship`], [`RelationshipOutputMembers`], [`GeoProperty`],
//! [`GeoPropertyOutputMembers`], [`GeoValue`], [`LanguageProperty`], [`LanguagePropertyOutputMembers`],
//! [`VocabProperty`], [`VocabPropertyOutputMembers`], [`JsonProperty`], [`JsonPropertyOutputMembers`],
//! [`ListProperty`], [`ListPropertyOutputMembers`], [`ListRelationship`], [`ListRelationshipOutputMembers`],
//! [`AttributeMeta`], [`Proof`], [`Attributes`], [`AttributesError`], [`Instances`], [`InstanceKey`],
//! [`InstancesError`], [`AttributeFragment`], [`AttributeMembers`], [`PatchMember`], [`PatchMemberIter`],
//! [`MemberOfOtherKind`], [`NullMember`].
//!
//! # Directions
//!
//! [`Direction`], [`Plain`], [`Input`], [`Patch`], [`Output`], [`Change`], [`Deletion`], [`InstanceDeletion`],
//! [`AttributeOutputMembers`], [`EntityOutputMembers`], [`MemberChange`].
//!
//! # Values
//!
//! [`Value`], [`ValueError`], [`Number`], [`NumberError`], [`NotFinite`], [`PositiveNumber`], [`NotPositive`],
//! [`TypedValue`], [`TypedValueError`], [`Datatype`], [`CoreDatatype`], [`ValuePatch`], [`ValueChange`],
//! [`LanguageMap`], [`LanguageKey`], [`LanguageText`], [`LanguageTagInput`], [`LanguageTagError`],
//! [`LanguageMapError`], [`DateTime`], [`Date`], [`Time`], [`TemporalError`], [`TemporalToken`], [`Duration`],
//! [`DurationError`], [`KeyValuePair`], [`OneOrMany`], [`OneOrManyDraft`], [`EmptyList`], [`RawJson`], [`InvalidJson`].
//!
//! # Names and identifiers
//!
//! [`Name`], [`Term`], [`AttributeName`], [`EntityTypeName`], [`VocabTerm`], [`NameError`], [`AttributeNameDraft`],
//! [`ReservedMember`], [`ReservedMemberIter`], [`EntityId`], [`DatasetId`], [`InstanceId`], [`Uri`], [`UriError`],
//! [`UrlDraft`], [`Scope`], [`ScopeLevel`], [`ScopeError`], [`Tenant`], [`TenantError`], [`Token`], [`TokenError`],
//! [`ContextSourceIdentity`], [`ContextSourceId`], [`ContextSourceAlias`], [`ContextSourceAliasError`],
//! [`ConformanceVersion`], [`ConformanceVersionError`].
//!
//! # Context
//!
//! The `@context`s a broker stores and serves (clause 5.13) are described by [`ContextId`], [`StoredContextKind`],
//! [`ContextMetadata`] and [`LocalContext`]. The crate carries a context as given and never resolves it.
//!
//! [`Context`], [`ContextEntry`], [`InlineContext`], [`InlineContextError`], [`ContextError`], [`CORE_CONTEXT`],
//! [`ContextId`], [`ContextMetadata`], [`LocalContext`], [`StoredContextKind`], [`StoredContextKindError`],
//! [`OtherContextKind`], [`ExtraMemberName`], [`ExtraMemberNameError`], [`NgsiLdMediaType`], [`NgsiLdMediaTypeError`],
//! [`APPLICATION_JSON`], [`APPLICATION_LD_JSON`], [`APPLICATION_GEO_JSON`].
//!
//! # Representations
//!
//! [`Representation`], [`Lossless`], [`SimplifiedSpelling`].
//!
//! # Errors
//!
//! [`ProblemDetails`], [`ErrorType`], [`BuildError`], [`BuildProblem`], [`ProblemKind`], [`MemberProblem`],
//! [`DocumentError`], [`DocumentErrorKind`], [`MemberError`], [`JsonType`], [`MemberPath`], [`PathSegment`].
//!
//! # Nested namespaces
//!
//! [`geometry`], [`evolution`], [`discovery`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`]. [`PendingChanges`] and [`PendingSimplifiedChanges`] are the changes
//! a fragment builder collects before it finishes.

pub use crate::model::{
    attribute::{
        Attribute,
        AttributeDraft,
        AttributeKind,
        AttributeKindIter,
        AttributeSlot,
        attributes::{Attributes, AttributesError},
        document::AttributeDocument,
        fragment::{AttributeFragment, AttributeMembers},
        geo_property::{GeoProperty, GeoPropertyOutputMembers, GeoValue},
        instances::{InstanceKey, Instances, InstancesError},
        json_property::{JsonProperty, JsonPropertyOutputMembers},
        language_property::{LanguageProperty, LanguagePropertyOutputMembers},
        list_property::{ListProperty, ListPropertyOutputMembers},
        list_relationship::{ListRelationship, ListRelationshipOutputMembers},
        meta::AttributeMeta,
        patch::{MemberOfOtherKind, PatchMember, PatchMemberIter},
        proof::Proof,
        property::{Property, PropertyOutputMembers},
        relationship::{Relationship, RelationshipOutputMembers},
        vocab_property::{VocabProperty, VocabPropertyOutputMembers},
    },
    build_error::BuildError,
    build_problem::{BuildProblem, ProblemKind},
    conformance::{ConformanceVersion, ConformanceVersionError},
    context::{CORE_CONTEXT, Context, ContextEntry, ContextError, InlineContext, InlineContextError},
    direction::{AttributeOutputMembers, Change, Deletion, Direction, EntityOutputMembers, Input, InstanceDeletion, Output, Patch, Plain},
    document_error::{
        DocumentError,
        kind::DocumentErrorKind,
        member_error::{JsonType, MemberError},
        path::{MemberPath, PathSegment},
    },
    duration::{Duration, DurationError},
    entity::{
        Entity,
        EntityDirection,
        completeness::{Complete, Completeness, Projected},
        draft::{IdDraft, ScopeDraft, TypeDraft},
    },
    feature::{Feature, FeatureCollection, GeoJsonType},
    fragment::{EntityFragment, PendingChanges, ScopeChange},
    identifier::{DatasetId, EntityId, InstanceId},
    key_value_pair::KeyValuePair,
    language::{LanguageKey, LanguageMap, LanguageMapError, LanguageTagError, LanguageTagInput, LanguageText},
    media_type::{APPLICATION_GEO_JSON, APPLICATION_JSON, APPLICATION_LD_JSON, NgsiLdMediaType, NgsiLdMediaTypeError},
    member_change::MemberChange,
    member_problem::MemberProblem,
    name::{
        AttributeName,
        EntityTypeName,
        Name,
        NameError,
        Term,
        VocabTerm,
        draft::AttributeNameDraft,
        reserved::{ReservedMember, ReservedMemberIter},
    },
    null::NullMember,
    one_or_many::{EmptyList, OneOrMany, OneOrManyDraft},
    problem::{ErrorType, ProblemDetails},
    raw_json::{InvalidJson, RawJson},
    representation::{Lossless, Representation, simplified_spelling::SimplifiedSpelling},
    scope::{Scope, ScopeError, ScopeLevel},
    simplified::{
        SimplifiedEntity,
        SimplifiedValue,
        fragment::{PendingSimplifiedChanges, SimplifiedAttributeChange, SimplifiedChange, SimplifiedFragment},
    },
    source_identity::{ContextSourceAlias, ContextSourceAliasError, ContextSourceId, ContextSourceIdentity},
    stored_context::{
        ContextId,
        ContextMetadata,
        LocalContext,
        StoredContextKind,
        StoredContextKindError,
        extra_member::{ExtraMemberName, ExtraMemberNameError},
        other_kind::OtherContextKind,
    },
    temporal::{Date, DateTime, TemporalError, TemporalToken, Time},
    tenant::{Tenant, TenantError},
    token::{Token, TokenError},
    uri::{Uri, UriError, draft::UrlDraft},
    value::{
        Value,
        ValueError,
        number::{NotFinite, Number, NumberError},
        patch::{ValueChange, ValuePatch},
        positive_number::{NotPositive, PositiveNumber},
        typed_value::{CoreDatatype, Datatype, TypedValue, TypedValueError},
    },
};

#[macro_use]
mod attribute_methods;
pub mod builders;
#[macro_use]
pub(crate) mod draft;
#[macro_use]
pub(crate) mod name;
mod attribute;
mod build_error;
mod build_problem;
mod conformance;
pub(crate) mod context;
mod direction;
pub mod discovery;
mod document_error;
mod duration;
mod entity;
pub mod evolution;
pub(crate) mod feature;
mod fragment;
pub mod geometry;
mod identifier;
mod key_value_pair;
pub(crate) mod keyword;
pub(crate) mod keyword_alias;
pub(crate) mod language;
mod media_type;
#[macro_use]
pub(crate) mod member_change;
mod member_problem;
pub(crate) mod nesting;
pub(crate) mod null;
mod one_or_many;
mod problem;
mod raw_json;
pub(crate) mod reader;
pub(crate) mod representation;
mod scope;
mod simplified;
mod source_identity;
mod stored_context;
mod temporal;
mod tenant;
pub(crate) mod token;
mod uri;
pub(crate) mod value;
pub(crate) mod written;
pub(crate) mod written_url;
