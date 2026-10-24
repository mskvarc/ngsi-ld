//! The builder methods that add attributes, shared by every builder that holds attributes: the eight attribute
//! builders (for sub-attributes) and the entity and fragment builders.

/// Generates, on the builder type `$builder<S>` whose typestate trait is `$state`, one method per attribute kind
/// (`property`, `geo_property`, `language_property`, `vocab_property`, `list_property`, `json_property`,
/// `relationship`, `list_relationship`) plus `attribute`, each adding an attribute to the builder's
/// pending attributes field `$pending`. Naming an attribute a second time adds another instance (clause 4.5.5).
///
/// The methods are generated inside an anonymous `const` block, so the crate items they name are imported there
/// by their real paths without reaching the invoking module's scope.
macro_rules! attribute_methods {
    ($builder:ident, $state:path, $pending:ident) => {
        const _: () = {
            use $crate::model::{
                Attribute,
                AttributeDraft,
                GeoProperty,
                Input,
                JsonProperty,
                LanguageProperty,
                ListProperty,
                ListRelationship,
                Property,
                Relationship,
                VocabProperty,
            };

            impl<S: $state> $builder<S> {
                /// Adds a Property (NGSI-LD clause 4.5.2): a value, a Property, or a Property builder.
                pub fn property(mut self, name: impl Into<AttributeNameDraft>, property: impl AttributeDraft<Property<Input>>) -> Self {
                    self.$pending.push_attribute(name, property.into_attribute().map(Attribute::Property));
                    self
                }

                /// Adds a `GeoProperty` (NGSI-LD clause 4.7): a geometry, a `GeoProperty`, or a `GeoProperty` builder.
                pub fn geo_property(mut self, name: impl Into<AttributeNameDraft>, property: impl AttributeDraft<GeoProperty<Input>>) -> Self {
                    self.$pending.push_attribute(name, property.into_attribute().map(Attribute::GeoProperty));
                    self
                }

                /// Adds a `LanguageProperty` (NGSI-LD clause 4.5.18).
                pub fn language_property(mut self, name: impl Into<AttributeNameDraft>, property: impl AttributeDraft<LanguageProperty<Input>>) -> Self {
                    self.$pending
                        .push_attribute(name, property.into_attribute().map(Attribute::LanguageProperty));
                    self
                }

                /// Adds a `VocabProperty` (NGSI-LD clause 4.5.20).
                pub fn vocab_property(mut self, name: impl Into<AttributeNameDraft>, property: impl AttributeDraft<VocabProperty<Input>>) -> Self {
                    self.$pending
                        .push_attribute(name, property.into_attribute().map(Attribute::VocabProperty));
                    self
                }

                /// Adds a `ListProperty` (NGSI-LD clause 4.5.21).
                pub fn list_property(mut self, name: impl Into<AttributeNameDraft>, property: impl AttributeDraft<ListProperty<Input>>) -> Self {
                    self.$pending.push_attribute(name, property.into_attribute().map(Attribute::ListProperty));
                    self
                }

                /// Adds a `JsonProperty` (NGSI-LD clause 4.5.24).
                pub fn json_property(mut self, name: impl Into<AttributeNameDraft>, property: impl AttributeDraft<JsonProperty<Input>>) -> Self {
                    self.$pending.push_attribute(name, property.into_attribute().map(Attribute::JsonProperty));
                    self
                }

                /// Adds a Relationship (NGSI-LD clause 4.5.3): an entity id, several, a Relationship, or a
                /// Relationship builder.
                pub fn relationship(mut self, name: impl Into<AttributeNameDraft>, relationship: impl AttributeDraft<Relationship<Input>>) -> Self {
                    self.$pending
                        .push_attribute(name, relationship.into_attribute().map(Attribute::Relationship));
                    self
                }

                /// Adds a `ListRelationship` (NGSI-LD clause 4.5.22).
                pub fn list_relationship(mut self, name: impl Into<AttributeNameDraft>, relationship: impl AttributeDraft<ListRelationship<Input>>) -> Self {
                    self.$pending
                        .push_attribute(name, relationship.into_attribute().map(Attribute::ListRelationship));
                    self
                }

                /// Adds an attribute of any kind that is already built.
                pub fn attribute(mut self, name: impl Into<AttributeNameDraft>, attribute: impl Into<Attribute<Input>>) -> Self {
                    self.$pending.push_attribute(name, Ok(attribute.into()));
                    self
                }
            }
        };
    };
}
