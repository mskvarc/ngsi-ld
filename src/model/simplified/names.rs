//! The names an entity in simplified representation holds: its types, its attribute names and the `vocab` of a
//! `VocabProperty` (NGSI-LD clause 5.5.7). A value is data and is kept, as is a `json`.

use crate::model::{
    Completeness,
    DocumentError,
    SimplifiedEntity,
    SimplifiedValue,
    entity::completeness::MapCompleteness,
    name::mapping::{MapNames, NameMapping, member},
};

entity_names!(impl<C: MapCompleteness> for SimplifiedEntity<C> => C);

impl MapNames for SimplifiedValue {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Vocab(vocab) => member(vocab, "vocab", mapping).map(Self::Vocab),
            Self::Dataset(instances) => instances
                .into_iter()
                .map(|(dataset_id, value)| {
                    let value = value
                        .map_names(mapping)
                        .map_err(|error| error.within(&dataset_id.to_string()).within("dataset"))?;
                    Ok((dataset_id, value))
                })
                .collect::<Result<_, _>>()
                .map(Self::Dataset),
            value @ (Self::Value(_) | Self::Geometry(_) | Self::LanguageMap(_) | Self::Json(_)) => Ok(value),
        }
    }
}

carries_context!(impl<C: Completeness> for SimplifiedEntity<C> => context);
