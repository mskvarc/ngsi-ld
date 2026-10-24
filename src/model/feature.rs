//! The `GeoJSON` representation of entities (NGSI-LD clauses 4.5.16, 4.5.17 and 5.2.29 to 5.2.31): one entity as a
//! `GeoJSON` Feature, a list of entities as a `GeoJSON` `FeatureCollection` (IETF RFC 7946).

use crate::model::{
    Context,
    DocumentError,
    DocumentErrorKind,
    Entity,
    Lossless,
    MemberError,
    Output,
    RawJson,
    SimplifiedEntity,
    geometry::Geometry,
    reader::{array, context, object, once, parse_json, required, string, unexpected, within},
    representation::{entity_object, read::read_entity, write::write_context},
};
use jstrict::{Object, Value as Json, object::Entry};

#[cfg(feature = "linked-data")]
mod names;

/// The `GeoJSON` object types of the `GeoJSON` representation (Tables 5.2.29-1 and 5.2.30-1), spelled as their `type`
/// member.
///
/// The enum is exhaustive: clauses 4.5.16 and 4.5.17 use these two types only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum GeoJsonType {
    /// `Feature`: one entity.
    Feature,
    /// `FeatureCollection`: a list of entities.
    FeatureCollection,
}

/// The members of a Feature (Table 5.2.29-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
enum FeatureMember {
    /// `id`, the entity id.
    #[strum(serialize = "id")]
    Id,
    /// `type`, always `Feature`.
    #[strum(serialize = "type")]
    Type,
    /// `geometry`.
    #[strum(serialize = "geometry")]
    Geometry,
    /// `properties`, the entity type and attributes.
    #[strum(serialize = "properties")]
    Properties,
    /// `@context`.
    #[strum(serialize = "@context")]
    Context,
}

/// The members of a `FeatureCollection` (Table 5.2.30-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
enum CollectionMember {
    /// `type`, always `FeatureCollection`.
    #[strum(serialize = "type")]
    Type,
    /// `features`.
    #[strum(serialize = "features")]
    Features,
    /// `@context`.
    #[strum(serialize = "@context")]
    Context,
}

/// An entity in `GeoJSON` representation (NGSI-LD clauses 4.5.16.2, 4.5.17.1 and 5.2.29): a `GeoJSON` Feature.
///
/// `E` is the entity: an [`Entity<Output>`] for the normalized and concise `GeoJSON` representation, a
/// [`SimplifiedEntity`] for the simplified one. Its `id` is the Feature's `id`; its other members are the
/// Feature's `properties`.
///
/// The Feature's `geometry` is the value of the `GeoProperty` the `geometryProperty` parameter selects, `location` by
/// default, or `null` (clause 4.5.16.1); its `properties` hold the entity's `type` and its attributes, in normalized or
/// concise representation (clause 4.5.16.2) or reduced to their values (clause 4.5.17.1), read and written by the
/// same readers and writers as the entity itself.
///
/// Table 5.2.29-1 places the `@context` of a Feature beside its other members; the normalized example of Annex
/// C.2.3 places it inside `properties`. Both are read and written back where they were: the Feature's own in
/// [`Feature::context`], the one inside `properties` as the entity's `@context`.
///
/// ```
/// use ngsi_ld::model::{Feature, SimplifiedEntity};
///
/// let text = r#"{"id":"urn:ngsi-ld:OffStreetParking:Downtown1","type":"Feature","geometry":{"type":"Point","coordinates":[-8.5,41.1]},"properties":{"type":"OffStreetParking","availableSpotNumber":121}}"#;
/// let parking = Feature::<SimplifiedEntity>::parse(text)?;
/// assert_eq!(parking.entity.id.as_str(), "urn:ngsi-ld:OffStreetParking:Downtown1");
/// assert!(parking.geometry.is_some());
/// assert_eq!(parking.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Feature<E> {
    /// The entity: the Feature's `id` and `properties`.
    pub entity: E,
    /// `geometry`: the value of the selected `GeoProperty`, without its sub-attributes, or `None` for the `null`
    /// written when the entity has no such `GeoProperty` (clause 4.5.16.1).
    pub geometry: Option<Geometry>,
    /// The `@context` beside the Feature's other members (Table 5.2.29-1), present when the response carries the
    /// context in its body.
    pub context: Option<Context>,
}

impl<E> Feature<E> {
    /// The Feature of `entity`, with `geometry` as its geometry and no `@context` of its own.
    #[must_use]
    pub const fn new(entity: E, geometry: Option<Geometry>) -> Feature<E> {
        Feature {
            entity,
            geometry,
            context: None,
        }
    }
}

/// A list of entities in `GeoJSON` representation (NGSI-LD clauses 4.5.16.3, 4.5.17.2 and 5.2.30): a `GeoJSON`
/// `FeatureCollection`.
///
/// ```
/// use ngsi_ld::model::{FeatureCollection, SimplifiedEntity};
///
/// let empty = FeatureCollection::<SimplifiedEntity>::parse(
///     r#"{"type":"FeatureCollection","features":[]}"#,
/// )?;
/// assert!(empty.features.is_empty());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FeatureCollection<E> {
    /// `features`, one per entity, in document order; empty when nothing matched (Table 5.2.30-1).
    pub features: Vec<Feature<E>>,
    /// The `@context` of the whole collection (Table 5.2.30-1); the Features carry none of their own (clause
    /// 4.5.16.3).
    pub context: Option<Context>,
}

impl<E> FeatureCollection<E> {
    /// The collection of `features`, with no `@context`.
    #[must_use]
    pub const fn new(features: Vec<Feature<E>>) -> FeatureCollection<E> {
        FeatureCollection { features, context: None }
    }
}

/// Checks that the `type` member names the `GeoJSON` type `expected`.
fn expect_type(json: Json, expected: GeoJsonType) -> Result<(), DocumentError> {
    let found = string(json, "type")?;
    if found == <&str>::from(expected) {
        Ok(())
    } else {
        Err(DocumentError::top(DocumentErrorKind::GeoJsonType { expected, found }))
    }
}

/// Reads the `geometry` member: a geometry, or `null` when the entity has none (clause 4.5.16.1).
fn read_geometry(json: &Json) -> Result<Option<Geometry>, DocumentError> {
    match json {
        Json::Null => Ok(None),
        Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => Geometry::from_json(json)
            .map(Some)
            .map_err(|source| DocumentError::top(MemberError::Geometry { source }.into()).within("geometry")),
    }
}

/// Reads the `properties` member: an object, which never holds the entity `id`, the Feature's own member.
fn read_properties(json: Json) -> Result<Object, DocumentError> {
    let properties = within("properties", object(json))?;
    if properties.contains_key("id") {
        return Err(unexpected("id").within("properties"));
    }
    Ok(properties)
}

/// Reads a Feature, reading its entity (its `id` followed by the members of `properties`) with `read`.
pub(crate) fn read_feature<E>(json: Json, read: impl FnOnce(Json) -> Result<E, DocumentError>) -> Result<Feature<E>, DocumentError> {
    let object = object(json)?;
    let mut id = None;
    let mut geo_type = None;
    let mut geometry = None;
    let mut properties = None;
    let mut feature_context = None;
    for Entry { key, value } in object {
        let key = key.as_str();
        match key.parse::<FeatureMember>() {
            Ok(FeatureMember::Id) => once(&mut id, key, value)?,
            Ok(FeatureMember::Type) => once(&mut geo_type, key, value)?,
            Ok(FeatureMember::Geometry) => once(&mut geometry, key, read_geometry(&value)?)?,
            Ok(FeatureMember::Properties) => once(&mut properties, key, read_properties(value)?)?,
            Ok(FeatureMember::Context) => once(&mut feature_context, key, context(value)?)?,
            Err(_) => return Err(unexpected(key)),
        }
    }
    expect_type(required(geo_type, "type")?, GeoJsonType::Feature)?;
    let geometry = required(geometry, "geometry")?;
    let mut entity = required(properties, "properties")?;
    if let Some(id) = id {
        entity.push_front("id".into(), id);
    }
    Ok(Feature {
        entity: read(Json::Object(entity))?,
        geometry,
        context: feature_context,
    })
}

/// Writes a Feature: `id`, `type`, `geometry`, `properties` and its own `@context`, the entity written by `write`
/// as one object whose `id` becomes the Feature's.
pub(crate) fn feature_object<E>(feature: &Feature<E>, write: impl FnOnce(&E) -> Object) -> Object {
    let mut properties = write(&feature.entity);
    let id = properties.index_of("id").and_then(|index| properties.remove_at(index));
    let mut object = Object::new();
    if let Some(Entry { value, .. }) = id {
        object.push("id".into(), value);
    }
    object.push("type".into(), Json::from(<&str>::from(GeoJsonType::Feature)));
    object.push("geometry".into(), feature.geometry.as_ref().map_or(Json::Null, Geometry::to_json));
    object.push("properties".into(), Json::Object(properties));
    write_context(&mut object, feature.context.as_ref());
    object
}

/// Reads a `FeatureCollection`, reading the entity of every Feature with `read`.
pub(crate) fn read_collection<E>(json: Json, mut read: impl FnMut(Json) -> Result<E, DocumentError>) -> Result<FeatureCollection<E>, DocumentError> {
    let object = object(json)?;
    let mut geo_type = None;
    let mut features = None;
    let mut collection_context = None;
    for Entry { key, value } in object {
        let key = key.as_str();
        match key.parse::<CollectionMember>() {
            Ok(CollectionMember::Type) => once(&mut geo_type, key, value)?,
            Ok(CollectionMember::Features) => once(&mut features, key, value)?,
            Ok(CollectionMember::Context) => once(&mut collection_context, key, context(value)?)?,
            Err(_) => return Err(unexpected(key)),
        }
    }
    expect_type(required(geo_type, "type")?, GeoJsonType::FeatureCollection)?;
    let features = array(required(features, "features")?)?
        .into_iter()
        .map(|feature| read_feature(feature, &mut read))
        .collect::<Result<_, _>>()?;
    Ok(FeatureCollection {
        features,
        context: collection_context,
    })
}

/// Writes a `FeatureCollection`: `type`, `features` and its `@context`, every entity written by `write`.
pub(crate) fn collection_object<E>(collection: &FeatureCollection<E>, mut write: impl FnMut(&E) -> Object) -> Object {
    let mut object = Object::new();
    object.push("type".into(), Json::from(<&str>::from(GeoJsonType::FeatureCollection)));
    let features = collection
        .features
        .iter()
        .map(|feature| Json::Object(feature_object(feature, &mut write)))
        .collect();
    object.push("features".into(), Json::Array(features));
    write_context(&mut object, collection.context.as_ref());
    object
}

impl Feature<Entity<Output>> {
    /// Reads an entity in normalized or concise `GeoJSON` representation (clause 4.5.16.2). The concise reader also
    /// reads normalized attributes (clause 4.5.2.3).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_feature(parse_json(text)?, |entity| read_entity(entity, form))
    }

    /// Writes the Feature with its entity in `form`, in canonical member order: `id`, `type`, `geometry`,
    /// `properties` (the entity's `type`, its attributes and its own `@context`) and the Feature's `@context`.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(feature_object(self, |entity| entity_object(entity, form))))
    }
}

impl Feature<SimplifiedEntity> {
    /// Reads an entity in simplified `GeoJSON` representation (clause 4.5.17.1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        read_feature(parse_json(text)?, SimplifiedEntity::from_json)
    }

    /// Writes the Feature in canonical member order: `id`, `type`, `geometry`, `properties` and its `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(feature_object(self, SimplifiedEntity::to_object)))
    }
}

impl FeatureCollection<Entity<Output>> {
    /// Reads a list of entities in normalized or concise `GeoJSON` representation (clause 4.5.16.3).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_collection(parse_json(text)?, |entity| read_entity(entity, form))
    }

    /// Writes the collection with every entity in `form`: `type`, `features` and the `@context`.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(collection_object(self, |entity| entity_object(entity, form))))
    }
}

impl FeatureCollection<SimplifiedEntity> {
    /// Reads a list of entities in simplified `GeoJSON` representation (clause 4.5.17.2).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        read_collection(parse_json(text)?, SimplifiedEntity::from_json)
    }

    /// Writes the collection: `type`, `features` and the `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(collection_object(self, SimplifiedEntity::to_object)))
    }
}

#[cfg(test)]
mod tests;
