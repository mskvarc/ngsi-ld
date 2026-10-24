//! The temporal evolution of an entity in normalized or concise representation (NGSI-LD clauses 4.5.6 and 5.2.20).

use crate::{
    model::{
        Attribute,
        AttributeDraft,
        AttributeName,
        AttributeNameDraft,
        BuildError,
        Complete,
        Completeness,
        Context,
        DateTime,
        DocumentError,
        EntityDirection,
        EntityId,
        EntityTypeName,
        IdDraft,
        Input,
        InstanceId,
        Lossless,
        OneOrMany,
        Output,
        RawJson,
        TypeDraft,
        attribute::pending::PendingAttributes,
        evolution::{Evolution, ScopeInstance, TemporalAttributes, read::read_temporal_entity, write::temporal_entity_object},
        reader::parse_json,
        representation::write::{direction::WriteEntityDirection, write_context},
    },
    query::temporal_query::TemporalProperty,
};
use bon::bon;
use jstrict::Value as Json;
use std::num::NonZeroU64;

/// The temporal evolution of an entity in normalized or concise representation (NGSI-LD clauses 4.5.6 and
/// 5.2.20): the entity's members, with every attribute replaced by the sequence of its instances over time.
///
/// `D` is the direction: [`Input`] for what Create or Update Temporal Evolution sends (clause 5.6.11.3), where an
/// instance may name its `instanceId` (see [`EntityDirection::TemporalInstanceId`]) but holds no system member and no
/// NGSI-LD Null; [`Output`] for what Retrieve and Query Temporal Evolution return, where an instance may carry
/// `instanceId` and the system members, and a deleted instance holds the NGSI-LD Null with its `deletedAt` (clauses
/// 4.5.7 and 4.5.8). `C` says whether the entity holds its `id` and `type`, which a projection may remove (clause
/// 4.21).
///
/// A request never records a deletion: `deletedAt` is system generated, and clause 5.2.2 says a Context Producer
/// shall not provide it (an implementation ignores it). Without it a Null instance is not the record of a deleted
/// attribute clauses 4.5.7 and 4.5.8 describe, and clause 5.5.4 refuses the Null in a request outside fragments, so
/// [`Input`] refuses both; a negative test sends such a document through the unchecked path.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{Lossless, Property, evolution::TemporalEntity};
///
/// let vehicle = TemporalEntity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///     .entity_type("Vehicle")
///     .property(
///         "speed",
///         Property::builder(120).observed_at("2018-08-01T12:03:00Z".parse()?),
///     )
///     .property(
///         "speed",
///         Property::builder(80).observed_at("2018-08-01T12:05:00Z".parse()?),
///     )
///     .build()?;
/// assert_eq!(
///     vehicle.render(Lossless::Concise).to_string(),
///     concat!(
///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
///         r#""speed":[{"value":120,"#,
///         r#""observedAt":"2018-08-01T12:03:00Z"},{"value":80,"#,
///         r#""observedAt":"2018-08-01T12:05:00Z"}]}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TemporalEntity<D: EntityDirection, C: Completeness = Complete> {
    /// `id`; optional only in a projected entity.
    pub id: C::Member<EntityId>,
    /// `type`: one entity type, or several (clause 4.16); optional only in a projected entity.
    pub entity_type: C::Member<OneOrMany<EntityTypeName>>,
    /// The evolution of `scope` (clause 4.5.6); empty when the entity has no `scope` member.
    pub scope: Vec<ScopeInstance<D>>,
    /// `expiresAt` (clause 4.22).
    pub expires_at: Option<DateTime>,
    /// The evolution of each attribute, in document order.
    pub attributes: TemporalAttributes<D>,
    /// The output-only members; `()` for a request.
    pub output: D::EntityOutput,
    /// The `@context` the document carried (clause 4.5.1); `render` writes it, a request body never does
    /// (clause 6.3.5).
    pub context: Option<Context>,
}

#[bon]
impl TemporalEntity<Input> {
    /// Starts a builder. `id` and `entity_type` are required, which the compiler checks; instances are added with
    /// the per-kind methods (`property`, `relationship`, ...), where naming an attribute again adds another instance
    /// to its evolution, and scope instances with `scope`. Every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid identifier, type, attribute name and value, every
    /// instance of another kind than the instances before it, and every `location`, `observationSpace` or
    /// `operationSpace` instance that is not a `GeoProperty`.
    #[builder]
    pub fn new(
        #[builder(field)] attributes: PendingAttributes,
        #[builder(field)] scope: Vec<ScopeInstance<Input>>,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        expires_at: Option<DateTime>,
        #[builder(into)] context: Option<Context>,
    ) -> Result<Self, BuildError> {
        assemble(
            TemporalDrafts {
                id,
                entity_type,
                expires_at,
                context,
            },
            attributes,
            scope,
            (),
        )
    }

    /// Writes the temporal entity in normalized or concise representation, every evolution an array of
    /// instances, with its own `@context` last when it has one.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        render_temporal(self, form)
    }

    /// Reads a temporal entity a client could send, in normalized or concise representation.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including any output-only member and any instance
    /// holding the NGSI-LD Null (clause 5.5.4).
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_temporal_entity(parse_json(text)?, form)
    }
}

attribute_methods!(TemporalEntityBuilder, temporal_entity_builder::State, attributes);

/// The members every temporal entity builder takes besides the evolutions and the output-only members.
pub(crate) struct TemporalDrafts {
    /// `id`.
    pub(crate) id: IdDraft,
    /// `type`.
    pub(crate) entity_type: TypeDraft,
    /// `expiresAt`.
    pub(crate) expires_at: Option<DateTime>,
    /// `@context`.
    pub(crate) context: Option<Context>,
}

/// Validates what a temporal entity builder collected and assembles the entity, recording every problem.
pub(crate) fn assemble<D: EntityDirection>(
    drafts: TemporalDrafts,
    attributes: PendingAttributes<D>,
    scope: Vec<ScopeInstance<D>>,
    output: D::EntityOutput,
) -> Result<TemporalEntity<D>, BuildError> {
    let mut problems = Vec::new();
    let id = drafts.id.resolve(&mut problems);
    let entity_type = drafts.entity_type.resolve(&mut problems);
    let attributes = attributes.resolve_evolutions(&mut problems);
    // An unresolved member has recorded its problem, so the error is never empty.
    if let (Some(id), Some(entity_type)) = (id, entity_type) {
        BuildError::finish(
            problems,
            TemporalEntity {
                id,
                entity_type,
                scope,
                expires_at: drafts.expires_at,
                attributes,
                output,
                context: drafts.context,
            },
        )
    } else {
        Err(BuildError::new(problems))
    }
}

impl<S: temporal_entity_builder::State> TemporalEntityBuilder<S> {
    /// Adds an instance to the evolution of the scope (clause 4.5.6), after the ones already given.
    pub fn scope(mut self, instance: ScopeInstance<Input>) -> Self {
        self.scope.push(instance);
        self
    }

    /// Adds an instance that names its `instanceId` to the evolution of `name`, after the ones already given: any
    /// attribute, or attribute builder, a per-kind method takes (see [`EntityDirection::TemporalInstanceId`]).
    pub fn identified<T: Into<Attribute<Input>>>(
        mut self,
        name: impl Into<AttributeNameDraft>,
        instance_id: impl Into<InstanceId>,
        attribute: impl AttributeDraft<T>,
    ) -> Self {
        self.attributes
            .push_identified(name, instance_id.into(), attribute.into_attribute().map(Into::into));
        self
    }
}

impl<C: Completeness> TemporalEntity<Output, C> {
    /// Writes the temporal entity in normalized or concise representation, with its output-only members and its
    /// own `@context` last when it has one.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        render_temporal(self, form)
    }

    /// Reads a temporal entity a broker returned, in normalized or concise representation, keeping its
    /// `@context`. `TemporalEntity::<Output>::parse` requires `id` and `type`;
    /// `TemporalEntity::<Output, Projected>::parse` reads an entity a projection reduced (clause 4.21).
    ///
    /// ```
    /// use ngsi_ld::model::{Lossless, Output, evolution::TemporalEntity};
    ///
    /// let vehicle = TemporalEntity::<Output>::parse(
    ///     r#"{
    ///       "id": "urn:ngsi-ld:Vehicle:B9211",
    ///       "type": "Vehicle",
    ///       "speed": [
    ///         {"type": "Property", "value": 120, "observedAt": "2018-08-01T12:03:00Z"},
    ///         {"type": "Property", "value": 80, "observedAt": "2018-08-01T12:05:00Z"}
    ///       ]
    ///     }"#,
    ///     Lossless::Normalized,
    /// )?;
    /// assert_eq!(
    ///     vehicle
    ///         .attributes
    ///         .iter()
    ///         .map(|(_, evolution)| evolution.len())
    ///         .sum::<usize>(),
    ///     2
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_temporal_entity(parse_json(text)?, form)
    }
}

impl<C: Completeness> TemporalEntity<Output, C> {
    /// Adds the instances of `later`, the answer to the request for the rest of the time range a 206 Partial Content
    /// answer did not cover (clause 6.3.10), dropping each instance this entity already holds: the instances recorded
    /// at the boundary of two ranges come back in both answers (see
    /// [`Coverage::following_query`](crate::http::Coverage::following_query)). Two instances are the same
    /// when they carry the same `instanceId` (clause 4.5.7), or, when one carries none, when they are equal. The
    /// scope's instances are joined the same way; the other members are this entity's.
    pub fn absorb(&mut self, later: TemporalEntity<Output, C>) {
        self.attributes.absorb(later.attributes);
        for instance in later.scope {
            if !self.scope.contains(&instance) {
                self.scope.push(instance);
            }
        }
    }

    /// Adds the instances of `older`, as [`absorb`](Self::absorb) does, for a request with `lastN`: clause 6.3.10
    /// walks such a request backwards, and the request for the rest of the range asks for the last `last_n`
    /// instances of each attribute again, so the join keeps, of each attribute, the `last_n` most recent instances
    /// by `time_property` (clause 4.11), the ones the whole request selects.
    ///
    /// ```
    /// use ngsi_ld::{
    ///     model::{Lossless, Output, evolution::TemporalEntity},
    ///     query::temporal_query::TemporalProperty,
    /// };
    /// use std::num::NonZeroU64;
    ///
    /// let read = |text: &str| TemporalEntity::<Output>::parse(text, Lossless::Concise);
    /// let mut recent = read(concat!(
    ///     r#"{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle","#,
    ///     r#" "speed": [{"value": 80,"#,
    ///     r#" "observedAt": "2018-08-01T12:05:00Z"}]}"#
    /// ))?;
    /// let older = read(
    ///     r#"{"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": [
    ///         {"value": 100, "observedAt": "2018-08-01T12:03:00Z"},
    ///         {"value": 120, "observedAt": "2018-08-01T12:01:00Z"}
    ///     ]}"#,
    /// )?;
    /// recent.absorb_last(
    ///     older,
    ///     NonZeroU64::try_from(2)?,
    ///     TemporalProperty::ObservedAt,
    /// );
    /// assert_eq!(
    ///     recent.render(Lossless::Concise).to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
    ///         r#""speed":[{"value":80,"#,
    ///         r#""observedAt":"2018-08-01T12:05:00Z"},{"value":100,"#,
    ///         r#""observedAt":"2018-08-01T12:03:00Z"}]}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn absorb_last(&mut self, older: TemporalEntity<Output, C>, last_n: NonZeroU64, time_property: TemporalProperty) {
        self.absorb(older);
        self.attributes.keep_last(last_n, time_property);
    }
}

impl TemporalEntity<Output> {
    /// Drops the members only a broker writes and keeps `@context`: the "read, modify, write back" flow of Create or
    /// Update Temporal Evolution of an Entity, as [`Entity::into_input`](crate::model::Entity::into_input)
    /// is for entities.
    ///
    /// ```
    /// use ngsi_ld::model::{Lossless, Output, evolution::TemporalEntity};
    ///
    /// let read = TemporalEntity::<Output>::parse(
    ///     r#"{
    ///       "id": "urn:ngsi-ld:Vehicle:B9211",
    ///       "type": "Vehicle",
    ///       "speed": [
    ///         {
    ///           "type": "Property",
    ///           "value": 120,
    ///           "observedAt": "2018-08-01T12:03:00Z",
    ///           "instanceId": "urn:ngsi-ld:attribute:instance:1",
    ///           "createdAt": "2018-08-01T12:03:01Z"
    ///         }
    ///       ]
    ///     }"#,
    ///     Lossless::Normalized,
    /// )?;
    /// let input = read.into_input()?;
    /// assert_eq!(
    ///     input.render(Lossless::Concise).to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
    ///         r#""speed":[{"value":120,"#,
    ///         r#""observedAt":"2018-08-01T12:03:00Z"}]}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] listing every instance that holds the NGSI-LD Null, the record of a deleted instance
    /// (clause 4.5.0) a request cannot carry (clause 5.5.4), under its attribute's name, and every removed scope.
    pub fn into_input(self) -> Result<TemporalEntity<Input>, BuildError> {
        let mut problems = Vec::new();
        let attributes = self.attributes.into_input(&mut problems);
        let scope = self.scope.into_iter().filter_map(|instance| instance.into_input(&mut problems)).collect();
        BuildError::finish(
            problems,
            TemporalEntity {
                id: self.id,
                entity_type: self.entity_type,
                scope,
                expires_at: self.expires_at,
                attributes,
                output: (),
                context: self.context,
            },
        )
    }
}

impl<D: EntityDirection, C: Completeness> TemporalEntity<D, C> {
    /// Returns the evolution of the attribute `name`.
    #[must_use]
    pub fn attribute(&self, name: &AttributeName) -> Option<&Evolution<D>> {
        self.attributes.get(name)
    }
}

/// Writes a temporal entity with its own `@context` last.
fn render_temporal<D: WriteEntityDirection, C: Completeness>(entity: &TemporalEntity<D, C>, form: Lossless) -> RawJson {
    let mut object = temporal_entity_object(entity, form);
    write_context(&mut object, entity.context.as_ref());
    RawJson::from_json(Json::Object(object))
}
