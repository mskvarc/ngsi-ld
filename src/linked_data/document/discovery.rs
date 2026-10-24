//! The discovery documents as linked documents (NGSI-LD clauses 5.2.24 to 5.2.31): every list they hold is an
//! array in their data types.

use crate::{
    linked_data::document::{
        sealed::Sealed,
        shape::{ARRAY, Shape},
    },
    model::{
        DocumentError,
        discovery::{AttributeDetails, AttributeInfo, AttributeList, EntityTypeDetails, EntityTypeInfo, EntityTypeList},
    },
};
use jstrict::Value as Json;

/// Implements [`Sealed`] for a discovery document, read with its `from_json` and written with its `render`.
macro_rules! discovery_document {
    ($($document:ty => $shape:expr),+ $(,)?) => {
        $(
            impl Sealed for $document {
                type ReadError = DocumentError;

                const SHAPE: Shape = $shape;

                fn write(&self) -> Json {
                    self.render().into_json()
                }

                fn read(json: Json) -> Result<Self, Self::ReadError> {
                    Self::from_json(json)
                }
            }
        )+
    };
}

discovery_document!(
    EntityTypeList => Shape::Object(&[("typeList", ARRAY)]),
    EntityTypeDetails => Shape::Object(&[("attributeNames", ARRAY)]),
    EntityTypeInfo => Shape::Object(&[("attributeDetails", Shape::Array(&Shape::Object(&[("attributeTypes", ARRAY)])))]),
    AttributeList => Shape::Object(&[("attributeList", ARRAY)]),
    AttributeDetails => Shape::Object(&[("typeNames", ARRAY)]),
    AttributeInfo => Shape::Object(&[("attributeTypes", ARRAY), ("typeNames", ARRAY)]),
);
