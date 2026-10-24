//! The names the discovery documents hold: the entity types and attributes they list (NGSI-LD clauses 5.2.24 to
//! 5.2.31). Their `id` is the Fully Qualified Name of what they describe, an IRI that no `@context` changes, so it
//! is kept.

use crate::model::{
    DocumentError,
    discovery::{AttributeDetails, AttributeInfo, AttributeList, EntityTypeAttribute, EntityTypeDetails, EntityTypeInfo, EntityTypeList},
    name::mapping::{MapNames, NameMapping, member},
};

impl MapNames for EntityTypeList {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            type_list: member(self.type_list, "typeList", mapping)?,
            ..self
        })
    }
}

impl MapNames for EntityTypeDetails {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attribute_names: member(self.attribute_names, "attributeNames", mapping)?,
            type_name: member(self.type_name, "typeName", mapping)?,
            ..self
        })
    }
}

impl MapNames for EntityTypeInfo {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attribute_details: member(self.attribute_details, "attributeDetails", mapping)?,
            type_name: member(self.type_name, "typeName", mapping)?,
            ..self
        })
    }
}

impl MapNames for EntityTypeAttribute {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attribute_name: member(self.attribute_name, "attributeName", mapping)?,
            ..self
        })
    }
}

impl MapNames for AttributeList {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attribute_list: member(self.attribute_list, "attributeList", mapping)?,
            ..self
        })
    }
}

impl MapNames for AttributeDetails {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attribute_name: member(self.attribute_name, "attributeName", mapping)?,
            type_names: member(self.type_names, "typeNames", mapping)?,
            ..self
        })
    }
}

impl MapNames for AttributeInfo {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attribute_name: member(self.attribute_name, "attributeName", mapping)?,
            type_names: member(self.type_names, "typeNames", mapping)?,
            ..self
        })
    }
}

carries_context!(impl for EntityTypeList => context);
carries_context!(impl for EntityTypeDetails => context);
carries_context!(impl for EntityTypeInfo => context);
carries_context!(impl for AttributeList => context);
carries_context!(impl for AttributeDetails => context);
carries_context!(impl for AttributeInfo => context);
