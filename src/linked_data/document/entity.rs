//! The entity documents as linked documents: entities, attributes, simplified entities, `GeoJSON` features and
//! temporal evolutions (NGSI-LD clauses 4.5 and 5.2.4).

use crate::{
    linked_data::document::{printed, sealed::Sealed, shape::Shape},
    model::{
        Attribute,
        Complete,
        Completeness,
        DocumentError,
        Entity,
        Feature,
        FeatureCollection,
        Input,
        Lossless,
        Output,
        Projected,
        SimplifiedEntity,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalEntity},
        representation::{entity_object, read::read_entity},
    },
    seal::Seal,
};
use jstrict::Value as Json;

impl Sealed for Entity<Input> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        Json::Object(entity_object(self, Lossless::Normalized))
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        read_entity(json, Lossless::Normalized)
    }
}

impl<C: Completeness> Sealed for Entity<Output, C> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        Json::Object(entity_object(self, Lossless::Normalized))
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        read_entity(json, Lossless::Normalized)
    }
}

/// An attribute alone, as Replace Attribute takes it (clause 6.7.3.3); it carries no `@context`.
impl Sealed for Attribute<Input> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse_input(&printed(&json), Lossless::Normalized)
    }
}

/// One attribute instance a broker returned; it carries no `@context`.
impl Sealed for Attribute<Output> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

impl Sealed for SimplifiedEntity<Complete> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}

impl Sealed for SimplifiedEntity<Projected> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse_projected(&printed(&json))
    }
}

impl Sealed for Feature<Entity<Output>> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

impl Sealed for Feature<SimplifiedEntity> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}

/// `features` is a `@set` in the core context, so compaction keeps its array.
impl Sealed for FeatureCollection<Entity<Output>> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

impl Sealed for FeatureCollection<SimplifiedEntity> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}

/// An attribute's evolution with one instance compacts to that instance, which the reader takes as the evolution
/// clause 4.5.7 allows.
impl Sealed for TemporalEntity<Input> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

impl<C: Completeness> Sealed for TemporalEntity<Output, C> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render(Lossless::Normalized).into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json), Lossless::Normalized)
    }
}

/// The aggregated periods are `@list` values in the core context, so compaction keeps their arrays.
impl<C: Completeness> Sealed for AggregatedTemporalEntity<C> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

/// The temporal values are `@list` values in the core context, so compaction keeps their arrays.
impl<C: Completeness> Sealed for SimplifiedTemporalEntity<C> {
    type ReadError = DocumentError;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self, _: Seal) -> Json {
        self.render().into_json()
    }

    fn read(_: Seal, json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}
