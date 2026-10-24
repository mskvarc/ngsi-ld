//! The names a temporal evolution holds, in every temporal representation: the entity types, the attribute names
//! and the `vocab` of a `VocabProperty` (NGSI-LD clause 5.5.7); and the names the `attrs` of a temporal query takes.

use crate::model::{
    Completeness,
    DocumentError,
    EntityDirection,
    direction::names::MapDirection,
    entity::completeness::MapCompleteness,
    evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalAttributeName, TemporalEntity, TemporalValue, TemporalValues},
    name::mapping::{MapNames, NameMapping, member},
};

/// An attribute name is mapped; `scope` is a term of the core `@context`, which no context redefines (clause 4.5.1).
impl MapNames for TemporalAttributeName {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Attribute(name) => name.map_names(mapping).map(Self::Attribute),
            Self::Scope => Ok(Self::Scope),
        }
    }
}

entity_names!(impl<D: EntityDirection + MapDirection, C: MapCompleteness> for TemporalEntity<D, C> => C);

entity_names!(impl<C: MapCompleteness> for AggregatedTemporalEntity<C> => C);

entity_names!(impl<C: MapCompleteness> for SimplifiedTemporalEntity<C> => C);

/// Only the `vocabs` of a `VocabProperty` hold names; every other kind's values are data.
impl MapNames for TemporalValues {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::VocabProperty(values) => member(values, "vocabs", mapping).map(Self::VocabProperty),
            values @ (Self::Property(_)
            | Self::GeoProperty(_)
            | Self::LanguageProperty(_)
            | Self::ListProperty(_)
            | Self::JsonProperty(_)
            | Self::Relationship(_)
            | Self::ListRelationship(_)) => Ok(values),
        }
    }
}

impl<T: MapNames> MapNames for TemporalValue<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            value: self.value.map_names(mapping)?,
            ..self
        })
    }
}

carries_context!(impl<D: EntityDirection, C: Completeness> for TemporalEntity<D, C> => context);

carries_context!(impl<C: Completeness> for AggregatedTemporalEntity<C> => context);

carries_context!(impl<C: Completeness> for SimplifiedTemporalEntity<C> => context);
