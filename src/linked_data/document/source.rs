//! The documents about a context source and a stored `@context` as linked documents (NGSI-LD clauses 5.2.40 and
//! 5.13.3): neither holds an array.

use crate::{
    linked_data::document::{sealed::Sealed, shape::Shape},
    model::{ContextMetadata, ContextSourceIdentity, DocumentError},
};
use jstrict::Value as Json;

impl Sealed for ContextSourceIdentity {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

/// The metadata of a stored context carries no `@context` of its own.
impl Sealed for ContextMetadata {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}
