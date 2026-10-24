//! Building the entity a broker or a context source returns (NGSI-LD clauses 5.2.2 and 5.2.4): the members a
//! request has, the system members of the entity, and attributes that may carry their own output-only members.

use crate::model::{
    Attribute,
    AttributeDraft,
    AttributeNameDraft,
    BuildError,
    Context,
    DateTime,
    Entity,
    EntityOutputMembers,
    IdDraft,
    Output,
    Projected,
    ScopeDraft,
    TypeDraft,
    attribute::pending::PendingAttributes,
    entity::{EntityDrafts, assemble},
};
use bon::bon;

#[bon]
impl Entity<Output> {
    /// Starts the builder of an entity as a broker or a context source returns it: what a request entity holds, plus
    /// the system members `createdAt`, `modifiedAt` and `deletedAt` (clause 5.2.2). `id` and `entity_type` are
    /// required, which the compiler checks. Attributes are added with the per-kind methods (`property`, `relationship`,
    /// ...), which take what the request builder takes and add no output-only member, or with
    /// [`output_attribute`](EntityOutputBuilder::output_attribute), which takes a response attribute built with its own
    /// output-only members, such as a
    /// [`Property::output_builder`](crate::model::Property::output_builder).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::{Entity, Property, Representation};
    ///
    /// let created = "2024-05-01T10:00:00Z".parse()?;
    /// let vehicle = Entity::output_builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
    ///     .entity_type("Vehicle")
    ///     .property("brandName", "Mercedes")
    ///     .output_attribute(
    ///         "speed",
    ///         Property::output_builder(80)
    ///             .created_at(created)
    ///             .previous_value(70),
    ///     )
    ///     .created_at(created)
    ///     .build()?;
    /// assert_eq!(
    ///     vehicle.render(Representation::Concise).to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","#,
    ///         r#""brandName":"Mercedes","speed":{"value":80,"#,
    ///         r#""previousValue":70,"createdAt":"2024-05-01T10:00:00Z"},"#,
    ///         r#""createdAt":"2024-05-01T10:00:00Z"}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every problem the request builder reports.
    #[builder(builder_type = EntityOutputBuilder, finish_fn = build)]
    pub fn output_builder(
        #[builder(field)] attributes: PendingAttributes<Output>,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        #[builder(into)] scope: Option<ScopeDraft>,
        expires_at: Option<DateTime>,
        created_at: Option<DateTime>,
        modified_at: Option<DateTime>,
        deleted_at: Option<DateTime>,
        #[builder(into)] context: Option<Context>,
    ) -> Result<Self, BuildError> {
        assemble(
            EntityDrafts {
                id: Some(id),
                entity_type: Some(entity_type),
                scope,
                expires_at,
                context,
            },
            attributes,
            EntityOutputMembers {
                created_at,
                modified_at,
                deleted_at,
            },
        )
    }
}

attribute_methods!(EntityOutputBuilder, entity_output_builder::State, attributes);

impl<S: entity_output_builder::State> EntityOutputBuilder<S> {
    /// Adds a response attribute of any kind: an attribute or attribute builder of the [`Output`] direction, which
    /// may carry output-only members. Naming an attribute a second time adds another instance (clause 4.5.5).
    pub fn output_attribute<T: Into<Attribute<Output>>>(mut self, name: impl Into<AttributeNameDraft>, attribute: impl AttributeDraft<T>) -> Self {
        self.attributes.push_output(name, attribute.into_attribute().map(Into::into));
        self
    }
}

#[bon]
impl Entity<Output, Projected> {
    /// Starts the builder of an entity a projection reduced (clause 4.21), as a broker or a context source answers a
    /// request whose `pick` leaves out `id` or `type`, or whose `omit` names them (Table 6.4.3.2-1), and as an inline
    /// linked entity is projected (clause 4.5.23.2). It takes what
    /// [`Entity::output_builder`](Entity::output_builder) takes, but `id` and `entity_type` are optional.
    ///
    /// ```
    /// use ngsi_ld::model::{Entity, Lossless, Output, Projected, Representation};
    ///
    /// let speed = Entity::projected_output_builder()
    ///     .property("speed", 80)
    ///     .build()?;
    /// assert_eq!(
    ///     speed.render(Representation::Concise).to_string(),
    ///     r#"{"speed":80}"#
    /// );
    /// assert_eq!(
    ///     Entity::<Output, Projected>::parse(r#"{"speed":80}"#, Lossless::Concise).ok(),
    ///     Some(speed)
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every problem the request builder reports.
    #[builder(builder_type = ProjectedEntityOutputBuilder, finish_fn = build)]
    pub fn projected_output_builder(
        #[builder(field)] attributes: PendingAttributes<Output>,
        #[builder(into)] id: Option<IdDraft>,
        #[builder(into)] entity_type: Option<TypeDraft>,
        #[builder(into)] scope: Option<ScopeDraft>,
        expires_at: Option<DateTime>,
        created_at: Option<DateTime>,
        modified_at: Option<DateTime>,
        deleted_at: Option<DateTime>,
        #[builder(into)] context: Option<Context>,
    ) -> Result<Self, BuildError> {
        assemble(
            EntityDrafts {
                id,
                entity_type,
                scope,
                expires_at,
                context,
            },
            attributes,
            EntityOutputMembers {
                created_at,
                modified_at,
                deleted_at,
            },
        )
    }
}

attribute_methods!(ProjectedEntityOutputBuilder, projected_entity_output_builder::State, attributes);

impl<S: projected_entity_output_builder::State> ProjectedEntityOutputBuilder<S> {
    /// Adds a response attribute of any kind, as [`EntityOutputBuilder::output_attribute`] does.
    pub fn output_attribute<T: Into<Attribute<Output>>>(mut self, name: impl Into<AttributeNameDraft>, attribute: impl AttributeDraft<T>) -> Self {
        self.attributes.push_output(name, attribute.into_attribute().map(Into::into));
        self
    }
}
