//! Entity map fragments (NGSI-LD clauses 5.2.39 and 5.14.2): the changes Update `EntityMap` applies to a map.

use crate::{
    model::{
        DateTime,
        DocumentError,
        RawJson,
        keyword::CONTEXT,
        reader::{context, date_time, fixed_token, identifier, object, once, parse_json, unexpected, within},
    },
    registration::{
        EntityMapId,
        entity_map::{ENTITY_MAP_TYPE, read_entities, read_linked_maps},
    },
};
use jstrict::{Object, Value as Json, object::Entry};

/// The changes Update `EntityMap` applies to an entity map.
///
/// NGSI-LD clauses 5.2.39 and 5.14.2.
///
/// A partial update "only changes the elements provided" (clause 5.14.2.1), and the expiry is the one member of
/// Table 5.2.39-1 a client can change: `entityMap` and `linkedMaps` are output-only and a broker ignores them
/// (clause 5.14.2.4). An empty fragment changes nothing.
///
/// It is written with `"type": "EntityMap"` and without `id`: the binding names the map.
/// [`EntityMapFragment::parse`] reads a body, which may leave `type` out and name the map with an `id`; neither is
/// kept.
///
/// ```
/// use ngsi_ld::registration::EntityMapFragment;
///
/// let fragment = EntityMapFragment::new("2030-01-01T00:00:00Z".parse()?);
/// assert_eq!(
///     fragment.render().to_string(),
///     r#"{"type":"EntityMap","expiresAt":"2030-01-01T00:00:00Z"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityMapFragment {
    /// `expiresAt`: the new expiry, when the fragment moves it.
    pub expires_at: Option<DateTime>,
}

impl EntityMapFragment {
    /// The fragment that moves the expiry of the map to `expires_at`.
    #[must_use]
    pub const fn new(expires_at: DateTime) -> EntityMapFragment {
        EntityMapFragment { expires_at: Some(expires_at) }
    }

    /// Reads the body of Update `EntityMap`.
    ///
    /// Clause 5.14.2.3 sends "a JSON-LD document representing an `EntityMap`", which clause 5.4 lets leave `id` and
    /// `type` out where the operation names the target. The output-only `entityMap` and `linkedMaps` are read and
    /// checked, and not kept, since a broker ignores them (clause 5.14.2.4). The `id` and the `@context` are checked
    /// and not kept either: the binding names the map and delivers the context (clause 6.3.5).
    ///
    /// ```
    /// use ngsi_ld::registration::EntityMapFragment;
    ///
    /// let fragment = EntityMapFragment::parse(r#"{"expiresAt":"2030-01-01T00:00:00Z"}"#)?;
    /// assert_eq!(
    ///     fragment
    ///         .expires_at
    ///         .map(|expiry| expiry.to_string())
    ///         .as_deref(),
    ///     Some("2030-01-01T00:00:00Z")
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault: a member given twice, a `type` other than `EntityMap`,
    /// an `id` that is not a URI, an `expiresAt` that is not a date and time, and a member Tables 5.2.39-1 and
    /// 5.2.39-2 do not define.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        let mut typed = None;
        let mut id: Option<EntityMapId> = None;
        let mut expires_at = None;
        let mut entity_map = None;
        let mut linked_maps = None;
        let mut document_context = None;
        for Entry { key, value } in object(parse_json(text)?)? {
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
        Ok(Self { expires_at })
    }

    /// Writes the fragment: `type`, then `expiresAt` when the fragment moves the expiry.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// Writes the fragment without an `@context`, which the request pipeline delivers (clause 6.3.5).
    fn to_object(&self) -> Object {
        let mut object = Object::new();
        object.push("type".into(), Json::from(ENTITY_MAP_TYPE));
        if let Some(expires_at) = &self.expires_at {
            object.push("expiresAt".into(), Json::from(expires_at.to_string()));
        }
        object
    }
}

#[cfg(test)]
mod tests;
