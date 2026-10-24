//! The names a `GeoJSON` Feature or `FeatureCollection` holds: those of its entities (NGSI-LD clause 5.5.7).

use crate::model::{
    DocumentError,
    Feature,
    FeatureCollection,
    name::mapping::{MapNames, NameMapping, member},
};

impl<E: MapNames> MapNames for Feature<E> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entity: self.entity.map_names(mapping)?,
            ..self
        })
    }
}

impl<E: MapNames> MapNames for FeatureCollection<E> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            features: member(self.features, "features", mapping)?,
            ..self
        })
    }
}

carries_context!(impl<E> for Feature<E> => context);

carries_context!(impl<E> for FeatureCollection<E> => context);
