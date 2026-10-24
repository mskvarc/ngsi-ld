//! Building the temporal evolution of an entity a broker or a context source returns to Retrieve or Query Temporal
//! Evolution (NGSI-LD clauses 4.5.6 to 4.5.8, 5.2.20, 5.7.3 and 5.7.4): instances that may carry `instanceId`, the
//! system members and, for a deleted instance, the NGSI-LD Null with `deletedAt`.

use crate::model::{
    Attribute,
    AttributeDraft,
    AttributeNameDraft,
    BuildError,
    Context,
    DateTime,
    EntityOutputMembers,
    IdDraft,
    Output,
    TypeDraft,
    attribute::pending::PendingAttributes,
    evolution::{
        ScopeInstance,
        TemporalEntity,
        temporal_entity::{TemporalDrafts, assemble},
    },
};
use bon::bon;

#[bon]
impl TemporalEntity<Output> {
    /// Starts the builder of a temporal entity as a broker or a context source returns it. `id` and `entity_type`
    /// are required, which the compiler checks. Instances are added with the per-kind methods (`property`,
    /// `relationship`, ...), which take what the request builder takes, or with
    /// [`output_attribute`](TemporalEntityOutputBuilder::output_attribute), which takes a response instance with its
    /// output-only members (`instanceId`, the system members) or a deleted instance built with
    /// [`Attribute::deleted`]. Naming an attribute again adds another instance to its evolution.
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::{
    ///     Attribute,
    ///     AttributeKind,
    ///     InstanceId,
    ///     Lossless,
    ///     Property,
    ///     evolution::TemporalEntity,
    /// };
    ///
    /// let vehicle = TemporalEntity::output_builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
    ///     .entity_type("Vehicle")
    ///     .output_attribute(
    ///         "speed",
    ///         Property::output_builder(
    ///             Property::builder(120).observed_at("2018-08-01T12:03:00Z".parse()?),
    ///         )
    ///         .instance_id("urn:ngsi-ld:attribute:instance:1".parse::<InstanceId>()?),
    ///     )
    ///     .output_attribute(
    ///         "speed",
    ///         Attribute::deleted(AttributeKind::Property).deleted_at("2018-08-01T12:05:00Z".parse()?),
    ///     )
    ///     .build()?;
    /// assert_eq!(
    ///     vehicle.render(Lossless::Concise).to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
    ///         r#""speed":[{"value":120,"#,
    ///         r#""observedAt":"2018-08-01T12:03:00Z","#,
    ///         r#""instanceId":"urn:ngsi-ld:attribute:instance:1"},"#,
    ///         r#"{"value":"urn:ngsi-ld:null","#,
    ///         r#""deletedAt":"2018-08-01T12:05:00Z"}]}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every problem the request builder reports and every instance of
    /// another kind than the instances before it.
    #[builder(builder_type = TemporalEntityOutputBuilder, finish_fn = build)]
    pub fn output_builder(
        #[builder(field)] attributes: PendingAttributes<Output>,
        #[builder(field)] scope: Vec<ScopeInstance<Output>>,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        expires_at: Option<DateTime>,
        created_at: Option<DateTime>,
        modified_at: Option<DateTime>,
        deleted_at: Option<DateTime>,
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
            EntityOutputMembers {
                created_at,
                modified_at,
                deleted_at,
            },
        )
    }
}

attribute_methods!(TemporalEntityOutputBuilder, temporal_entity_output_builder::State, attributes);

impl<S: temporal_entity_output_builder::State> TemporalEntityOutputBuilder<S> {
    /// Adds a response instance of any kind to the evolution of `name`, after the ones already given: an attribute or
    /// attribute builder of the [`Output`] direction, which may carry output-only members.
    pub fn output_attribute<T: Into<Attribute<Output>>>(mut self, name: impl Into<AttributeNameDraft>, attribute: impl AttributeDraft<T>) -> Self {
        self.attributes.push_output(name, attribute.into_attribute().map(Into::into));
        self
    }

    /// Adds an instance to the evolution of the scope (clause 4.5.6), after the ones already given.
    pub fn scope(mut self, instance: ScopeInstance<Output>) -> Self {
        self.scope.push(instance);
        self
    }
}
