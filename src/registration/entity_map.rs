//! Entity maps (NGSI-LD clauses 4.3.6.7, 4.5.25 and 5.2.39): which context source registrations hold which
//! entities during a distributed query, so that every page of the query reaches the same sources.
//!
//! The map's values are registration ids, so it lives here, beside [`RegistrationId`].

use crate::{
    model::{
        Context,
        DateTime,
        DocumentError,
        EntityId,
        RawJson,
        keyword::{CONTEXT, NONE},
        reader::{
            array,
            context,
            date_time,
            duplicate,
            fixed_token,
            identifier,
            identifier_text,
            object,
            once,
            parse_json,
            required,
            string,
            unexpected,
            within,
        },
        representation::write::write_context,
    },
    registration::RegistrationId,
};
use indexmap::IndexMap;
use jstrict::{Object, Value as Json, object::Entry};

/// The `type` of an entity map (Table 5.2.39-1).
const ENTITY_MAP_TYPE: &str = "EntityMap";

uri_identifier!(
    /// The identifier of an entity map: a URI the broker generates.
    ///
    /// It is a URI (NGSI-LD Table 5.2.39-1), and the last path segment of the map's resource,
    /// `/entityMaps/{entityMapId}` (Table 6.2-1).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::registration::EntityMapId;
    ///
    /// let id = EntityMapId::from(iri!("urn:ngsi-ld:EntityMap:1"));
    /// assert_eq!(id, "urn:ngsi-ld:EntityMap:1".parse::<EntityMapId>()?);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    EntityMapId
);

/// Where an entity of an entity map is held.
///
/// NGSI-LD Table 5.2.39-2.
///
/// Table 5.2.39-2 says the key `@none` refers to an entity held locally; a key names an entity, and what is local is
/// the source holding it, so `@none` is read where the sources are listed, as [`EntitySource::Local`]. The enum is
/// exhaustive: the table defines these two sources only.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::registration::EntitySource;
///
/// let remote =
///     EntitySource::Registration(iri!("urn:ngsi-ld:ContextSourceRegistration:r1").into());
/// assert_ne!(remote, EntitySource::Local);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntitySource {
    /// The broker itself (`@none`).
    Local,
    /// The context source of a registration.
    Registration(RegistrationId),
}

impl EntitySource {
    /// The source as written in an entity map.
    fn as_str(&self) -> &str {
        match self {
            Self::Local => NONE,
            Self::Registration(id) => id.as_str(),
        }
    }
}

/// An entity map.
///
/// It is what Retrieve `EntityMap` answers, and the body of the answer of Create `EntityMap` for Query Entities and for
/// Query Temporal Evolution (NGSI-LD clause 5.2.39).
///
/// Only a broker writes one; it is read strictly, a member Tables 5.2.39-1 and 5.2.39-2 do not define failing the
/// read. Clause 4.5.25 calls the expiry `expiredBy`, while Table 5.2.39-1 and the core context call it
/// `expiresAt`, and the operations return the map "as mandated by clause 5.2.39", so `expiresAt` is read and
/// `expiredBy` refused.
///
/// ```
/// use ngsi_ld::registration::{EntityMap, EntitySource};
///
/// let text = concat!(
///     concat!(
///         r#"{"id":"urn:ngsi-ld:EntityMap:1","type":"EntityMap","#,
///         r#""expiresAt":"2030-01-01T00:00:00Z","#
///     ),
///     r#""entityMap":{"urn:ngsi-ld:Vehicle:A4567":["@none"]},"linkedMaps":{}}"#
/// );
/// let map = EntityMap::parse(text)?;
/// assert_eq!(
///     map.entity_map.values().next().map(Vec::as_slice),
///     Some([EntitySource::Local].as_slice())
/// );
/// assert_eq!(map.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityMap {
    /// `id`.
    pub id: EntityMapId,
    /// `expiresAt`: when the map can no longer be used.
    pub expires_at: DateTime,
    /// `entityMap`: each entity the map holds, with the sources that returned data for it, in the order written.
    pub entity_map: IndexMap<EntityId, Vec<EntitySource>>,
    /// `linkedMaps`: for each registration the query was forwarded to, the entity map its context source used.
    pub linked_maps: IndexMap<RegistrationId, EntityMapId>,
    /// The `@context` the document carried (clause 4.5.25).
    pub context: Option<Context>,
}

impl EntityMap {
    /// The map `id`, expiring at `expires_at`, holding no entity and linking no map.
    #[must_use]
    pub fn new(id: EntityMapId, expires_at: DateTime) -> EntityMap {
        EntityMap {
            id,
            expires_at,
            entity_map: IndexMap::new(),
            linked_maps: IndexMap::new(),
            context: None,
        }
    }

    /// Reads the map.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the map in the member order of Tables 5.2.39-1 and 5.2.39-2: `id`, `type`, `expiresAt`, `entityMap`,
    /// `linkedMaps`, then `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Self::head(&self.id, &self.expires_at);
        let mut entities = Object::new();
        for (id, sources) in &self.entity_map {
            entities.push(
                id.as_str().into(),
                Json::Array(sources.iter().map(|source| Json::from(source.as_str())).collect()),
            );
        }
        object.push("entityMap".into(), Json::Object(entities));
        let mut linked = Object::new();
        for (registration, map) in &self.linked_maps {
            linked.push(registration.as_str().into(), Json::from(map.as_str()));
        }
        object.push("linkedMaps".into(), Json::Object(linked));
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// The body of Update `EntityMap`: the members of Table 5.2.39-1, `{"id": ..., "type": "EntityMap", "expiresAt":
    /// ...}`. Clause 5.14.2.3 sends "a JSON-LD document representing an `EntityMap`", whose output-only members a
    /// broker ignores (clause 5.14.2.4), so the expiry is the one member a client can change.
    #[cfg(feature = "http")]
    pub(crate) fn update_object(id: &EntityMapId, expires_at: &DateTime) -> Object {
        Self::head(id, expires_at)
    }

    /// The members of Table 5.2.39-1.
    fn head(id: &EntityMapId, expires_at: &DateTime) -> Object {
        let mut object = Object::new();
        object.push("id".into(), Json::from(id.as_str()));
        object.push("type".into(), Json::from(ENTITY_MAP_TYPE));
        object.push("expiresAt".into(), Json::from(expires_at.to_string()));
        object
    }

    /// Reads the map from JSON already parsed, such as the body of Retrieve `EntityMap`.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut expires_at = None;
        let mut entity_map = None;
        let mut linked_maps = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, identifier(value, "id"))?)?,
                "type" => once(&mut typed, key, within(key, fixed_token(value, "type", ENTITY_MAP_TYPE))?)?,
                "expiresAt" => once(&mut expires_at, key, within(key, date_time(value, "expiresAt"))?)?,
                "entityMap" => once(&mut entity_map, key, within(key, read_entities(value))?)?,
                "linkedMaps" => once(&mut linked_maps, key, within(key, read_linked_maps(value))?)?,
                CONTEXT => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            expires_at: required(expires_at, "expiresAt")?,
            entity_map: required(entity_map, "entityMap")?,
            linked_maps: required(linked_maps, "linkedMaps")?,
            context: document_context,
        })
    }
}

/// Reads `entityMap`: an object from entity ids to arrays of sources.
fn read_entities(json: Json) -> Result<IndexMap<EntityId, Vec<EntitySource>>, DocumentError> {
    let mut entities = IndexMap::new();
    for Entry { key, value } in object(json)? {
        let id: EntityId = identifier_text(key.as_str())?;
        if entities.contains_key(&id) {
            return Err(duplicate(key.as_str()));
        }
        let sources = within(key.as_str(), read_sources(value))?;
        entities.insert(id, sources);
    }
    Ok(entities)
}

/// Reads the sources of one entity: an array of registration ids, `@none` standing for the broker itself.
fn read_sources(json: Json) -> Result<Vec<EntitySource>, DocumentError> {
    array(json)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            string(item, "entityMap")
                .and_then(|text| match text.as_str() {
                    NONE => Ok(EntitySource::Local),
                    registration => identifier_text(registration).map(EntitySource::Registration),
                })
                .map_err(|error| error.at_index(index))
        })
        .collect()
}

/// Reads `linkedMaps`: an object from registration ids to entity map ids.
fn read_linked_maps(json: Json) -> Result<IndexMap<RegistrationId, EntityMapId>, DocumentError> {
    let mut linked = IndexMap::new();
    for Entry { key, value } in object(json)? {
        let registration: RegistrationId = identifier_text(key.as_str())?;
        if linked.contains_key(&registration) {
            return Err(duplicate(key.as_str()));
        }
        let map = within(key.as_str(), identifier(value, "linkedMaps"))?;
        linked.insert(registration, map);
    }
    Ok(linked)
}

#[cfg(test)]
mod tests;
