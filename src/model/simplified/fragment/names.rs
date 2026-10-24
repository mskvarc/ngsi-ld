//! The names a fragment in simplified representation holds: the types it adds, the attributes it changes and the
//! `vocab` of the values it sets (NGSI-LD clause 5.5.7).

use crate::model::{
    DocumentError,
    SimplifiedAttributeChange,
    SimplifiedChange,
    SimplifiedFragment,
    name::mapping::{MapNames, NameMapping, member},
};

impl MapNames for SimplifiedFragment {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entity_type: member(self.entity_type, "type", mapping)?,
            attributes: self.attributes.map_names(mapping)?,
            ..self
        })
    }
}

impl MapNames for SimplifiedAttributeChange {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Default(change) => change.map_names(mapping).map(Self::Default),
            Self::Dataset(instances) => instances
                .into_iter()
                .map(|(dataset_id, change)| {
                    let change = change
                        .map_names(mapping)
                        .map_err(|error| error.within(&dataset_id.to_string()).within("dataset"))?;
                    Ok((dataset_id, change))
                })
                .collect::<Result<_, _>>()
                .map(Self::Dataset),
        }
    }
}

impl MapNames for SimplifiedChange {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Set(value) => value.map_names(mapping).map(Self::Set),
            Self::Delete => Ok(Self::Delete),
        }
    }
}

carries_context!(
    /// A fragment carries no `@context`: its request delivers the endpoint's (clause 6.3.5).
    impl for SimplifiedFragment
);
