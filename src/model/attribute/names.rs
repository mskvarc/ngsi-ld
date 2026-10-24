//! The names an attribute holds: its sub-attribute names, the `objectType` of a relationship, the `vocab` of a
//! `VocabProperty` and the `valueType` of every kind (NGSI-LD clause 5.5.7). The `json` of a `JsonProperty` is
//! never expanded, so its keys are kept.

use crate::model::{
    Attribute,
    AttributeMeta,
    Direction,
    DocumentError,
    GeoProperty,
    JsonProperty,
    LanguageProperty,
    ListProperty,
    ListRelationship,
    ListRelationshipOutputMembers,
    Proof,
    Property,
    Relationship,
    RelationshipOutputMembers,
    VocabProperty,
    VocabPropertyOutputMembers,
    direction::names::MapDirection,
    name::mapping::{MapNames, NameMapping, member},
};

impl<D: MapDirection> MapNames for Attribute<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Property(attribute) => attribute.map_names(mapping).map(Self::Property),
            Self::GeoProperty(attribute) => attribute.map_names(mapping).map(Self::GeoProperty),
            Self::LanguageProperty(attribute) => attribute.map_names(mapping).map(Self::LanguageProperty),
            Self::VocabProperty(attribute) => attribute.map_names(mapping).map(Self::VocabProperty),
            Self::ListProperty(attribute) => attribute.map_names(mapping).map(Self::ListProperty),
            Self::JsonProperty(attribute) => attribute.map_names(mapping).map(Self::JsonProperty),
            Self::Relationship(attribute) => attribute.map_names(mapping).map(Self::Relationship),
            Self::ListRelationship(attribute) => attribute.map_names(mapping).map(Self::ListRelationship),
        }
    }
}

/// The sub-attributes, and the `ngsildproof`, whose sealed entity types are names too.
impl<D: MapDirection> MapNames for AttributeMeta<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            proof: member(self.proof, "ngsildproof", mapping)?,
            attributes: self.attributes.map_names(mapping)?,
            ..self
        })
    }
}

impl<D: MapDirection> MapNames for Proof<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entity_type_sealed: member(self.entity_type_sealed, "entityTypeSealed", mapping)?,
            property: self.property.map_names(mapping)?,
            ..self
        })
    }
}

/// Implements [`MapNames`] for attribute kinds whose only names, beside their sub-attributes and the members only
/// a broker writes, are in one member: `kind_names!(Kind, .. => field: "member")` expands, for each kind, to an
/// impl that maps `self.field`, placing an error under `member`, then `meta` and `output`, and keeps the rest.
macro_rules! kind_names {
    ($($kind:ident),+ => $field:ident: $member:literal) => {$(
        impl<D: MapDirection> MapNames for $kind<D> {
            fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
                Ok(Self {
                    $field: D::map_member(self.$field, $member, mapping)?,
                    meta: self.meta.map_names(mapping)?,
                    output: D::map_kind_output(self.output, mapping)?,
                    ..self
                })
            }
        }
    )+};
}

// The `json` of a `JsonProperty` is kept as written: clause 5.5.7 expands no key inside it.
kind_names!(Property, GeoProperty, LanguageProperty, ListProperty, JsonProperty => value_type: "valueType");

kind_names!(Relationship, ListRelationship => object_type: "objectType");

/// The `vocab` values are `@vocab`-coerced terms (clause 5.5.7).
impl<D: MapDirection> MapNames for VocabProperty<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            vocab: member(self.vocab, "vocab", mapping)?,
            value_type: D::map_member(self.value_type, "valueType", mapping)?,
            meta: self.meta.map_names(mapping)?,
            output: D::map_kind_output(self.output, mapping)?,
        })
    }
}

impl MapNames for VocabPropertyOutputMembers {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            previous_vocab: member(self.previous_vocab, "previousVocab", mapping)?,
        })
    }
}

/// The linked entities inline retrieval adds are entities in the same context.
impl MapNames for RelationshipOutputMembers {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entity: member(self.entity, "entity", mapping)?,
            ..self
        })
    }
}

impl MapNames for ListRelationshipOutputMembers {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entity_list: member(self.entity_list, "entityList", mapping)?,
            ..self
        })
    }
}

carries_context!(
    /// An attribute alone carries no `@context`.
    impl<D: Direction> for Attribute<D>
);
