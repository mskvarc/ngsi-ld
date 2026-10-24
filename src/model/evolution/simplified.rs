//! The simplified temporal representation of an entity (NGSI-LD clause 4.5.9, `format=temporalValues`): each
//! attribute reduced to pairs of a payload and the time it was recorded at.

use crate::model::{
    AttributeKind,
    AttributeName,
    Complete,
    Completeness,
    Context,
    DocumentError,
    DocumentErrorKind,
    EntityId,
    EntityTypeName,
    OneOrMany,
    Projected,
    RawJson,
    entity::member::EntityMember,
    evolution::{
        ScopeValue,
        TemporalValue,
        TemporalValues,
        simplified::values::{read_pairs, read_temporal_attribute, temporal_attribute, write_pairs},
    },
    reader::{attribute_name, context, duplicate, entity_types, object, once, parse_json, unexpected},
    representation::{
        member::{entity_id, identifying},
        write::{write_context, write_identity},
    },
};
use indexmap::IndexMap;
use jstrict::{Object, Value as Json, object::Entry};

pub(super) mod input;
pub(super) mod output;
pub(super) mod values;

/// The temporal evolution of an entity in simplified temporal representation (NGSI-LD clause 4.5.9): `id`, `type`,
/// the scope's values over time (Annex C.5.16), each attribute's [`TemporalValues`], and the `@context`.
///
/// A broker writes it as the answer to `format=temporalValues` (clause 6.3.12), where a payload may be the NGSI-LD Null
/// of a deleted instance. `C` says whether the entity holds its `id` and `type`, which a projection may remove (clause
/// 4.21). What a client sends in this representation is a [`SimplifiedTemporalInput`](input::SimplifiedTemporalInput),
/// which holds no Null.
///
/// ```
/// use ngsi_ld::model::evolution::{SimplifiedTemporalEntity, TemporalValues};
///
/// let vehicle = SimplifiedTemporalEntity::parse(
///     r#"{
///       "id": "urn:ngsi-ld:Vehicle:B9211",
///       "type": "Vehicle",
///       "speed": {
///         "type": "Property",
///         "values": [[120, "2018-08-01T12:03:00Z"], [80, "2018-08-01T12:05:00Z"]]
///       }
///     }"#,
/// )?;
/// let Some(TemporalValues::Property(points)) = vehicle.attributes.values().next() else {
///     anyhow::bail!("expected the values of a Property");
/// };
/// assert_eq!(points.len(), 2);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SimplifiedTemporalEntity<C: Completeness = Complete> {
    /// `id`; optional only in a projected entity.
    pub id: C::Member<EntityId>,
    /// `type`; optional only in a projected entity.
    pub entity_type: C::Member<OneOrMany<EntityTypeName>>,
    /// The values of `scope` over time; empty when the entity has no `scope` member.
    pub scope: Vec<TemporalValue<ScopeValue>>,
    /// The temporal values of each attribute, in document order.
    pub attributes: IndexMap<AttributeName, TemporalValues>,
    /// The `@context` the document carried; [`render`](Self::render) writes it last.
    pub context: Option<Context>,
}

impl<C: Completeness> SimplifiedTemporalEntity<C> {
    /// Writes the entity in simplified temporal representation: `id`, `type`, `scope`, the attributes in their order,
    /// and the `@context` last when the entity has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = self.object();
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// The members without the `@context`: what a request body carries (clause 6.3.5).
    pub(crate) fn object(&self) -> Object {
        let mut object = Object::new();
        write_identity::<C>(&mut object, &self.id, &self.entity_type);
        if !self.scope.is_empty() {
            let values = write_pairs(&self.scope, ScopeValue::to_json);
            object.push("scope".into(), temporal_attribute(AttributeKind::Property, "values", values));
        }
        for (name, values) in &self.attributes {
            object.push(name.as_ref().into(), values.to_json());
        }
        object
    }

    /// Reads an entity in simplified temporal representation from JSON already parsed, such as an element of the
    /// array Query Temporal Evolution answers with.
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
                Ok(EntityMember::Scope) => once(&mut scope, key, scope_values(value).map_err(|error| error.within(key))?)?,
                Ok(EntityMember::Context) => once(&mut entity_context, key, context(value)?)?,
                // Clause 4.5.9 lists no `expiresAt` among the members of a simplified temporal entity.
                Ok(EntityMember::ExpiresAt) => return Err(unexpected(key)),
                Err(_) => {
                    let name = attribute_name(key)?;
                    let values = TemporalValues::from_json(value).map_err(|error| error.within(key))?;
                    if attributes.insert(name, values).is_some() {
                        return Err(duplicate(key));
                    }
                }
            }
        }
        Ok(Self {
            id: identifying::<C, _>(id, "id")?,
            entity_type: identifying::<C, _>(entity_type, "type")?,
            scope: scope.unwrap_or_default(),
            attributes,
            context: entity_context,
        })
    }
}

/// Reads the values of `scope`: a Property whose `values` pair one scope, several, or the NGSI-LD Null of a removed
/// scope with a time (Annex C.5.16).
fn scope_values(json: Json) -> Result<Vec<TemporalValue<ScopeValue>>, DocumentError> {
    let (kind, pairs) = read_temporal_attribute(json, |_| "values")?;
    if kind != AttributeKind::Property {
        return Err(DocumentError::top(DocumentErrorKind::ScopeNotAProperty {
            found: kind.as_str().to_owned(),
        }));
    }
    read_pairs(pairs, ScopeValue::from_json)
}

impl SimplifiedTemporalEntity {
    /// Reads an entity in simplified temporal representation from JSON text, keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a missing `id` or `type`.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }
}

impl SimplifiedTemporalEntity<Projected> {
    /// Reads an entity a projection may have reduced (clause 4.21) in simplified temporal representation; `id` and
    /// `type` may be missing.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse_projected(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }
}
