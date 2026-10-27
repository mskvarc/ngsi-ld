//! The record of a deleted attribute as a broker reports it (NGSI-LD clauses 4.5.0, 4.5.7, 4.5.8 and 5.8.6): an
//! instance whose payload is the NGSI-LD Null in the shape its kind needs.

use crate::model::{
    Attribute,
    AttributeDraft,
    AttributeKind,
    AttributeMeta,
    AttributeOutputMembers,
    BuildError,
    DatasetId,
    DateTime,
    EntityId,
    GeoProperty,
    GeoPropertyOutputMembers,
    GeoValue,
    InstanceId,
    JsonProperty,
    JsonPropertyOutputMembers,
    LanguageMap,
    LanguageProperty,
    LanguagePropertyOutputMembers,
    ListProperty,
    ListPropertyOutputMembers,
    ListRelationship,
    ListRelationshipOutputMembers,
    OneOrMany,
    Output,
    Property,
    PropertyOutputMembers,
    RawJson,
    Relationship,
    RelationshipOutputMembers,
    Value,
    VocabProperty,
    VocabPropertyOutputMembers,
    VocabTerm,
    null::NGSI_LD_NULL,
};
use bon::bon;
use jstrict::Value as Json;

/// The members every kind shares, as the record of a deleted attribute holds them: no `expiresAt`, signature or
/// sub-attribute, since the attribute is gone.
fn deleted_meta(dataset_id: Option<DatasetId>, observed_at: Option<DateTime>, output: AttributeOutputMembers) -> AttributeMeta<Output> {
    let mut meta = AttributeMeta::<Output>::empty();
    meta.dataset_id = dataset_id;
    meta.observed_at = observed_at;
    meta.output = output;
    meta
}

/// The NGSI-LD Null as an entity id: the `object` of a deleted Relationship and the one entry of the `objectList` of
/// a deleted `ListRelationship` (clause 4.5.0).
fn null_id() -> EntityId {
    EntityId::from(NGSI_LD_NULL)
}

/// The NGSI-LD Null as a Value: the `value` of a deleted Property and the one entry of the `valueList` of a deleted
/// `ListProperty` (clause 4.5.0).
fn null_value() -> Value {
    Value::Text(NGSI_LD_NULL.as_str().to_owned())
}

/// Generates, for the attribute kind `$kind` whose output-only members are `$members`, the record of a deleted
/// attribute of that kind (`null_record`, its payload members set to `$null`, a `const fn` when `@const` leads the
/// invocation because every `$null` is a constant) and its builder,
/// `$kind::deleted()`, whose type is `$builder` and whose typestate module is `$state`. The builder sets the members
/// every kind shares and `$previous`, the kind's previous payload; it is itself a draft of the response attribute.
macro_rules! deleted_builder {
    (
        $(@ $constness:tt)?
        $kind:ident,
        $members:ident,
        $builder:ident,
        $state:ident,
        { $($payload:ident: $null:expr),* $(,)? },
        $(#[$previous_attribute:meta])* $previous:ident: $previous_type:ty $(,)?
    ) => {
        impl $kind<Output> {
            /// The record of a deleted attribute of this kind holding `meta` and `output`: its payload is the
            /// NGSI-LD Null in the shape the kind needs (clause 4.5.0).
            $($constness)? fn null_record(meta: AttributeMeta<Output>, output: $members) -> Self {
                Self {
                    $($payload: $null,)*
                    meta,
                    output,
                }
            }
        }

        #[bon]
        impl $kind<Output> {
            /// Starts the builder of a deleted attribute of this kind as a broker reports it: its payload is the
            /// NGSI-LD Null in the shape the kind needs (clause 4.5.0), with the members every kind shares and the
            /// kind's previous payload, which a notification of a subscription with `showChanges` carries (clause
            /// 5.8.6). Building cannot fail.
            #[builder(builder_type = $builder, derive(Debug), finish_fn = build)]
            pub fn deleted(
                #[builder(into)] dataset_id: Option<DatasetId>,
                observed_at: Option<DateTime>,
                created_at: Option<DateTime>,
                modified_at: Option<DateTime>,
                deleted_at: Option<DateTime>,
                #[builder(into)] instance_id: Option<InstanceId>,
                $(#[$previous_attribute])* $previous: Option<$previous_type>,
            ) -> Self {
                let shared = AttributeOutputMembers {
                    created_at,
                    modified_at,
                    deleted_at,
                    instance_id,
                };
                Self::null_record(
                    deleted_meta(dataset_id, observed_at, shared),
                    $members {
                        $previous,
                        ..$members::default()
                    },
                )
            }
        }

        impl<S: $state::IsComplete> AttributeDraft<$kind<Output>> for $builder<S> {
            fn into_attribute(self) -> Result<$kind<Output>, BuildError> {
                Ok(self.build())
            }
        }
    };
}

deleted_builder!(
    Property,
    PropertyOutputMembers,
    PropertyDeletedBuilder,
    property_deleted_builder,
    { value: null_value(), unit_code: None, value_type: None },
    #[builder(into)] previous_value: Value,
);
deleted_builder!(
    @const
    GeoProperty,
    GeoPropertyOutputMembers,
    GeoPropertyDeletedBuilder,
    geo_property_deleted_builder,
    { value: GeoValue::Null, unit_code: None, value_type: None },
    #[builder(into)] previous_value: GeoValue,
);
deleted_builder!(
    LanguageProperty,
    LanguagePropertyOutputMembers,
    LanguagePropertyDeletedBuilder,
    language_property_deleted_builder,
    { language_map: LanguageMap::ngsi_ld_null(), value_type: None },
    previous_language_map: LanguageMap,
);
deleted_builder!(
    VocabProperty,
    VocabPropertyOutputMembers,
    VocabPropertyDeletedBuilder,
    vocab_property_deleted_builder,
    { vocab: OneOrMany::one(VocabTerm::ngsi_ld_null()), value_type: None },
    #[builder(into)] previous_vocab: OneOrMany<VocabTerm>,
);
deleted_builder!(
    ListProperty,
    ListPropertyOutputMembers,
    ListPropertyDeletedBuilder,
    list_property_deleted_builder,
    { value_list: vec![null_value()], unit_code: None, value_type: None },
    previous_value_list: Vec<Value>,
);
deleted_builder!(
    JsonProperty,
    JsonPropertyOutputMembers,
    JsonPropertyDeletedBuilder,
    json_property_deleted_builder,
    { json: RawJson::from_json(Json::from(NGSI_LD_NULL.as_str())), value_type: None },
    previous_json: RawJson,
);
deleted_builder!(
    Relationship,
    RelationshipOutputMembers,
    RelationshipDeletedBuilder,
    relationship_deleted_builder,
    { object: OneOrMany::one(null_id()), object_type: None },
    #[builder(into)] previous_object: OneOrMany<EntityId>,
);
deleted_builder!(
    ListRelationship,
    ListRelationshipOutputMembers,
    ListRelationshipDeletedBuilder,
    list_relationship_deleted_builder,
    { object_list: vec![null_id()], object_type: None },
    previous_object_list: Vec<EntityId>,
);

impl<S: attribute_deleted_builder::IsComplete> AttributeDraft<Attribute<Output>> for AttributeDeletedBuilder<S> {
    fn into_attribute(self) -> Result<Attribute<Output>, BuildError> {
        Ok(self.build())
    }
}

#[bon]
impl Attribute<Output> {
    /// Starts the builder of a deleted attribute of `kind` as a broker reports it: an instance whose payload is the
    /// NGSI-LD Null in the shape the kind needs (clause 4.5.0), as a notification reports a deleted attribute
    /// (clause 5.8.6) and a temporal evolution records a deleted Property or Relationship, with its `deletedAt`
    /// (clauses 4.5.7 and 4.5.8). A client cannot send it (clause 5.5.4) except as a
    /// [`DeletedInstance`](crate::model::evolution::DeletedInstance) of a temporal evolution; a fragment deletes
    /// through [`Deletion`](crate::model::Deletion) instead. The previous payload a notification with
    /// `showChanges` carries is set on the builder of the kind (`Property::deleted()`, `Relationship::deleted()`,
    /// ...), whose type says which `previous*` member it has.
    ///
    /// Written alone with nothing but the Null, a deleted attribute of any kind is the bare `"urn:ngsi-ld:null"`
    /// (clause 5.8.6).
    ///
    /// ```
    /// use ngsi_ld::model::{Attribute, AttributeKind, Lossless};
    ///
    /// let deleted = Attribute::deleted(AttributeKind::Relationship)
    ///     .deleted_at("2024-01-02T00:00:00Z".parse()?)
    ///     .build();
    /// assert_eq!(
    ///     deleted.render(Lossless::Normalized).to_string(),
    ///     concat!(
    ///         r#"{"type":"Relationship","object":"urn:ngsi-ld:null","#,
    ///         r#""deletedAt":"2024-01-02T00:00:00Z"}"#
    ///     )
    /// );
    /// let bare = Attribute::deleted(AttributeKind::Relationship).build();
    /// assert_eq!(
    ///     bare.render(Lossless::Normalized).to_string(),
    ///     r#""urn:ngsi-ld:null""#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// Each kind also has its own `deleted()` builder, which adds the kind's previous payload: a notification of a
    /// subscription with `showChanges` reports what the attribute held before it was deleted (clause 5.8.6), and only
    /// the kind's own `previous*` member can be set.
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::{Attribute, EntityId, Lossless, Output, Property, Relationship};
    ///
    /// let speed: Attribute<Output> = Property::deleted().previous_value(80).build().into();
    /// assert_eq!(
    ///     speed.render(Lossless::Normalized).to_string(),
    ///     concat!(
    ///         r#"{"type":"Property","value":"urn:ngsi-ld:null","#,
    ///         r#""previousValue":80}"#
    ///     )
    /// );
    /// let owner: Attribute<Output> = Relationship::deleted()
    ///     .previous_object(EntityId::from(iri!("urn:ngsi-ld:Person:Bob")))
    ///     .deleted_at("2024-01-02T00:00:00Z".parse()?)
    ///     .build()
    ///     .into();
    /// assert_eq!(
    ///     owner.render(Lossless::Normalized).to_string(),
    ///     concat!(
    ///         r#"{"type":"Relationship","object":"urn:ngsi-ld:null","#,
    ///         r#""previousObject":"urn:ngsi-ld:Person:Bob","#,
    ///         r#""deletedAt":"2024-01-02T00:00:00Z"}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[builder(finish_fn = build)]
    pub fn deleted(
        #[builder(start_fn)] kind: AttributeKind,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        created_at: Option<DateTime>,
        modified_at: Option<DateTime>,
        deleted_at: Option<DateTime>,
        #[builder(into)] instance_id: Option<InstanceId>,
    ) -> Self {
        let shared = AttributeOutputMembers {
            created_at,
            modified_at,
            deleted_at,
            instance_id,
        };
        let meta = deleted_meta(dataset_id, observed_at, shared);
        match kind {
            AttributeKind::Property => Property::null_record(meta, PropertyOutputMembers::default()).into(),
            AttributeKind::GeoProperty => GeoProperty::null_record(meta, GeoPropertyOutputMembers::default()).into(),
            AttributeKind::LanguageProperty => LanguageProperty::null_record(meta, LanguagePropertyOutputMembers::default()).into(),
            AttributeKind::VocabProperty => VocabProperty::null_record(meta, VocabPropertyOutputMembers::default()).into(),
            AttributeKind::ListProperty => ListProperty::null_record(meta, ListPropertyOutputMembers::default()).into(),
            AttributeKind::JsonProperty => JsonProperty::null_record(meta, JsonPropertyOutputMembers::default()).into(),
            AttributeKind::Relationship => Relationship::null_record(meta, RelationshipOutputMembers::default()).into(),
            AttributeKind::ListRelationship => ListRelationship::null_record(meta, ListRelationshipOutputMembers::default()).into(),
        }
    }
}
