//! The simplified representation (NGSI-LD clause 4.5.4): an entity reduced to its attribute values.

use crate::model::{
    Attribute,
    AttributeName,
    Complete,
    Completeness,
    Context,
    DatasetId,
    Direction,
    DocumentError,
    DocumentErrorKind,
    Entity,
    EntityDirection,
    EntityId,
    EntityTypeName,
    GeoValue,
    LanguageMap,
    OneOrMany,
    Projected,
    RawJson,
    Scope,
    Value,
    VocabTerm,
    entity::member::EntityMember,
    geometry::Geometry,
    null::{self, NGSI_LD_NULL},
    reader::{attribute_name, context, duplicate, entity_types, identifier_text, object, once, one_or_many, parse_json, parsed, scopes, unexpected, value},
    representation::{
        member::{entity_id, identifying},
        write::{write_context, write_identity},
    },
};
use indexmap::IndexMap;
use jstrict::{Object, Value as Json, object::Entry};
use std::{borrow::Cow, mem};

pub(super) mod fragment;
#[cfg(feature = "linked-data")]
mod names;

/// The simplified value of one attribute (NGSI-LD clause 4.5.4).
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum SimplifiedValue {
    /// A Property value, a Relationship object or objects, a `ListProperty` or `ListRelationship` list: the
    /// JSON the simplified form writes for them, which does not say which of these it was.
    Value(Value),
    /// A `GeoProperty` value.
    Geometry(Geometry),
    /// A `LanguageProperty`, written `{"languageMap": ...}`.
    LanguageMap(LanguageMap),
    /// A `VocabProperty`, written `{"vocab": ...}`.
    Vocab(OneOrMany<VocabTerm>),
    /// A `JsonProperty`, written `{"json": ...}`.
    Json(RawJson),
    /// Several instances, written `{"dataset": {...}}` keyed by dataset id, `@none` for the default instance.
    Dataset(IndexMap<DatasetId, SimplifiedValue>),
}

/// The member that wraps a simplified value which is not written bare (clause 4.5.4). The specification closes
/// the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
enum WrappingMember {
    /// `languageMap`.
    #[strum(serialize = "languageMap")]
    LanguageMap,
    /// `vocab`.
    #[strum(serialize = "vocab")]
    Vocab,
    /// `json`.
    #[strum(serialize = "json")]
    Json,
    /// `dataset`.
    #[strum(serialize = "dataset")]
    Dataset,
}

/// An entity in simplified representation (NGSI-LD clause 4.5.4).
///
/// The representation is lossy: sub-attributes, `observedAt`, `unitCode` and the attribute kinds are gone, and
/// a Relationship's object cannot be told from a string Property value. It is therefore its own model.
///
/// `C` says whether the entity holds its `id` and `type`: every entity does ([`Complete`], the default), except
/// one in the answer to a projection that removed them ([`Projected`], clause 4.21).
///
/// ```
/// use ngsi_ld::model::SimplifiedEntity;
///
/// let vehicle = SimplifiedEntity::parse(
///     r#"{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "brandName": "Mercedes"}"#,
/// )?;
/// assert_eq!(vehicle.attributes.len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SimplifiedEntity<C: Completeness = Complete> {
    /// `id`; optional only in a projected entity.
    pub id: C::Member<EntityId>,
    /// `type`; optional only in a projected entity.
    pub entity_type: C::Member<OneOrMany<EntityTypeName>>,
    /// `scope`.
    pub scope: Option<OneOrMany<Scope>>,
    /// The attribute values, in document order.
    pub attributes: IndexMap<AttributeName, SimplifiedValue>,
    /// The `@context` the document carried (clause 4.5.4); [`render`](SimplifiedEntity::render) writes it last.
    pub context: Option<Context>,
}

// No derive reduces every attribute to its simplified value.
impl<D: EntityDirection, C: Completeness> From<&Entity<D, C>> for SimplifiedEntity<C> {
    fn from(entity: &Entity<D, C>) -> Self {
        Self {
            // The simplified entity is a separate document, so it owns copies of the shared members.
            id: entity.id.clone(),
            entity_type: entity.entity_type.clone(),
            scope: entity.scope.clone(),
            attributes: entity
                .attributes
                .iter()
                .map(|(name, instances)| (name.clone(), simplify_instances(instances.iter().map(simplify))))
                .collect(),
            context: entity.context.clone(),
        }
    }
}

// No derive reduces every attribute to its simplified value. The entity is taken apart, so nothing is copied.
impl<D: EntityDirection, C: Completeness> From<Entity<D, C>> for SimplifiedEntity<C> {
    fn from(entity: Entity<D, C>) -> Self {
        Self {
            id: entity.id,
            entity_type: entity.entity_type,
            scope: entity.scope,
            attributes: entity
                .attributes
                .into_iter()
                .map(|(name, instances)| (name, simplify_instances(instances.into_vec().into_iter().map(into_simplified))))
                .collect(),
            context: entity.context,
        }
    }
}

/// The simplified value of every instance, each given with its dataset id: the value of the only one, or the
/// `dataset` object, which owns its dataset ids.
fn simplify_instances<'a>(mut instances: impl Iterator<Item = (Option<Cow<'a, DatasetId>>, SimplifiedValue)>) -> SimplifiedValue {
    match (instances.next(), instances.next()) {
        (Some((_, only)), None) => only,
        (first, second) => SimplifiedValue::Dataset(
            first
                .into_iter()
                .chain(second)
                .chain(instances)
                .map(|(dataset_id, value)| (dataset_id.map_or(DatasetId::DefaultInstance, Cow::into_owned), value))
                .collect(),
        ),
    }
}

/// The dataset id and the simplified value of one instance, which is left as it is.
fn simplify<D: Direction>(attribute: &Attribute<D>) -> (Option<Cow<'_, DatasetId>>, SimplifiedValue) {
    // The simplified value is a separate document, so it owns copies of the values.
    let value = match attribute {
        Attribute::Property(property) => SimplifiedValue::Value(property.value().clone()),
        Attribute::GeoProperty(property) => simplify_geo_value(property.value().clone()),
        Attribute::LanguageProperty(property) => SimplifiedValue::LanguageMap(property.language_map().clone()),
        Attribute::VocabProperty(property) => SimplifiedValue::Vocab(property.vocab().clone()),
        Attribute::ListProperty(property) => SimplifiedValue::Value(Value::Array(property.value_list().to_vec())),
        Attribute::JsonProperty(property) => SimplifiedValue::Json(property.json().clone()),
        Attribute::Relationship(relationship) => simplify_object(relationship.object()),
        Attribute::ListRelationship(relationship) => simplify_object_list(relationship.object_list()),
    };
    (attribute.meta().dataset_id.as_ref().map(Cow::Borrowed), value)
}

/// The dataset id and the simplified value of one instance, taken out of it.
fn into_simplified<D: Direction>(attribute: Attribute<D>) -> (Option<Cow<'static, DatasetId>>, SimplifiedValue) {
    let (dataset_id, value) = match attribute {
        Attribute::Property(property) => (property.meta.dataset_id, SimplifiedValue::Value(property.value)),
        Attribute::GeoProperty(property) => (property.meta.dataset_id, simplify_geo_value(property.value)),
        Attribute::LanguageProperty(property) => (property.meta.dataset_id, SimplifiedValue::LanguageMap(property.language_map)),
        Attribute::VocabProperty(property) => (property.meta.dataset_id, SimplifiedValue::Vocab(property.vocab)),
        Attribute::ListProperty(property) => (property.meta.dataset_id, SimplifiedValue::Value(Value::Array(property.value_list))),
        Attribute::JsonProperty(property) => (property.meta.dataset_id, SimplifiedValue::Json(property.json)),
        Attribute::Relationship(relationship) => (relationship.meta.dataset_id, simplify_object(&relationship.object)),
        Attribute::ListRelationship(relationship) => (relationship.meta.dataset_id, simplify_object_list(&relationship.object_list)),
    };
    (dataset_id.map(Cow::Owned), value)
}

/// The simplified value of a `GeoProperty`: its geometry, or the NGSI-LD Null a deleted one holds.
fn simplify_geo_value(value: GeoValue) -> SimplifiedValue {
    match value {
        GeoValue::Geometry(geometry) => SimplifiedValue::Geometry(geometry),
        GeoValue::Null => SimplifiedValue::Value(Value::Text(NGSI_LD_NULL.as_str().to_owned())),
    }
}

/// The simplified value of a Relationship: the id of its object, or an array of the ids of its objects.
fn simplify_object(object: &OneOrMany<EntityId>) -> SimplifiedValue {
    let text = |id: &EntityId| Value::Text(id.to_string());
    SimplifiedValue::Value(object.as_one().map_or_else(|| Value::Array(object.iter().map(text).collect()), text))
}

/// The simplified value of a `ListRelationship`: an array of the ids of its objects.
fn simplify_object_list(objects: &[EntityId]) -> SimplifiedValue {
    SimplifiedValue::Value(Value::Array(objects.iter().map(|id| Value::Text(id.to_string())).collect()))
}

/// Writes `json` wrapped in the single member `member`: `{"languageMap": ...}`, `{"dataset": ...}`, ...
fn wrapped(member: WrappingMember, json: Json) -> Json {
    let mut object = Object::new();
    object.push(<&str>::from(member).into(), json);
    Json::Object(object)
}

/// Writes `{"dataset": {...}}`: one member per instance, keyed by dataset id, `@none` for the default instance.
fn dataset_json<'a>(instances: impl IntoIterator<Item = (&'a DatasetId, Json)>) -> Json {
    wrapped(
        WrappingMember::Dataset,
        Json::Object(instances.into_iter().map(|(dataset_id, json)| (dataset_id.to_string().into(), json)).collect()),
    )
}

impl SimplifiedValue {
    /// Whether the value is the record of a deleted attribute, as a notification in the simplified representation
    /// reports one (clauses 4.5.0 and 5.8.6): the bare NGSI-LD Null, the Null alone in a list, a `languageMap`
    /// whose `@none` text is the Null, the Null alone as `vocab`, or the Null as `json`. The simplified form does
    /// not say which kind the attribute was, so each shape is recognised whatever the kind.
    ///
    /// ```
    /// use ngsi_ld::model::SimplifiedEntity;
    ///
    /// let vehicle = SimplifiedEntity::parse(r#"{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": 55, "isParked": "urn:ngsi-ld:null"}"#)?;
    /// let deleted: Vec<String> =
    ///     vehicle.attributes.iter().filter(|(_, value)| value.is_deleted()).map(|(name, _)| name.to_string()).collect();
    /// assert_eq!(deleted, ["isParked"]);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        null::is_null_simplified(self)
    }

    /// Writes the value.
    fn to_json(&self) -> Json {
        match self {
            Self::Value(value) => value.to_json(),
            Self::Geometry(geometry) => geometry.to_json(),
            Self::LanguageMap(map) => wrapped(WrappingMember::LanguageMap, map.to_json()),
            Self::Vocab(vocab) => wrapped(WrappingMember::Vocab, vocab.to_json(|term| Json::from(term.to_string()))),
            // The raw JSON is carried verbatim, so the tree is copied as it is.
            Self::Json(json) => wrapped(WrappingMember::Json, json.to_json()),
            Self::Dataset(instances) => dataset_json(instances.iter().map(|(dataset_id, value)| (dataset_id, value.to_json()))),
        }
    }

    /// Reads the simplified value of an attribute: the `dataset` object of several instances, or one instance.
    fn from_json(mut json: Json) -> Result<Self, DocumentError> {
        match take_wrapped(&mut json) {
            Some((WrappingMember::Dataset, instances)) => dataset(instances),
            Some((member, payload)) => Self::from_wrapped(member, payload),
            None => Self::from_bare(json),
        }
    }

    /// Reads the simplified value of one instance. A `dataset` holds instances, never another `dataset`, so one is
    /// refused here before it is read, which keeps a document from nesting them.
    fn instance_from_json(mut json: Json) -> Result<Self, DocumentError> {
        match take_wrapped(&mut json) {
            Some((member, payload)) => Self::from_wrapped(member, payload),
            None => Self::from_bare(json),
        }
    }

    /// Reads the payload of an instance written `{"languageMap": ...}`, `{"vocab": ...}` or `{"json": ...}`.
    fn from_wrapped(member: WrappingMember, payload: Json) -> Result<Self, DocumentError> {
        match member {
            WrappingMember::LanguageMap => LanguageMap::from_json(payload)
                .map(Self::LanguageMap)
                .map_err(|source| DocumentError::top(DocumentErrorKind::LanguageMap { source })),
            WrappingMember::Vocab => one_or_many(payload, |item| parsed::<VocabTerm, _>(item, "vocab")).map(Self::Vocab),
            WrappingMember::Json => Ok(Self::Json(RawJson::from_json(payload))),
            WrappingMember::Dataset => Err(unexpected(<&str>::from(WrappingMember::Dataset))),
        }
    }

    /// Reads an instance written bare: a geometry is a geometry, anything else is a value.
    fn from_bare(json: Json) -> Result<Self, DocumentError> {
        if let Ok(geometry) = Geometry::from_json(&json) {
            return Ok(Self::Geometry(geometry));
        }
        value(json).map(Self::Value)
    }
}

/// Takes the payload out of an object with exactly one `languageMap`, `vocab`, `json` or `dataset` member; `None`
/// leaves any other JSON as it is. The payload is moved out, never copied, so a deep payload costs nothing here.
fn take_wrapped(json: &mut Json) -> Option<(WrappingMember, Json)> {
    let Json::Object(object) = json else {
        return None;
    };
    if object.len() != 1 {
        return None;
    }
    let (key, payload) = object.iter_mut().next()?;
    let member = key.as_str().parse::<WrappingMember>().ok()?;
    Some((member, mem::replace(payload, Json::Null)))
}

/// Reads `{"dataset": {...}}`, whose values are the simplified values of single instances.
fn dataset(json: Json) -> Result<SimplifiedValue, DocumentError> {
    let object = object(json)?;
    object
        .into_iter()
        .map(|Entry { key, value }| {
            let dataset_id = identifier_text::<DatasetId, _>(key.as_str())?;
            let instance = SimplifiedValue::instance_from_json(value).map_err(|error| error.within(key.as_str()))?;
            Ok((dataset_id, instance))
        })
        .collect::<Result<_, _>>()
        .map(SimplifiedValue::Dataset)
}

impl<C: Completeness> SimplifiedEntity<C> {
    /// Writes the entity in simplified representation: `id`, `type`, `scope`, the attribute values in their order,
    /// and the `@context` last when the entity has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// The entity as a JSON object, in the member order of [`render`](Self::render).
    pub(crate) fn to_object(&self) -> Object {
        let mut object = Object::new();
        write_identity::<C>(&mut object, &self.id, &self.entity_type);
        if let Some(scope) = &self.scope {
            object.push("scope".into(), scope.to_json(|scope| Json::from(scope.to_string())));
        }
        for (name, value) in &self.attributes {
            object.push(name.as_ref().into(), value.to_json());
        }
        write_context(&mut object, self.context.as_ref());
        object
    }

    /// Reads an entity in simplified representation from a JSON value.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let object = object(json)?;
        let mut id = None;
        let mut entity_type = None;
        let mut scope = None;
        let mut entity_context = None;
        let mut attributes = IndexMap::new();
        for Entry { key, value } in object {
            let key = key.as_str();
            match key.parse::<EntityMember>() {
                Ok(EntityMember::Id) => once(&mut id, key, entity_id(value, "id")?)?,
                Ok(EntityMember::Type) => once(&mut entity_type, key, entity_types(value)?)?,
                Ok(EntityMember::Scope) => once(&mut scope, key, scopes(value)?)?,
                Ok(EntityMember::Context) => once(&mut entity_context, key, context(value)?)?,
                // Clause 4.5.4 lists no `expiresAt` among the members of a simplified entity.
                Ok(EntityMember::ExpiresAt) => return Err(unexpected(key)),
                Err(_) => {
                    let name = attribute_name(key)?;
                    let value = SimplifiedValue::from_json(value).map_err(|error| error.within(key))?;
                    if attributes.insert(name, value).is_some() {
                        return Err(duplicate(key));
                    }
                }
            }
        }
        Ok(Self {
            id: identifying::<C, _>(id, "id")?,
            entity_type: identifying::<C, _>(entity_type, "type")?,
            scope,
            attributes,
            context: entity_context,
        })
    }
}

/// Reads simplified JSON text, keeping its `@context`.
fn parse_simplified<C: Completeness>(text: &str) -> Result<SimplifiedEntity<C>, DocumentError> {
    SimplifiedEntity::from_json(parse_json(text)?)
}

impl SimplifiedEntity {
    /// Reads an entity in simplified representation from JSON text, keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a missing `id` or `type`.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        parse_simplified(text)
    }
}

impl SimplifiedEntity<Projected> {
    /// Reads an entity a projection may have reduced (clause 4.21) in simplified representation from JSON text,
    /// keeping its `@context`; `id` and `type` may be missing. It has its own name so that
    /// [`SimplifiedEntity::parse`] keeps reading complete entities without naming their completeness.
    ///
    /// ```
    /// use ngsi_ld::model::SimplifiedEntity;
    ///
    /// let speed = SimplifiedEntity::parse_projected(r#"{"speed": 80}"#)?;
    /// assert_eq!(speed.id, None);
    /// assert_eq!(speed.attributes.len(), 1);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse_projected(text: &str) -> Result<Self, DocumentError> {
        parse_simplified(text)
    }
}
