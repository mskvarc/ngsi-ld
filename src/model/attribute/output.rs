//! Building the attributes a broker or a context source returns: a request attribute plus the members only a
//! response carries (NGSI-LD clause 5.2.2 and the "output only members" Tables 5.2.5-2, 5.2.6-2, 5.2.7-2, 5.2.32-2,
//! 5.2.35-2, 5.2.36-2, 5.2.37-2 and 5.2.38-2). The record of a deleted attribute is built in the `deleted` module.

use crate::model::{
    Attribute,
    AttributeDraft,
    AttributeMeta,
    AttributeOutputMembers,
    BuildError,
    DateTime,
    Entity,
    EntityId,
    GeoProperty,
    GeoPropertyOutputMembers,
    GeoValue,
    Input,
    InstanceId,
    JsonProperty,
    JsonPropertyOutputMembers,
    LanguageKey,
    LanguageMap,
    LanguageProperty,
    LanguagePropertyOutputMembers,
    ListProperty,
    ListPropertyOutputMembers,
    ListRelationship,
    ListRelationshipOutputMembers,
    OneOrMany,
    Output,
    Projected,
    Property,
    PropertyOutputMembers,
    RawJson,
    Relationship,
    RelationshipOutputMembers,
    Value,
    VocabProperty,
    VocabPropertyOutputMembers,
    VocabTerm,
};
use bon::bon;

/// Generates, for the attribute kind `$kind` whose output-only members are `$members`, the builder of a response
/// attribute of that kind: `$kind::output_builder(draft)`, whose type is `$builder` and whose typestate module is
/// `$state`. The draft is anything the kind's request builder accepts; the shared output-only members (`createdAt`,
/// `modifiedAt`, `deletedAt`, `instanceId`) and the kind's own (`$field`) have setters. The builder, unfinished, is
/// itself a draft of the response attribute.
macro_rules! output_builder {
    ($kind:ident, $members:ident, $builder:ident, $state:ident, { $($(#[$attribute:meta])* $field:ident: $field_type:ty),* $(,)? }) => {
        #[bon]
        impl $kind<Output> {
            /// Starts the builder of a response attribute of this kind from anything its request builder accepts:
            /// the attribute's content as a client would send it, then the output-only members a broker adds
            /// (clause 5.2.2 and the kind's "output only members" table). Sub-attributes carry none.
            ///
            /// # Errors
            ///
            /// `build()` returns [`BuildError`] with every problem the draft reports.
            #[builder(builder_type = $builder, derive(Debug), finish_fn(name = build, doc {
                /// Builds the response attribute.
                ///
                /// # Errors
                ///
                /// Returns [`BuildError`] with every problem the draft reports.
            }))]
            pub fn output_builder<A: AttributeDraft<$kind<Input>>>(
                #[builder(start_fn)] attribute: A,
                created_at: Option<DateTime>,
                modified_at: Option<DateTime>,
                deleted_at: Option<DateTime>,
                #[builder(into)] instance_id: Option<InstanceId>,
                $($(#[$attribute])* $field: Option<$field_type>,)*
            ) -> Result<Self, BuildError> {
                let mut attribute = attribute.into_attribute()?.map_meta(AttributeMeta::into_output);
                attribute.meta.output = AttributeOutputMembers {
                    created_at,
                    modified_at,
                    deleted_at,
                    instance_id,
                };
                attribute.output = $members {
                    $($field,)*
                    ..$members::default()
                };
                Ok(attribute)
            }
        }

        impl<A: AttributeDraft<$kind<Input>>, S: $state::IsComplete> AttributeDraft<$kind<Output>> for $builder<A, S> {
            fn into_attribute(self) -> Result<$kind<Output>, BuildError> {
                self.build()
            }
        }

        impl AttributeDraft<$kind<Output>> for $kind<Output> {
            fn into_attribute(self) -> Result<$kind<Output>, BuildError> {
                Ok(self)
            }
        }
    };
}

output_builder!(Property, PropertyOutputMembers, PropertyOutputBuilder, property_output_builder, {
    #[builder(into)] previous_value: Value,
    lang: LanguageKey,
});
output_builder!(GeoProperty, GeoPropertyOutputMembers, GeoPropertyOutputBuilder, geo_property_output_builder, {
    #[builder(into)] previous_value: GeoValue,
});
output_builder!(LanguageProperty, LanguagePropertyOutputMembers, LanguagePropertyOutputBuilder, language_property_output_builder, {
    previous_language_map: LanguageMap,
});
output_builder!(VocabProperty, VocabPropertyOutputMembers, VocabPropertyOutputBuilder, vocab_property_output_builder, {
    #[builder(into)] previous_vocab: OneOrMany<VocabTerm>,
});
output_builder!(ListProperty, ListPropertyOutputMembers, ListPropertyOutputBuilder, list_property_output_builder, {
    previous_value_list: Vec<Value>,
});
output_builder!(JsonProperty, JsonPropertyOutputMembers, JsonPropertyOutputBuilder, json_property_output_builder, {
    previous_json: RawJson,
});
// The linked entities of inline retrieval (`entity` and `entityList`, clause 4.5.23.2) are projected entities, since a
// projection of the response applies to them too; a complete entity converts into one.
output_builder!(Relationship, RelationshipOutputMembers, RelationshipOutputBuilder, relationship_output_builder, {
    #[builder(into)] previous_object: OneOrMany<EntityId>,
    #[builder(into)] entity: OneOrMany<Entity<Output, Projected>>,
});
output_builder!(ListRelationship, ListRelationshipOutputMembers, ListRelationshipOutputBuilder, list_relationship_output_builder, {
    previous_object_list: Vec<EntityId>,
    #[builder(with = |entities: impl IntoIterator<Item = impl Into<Entity<Output, Projected>>>| entities.into_iter().map(Into::into).collect())]
    entity_list: Vec<Entity<Output, Projected>>,
});

impl AttributeDraft<Attribute<Output>> for Attribute<Output> {
    fn into_attribute(self) -> Result<Attribute<Output>, BuildError> {
        Ok(self)
    }
}
