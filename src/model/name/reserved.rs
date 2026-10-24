//! Members the core `@context` reserves, which can never name an attribute (NGSI-LD clause 4.5.1).

/// A member of an entity or attribute that cannot be used as an attribute name.
///
/// The core `@context` defines it, which reserves the name.
///
/// Clause 4.5.1 forbids the terms the core `@context` defines as non-reified Properties (`datasetId`,
/// `instanceId`, ...) as attribute names; the structural members of an entity (`id`, `type`, `scope`,
/// `@context`) and of an attribute (`value`, `object`, `languageMap`, ...) are excluded for the same reason: a
/// member with that name would be read as structure, not as an attribute.
///
/// The enum is `#[non_exhaustive]`: each version of the core `@context` may reserve more terms, and a caller who
/// matches on the member that refused a name keeps compiling when it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[non_exhaustive]
pub enum ReservedMember {
    /// `id` (clause 4.5.1).
    #[strum(serialize = "id")]
    Id,
    /// `@id`, the JSON-LD alias of `id` (clause 4.4).
    #[strum(serialize = "@id")]
    AtId,
    /// `type` (clause 4.5.1).
    #[strum(serialize = "type")]
    Type,
    /// `@type`, the JSON-LD alias of `type` (clause 4.4).
    #[strum(serialize = "@type")]
    AtType,
    /// `scope` (clause 4.18).
    #[strum(serialize = "scope")]
    Scope,
    /// `@context` (clause 4.4).
    #[strum(serialize = "@context")]
    Context,
    /// `createdAt` (clause 4.8).
    #[strum(serialize = "createdAt")]
    CreatedAt,
    /// `modifiedAt` (clause 4.8).
    #[strum(serialize = "modifiedAt")]
    ModifiedAt,
    /// `deletedAt` (clause 4.8).
    #[strum(serialize = "deletedAt")]
    DeletedAt,
    /// `observedAt` (clause 4.8).
    #[strum(serialize = "observedAt")]
    ObservedAt,
    /// `expiresAt` (clause 4.22).
    #[strum(serialize = "expiresAt")]
    ExpiresAt,
    /// `datasetId` (clause 4.5.5).
    #[strum(serialize = "datasetId")]
    DatasetId,
    /// `instanceId` (clause 4.5.7).
    #[strum(serialize = "instanceId")]
    InstanceId,
    /// `unitCode` (clause 4.5.2.2).
    #[strum(serialize = "unitCode")]
    UnitCode,
    /// `valueType` (clause 4.5.2.2).
    #[strum(serialize = "valueType")]
    ValueType,
    /// `value` (clause 4.5.2.2).
    #[strum(serialize = "value")]
    Value,
    /// `object` (clause 4.5.3.2).
    #[strum(serialize = "object")]
    Object,
    /// `objectType` (clause 4.5.3.2).
    #[strum(serialize = "objectType")]
    ObjectType,
    /// `objectList` (clause 4.5.22.2).
    #[strum(serialize = "objectList")]
    ObjectList,
    /// `languageMap` (clause 4.5.18.2).
    #[strum(serialize = "languageMap")]
    LanguageMap,
    /// `vocab` (clause 4.5.20.2).
    #[strum(serialize = "vocab")]
    Vocab,
    /// `valueList` (clause 4.5.21.2).
    #[strum(serialize = "valueList")]
    ValueList,
    /// `json` (clause 4.5.24.2).
    #[strum(serialize = "json")]
    Json,
    /// `entity` (clause 4.5.23.2).
    #[strum(serialize = "entity")]
    Entity,
    /// `entityList` (clause 4.5.23.2).
    #[strum(serialize = "entityList")]
    EntityList,
    /// `previousValue` (clause 4.5.2.2).
    #[strum(serialize = "previousValue")]
    PreviousValue,
    /// `previousObject` (clause 4.5.3.2).
    #[strum(serialize = "previousObject")]
    PreviousObject,
    /// `previousObjectList` (clause 4.5.22.2).
    #[strum(serialize = "previousObjectList")]
    PreviousObjectList,
    /// `previousLanguageMap` (clause 4.5.18.2).
    #[strum(serialize = "previousLanguageMap")]
    PreviousLanguageMap,
    /// `previousVocab` (clause 4.5.20.2).
    #[strum(serialize = "previousVocab")]
    PreviousVocab,
    /// `previousValueList` (clause 4.5.21.2).
    #[strum(serialize = "previousValueList")]
    PreviousValueList,
    /// `previousJson` (clause 4.5.24.2).
    #[strum(serialize = "previousJson")]
    PreviousJson,
    /// `lang`, the non-reified sub-property the language filter adds (clause 4.15).
    #[strum(serialize = "lang")]
    Lang,
    /// `ngsildproof`, the digital signature of an attribute, whose members the core `@context` defines
    /// (clause 4.5.2.2).
    #[strum(serialize = "ngsildproof")]
    NgsildProof,
    /// `entityIdSealed`, a non-reified sub-property of `ngsildproof` (clause 4.5.2.2).
    #[strum(serialize = "entityIdSealed")]
    EntityIdSealed,
    /// `entityTypeSealed`, a non-reified sub-property of `ngsildproof` (clause 4.5.2.2).
    #[strum(serialize = "entityTypeSealed")]
    EntityTypeSealed,
}

impl ReservedMember {
    /// Returns the reserved member `name` spells, if any.
    #[must_use]
    pub fn find(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    /// Returns the member as it is spelled in a document.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}
