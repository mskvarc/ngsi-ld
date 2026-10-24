//! The JSON forms of the query languages as linked documents: an entity selector (NGSI-LD clause 5.2.33), a
//! geoquery (clause 5.2.13) and a temporal query (clause 5.2.21). None carries an `@context`.

use crate::{
    linked_data::document::{sealed::Sealed, shape::Shape},
    model::DocumentError,
    query::{
        EntitySelector,
        EntitySelectorError,
        geoquery::{GeoQuery, GeoQueryError},
        temporal_query::{TemporalQuery, TemporalQueryError},
    },
};
use jstrict::Value as Json;

impl Sealed for EntitySelector {
    type ReadError = DocumentError<EntitySelectorError>;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

/// `coordinates` is a `@list` in the core context, so compaction keeps its array.
impl Sealed for GeoQuery {
    type ReadError = DocumentError<GeoQueryError>;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

impl Sealed for TemporalQuery {
    type ReadError = DocumentError<TemporalQueryError>;

    const SHAPE: Shape = Shape::Kept;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}
