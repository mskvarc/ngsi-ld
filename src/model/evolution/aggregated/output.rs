//! Building the aggregated temporal representation of an entity a broker or a context source returns to
//! `format=aggregatedValues` (NGSI-LD clauses 4.5.19 and 6.3.12).

use crate::model::{
    AttributeNameDraft,
    BuildError,
    Context,
    IdDraft,
    TypeDraft,
    evolution::{AggregatedAttribute, AggregatedTemporalEntity, checked_names::checked_attributes},
};
use bon::bon;

#[bon]
impl AggregatedTemporalEntity {
    /// Starts the builder of an entity in aggregated temporal representation as a broker returns it. `id` and
    /// `entity_type` are required, which the compiler checks; each attribute is given once with `attribute`.
    ///
    /// ```
    /// use indexmap::IndexMap;
    /// use iri_rs::iri;
    /// use ngsi_ld::model::{
    ///     Value,
    ///     evolution::{
    ///         AggregatedAttribute,
    ///         AggregatedKind,
    ///         AggregatedTemporalEntity,
    ///         AggregatedValue,
    ///         AggregationMethod,
    ///     },
    /// };
    ///
    /// let max = AggregatedValue {
    ///     value: Value::from(120),
    ///     start: "2018-08-01T12:00:00Z".parse()?,
    ///     end: "2018-08-01T12:04:00Z".parse()?,
    /// };
    /// let speed = AggregatedAttribute {
    ///     kind: AggregatedKind::Property,
    ///     methods: IndexMap::from([(AggregationMethod::Max, vec![max])]),
    /// };
    /// let vehicle = AggregatedTemporalEntity::builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
    ///     .entity_type("Vehicle")
    ///     .attribute("speed", speed)
    ///     .build()?;
    /// assert_eq!(
    ///     vehicle.render().to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
    ///         r#""speed":{"type":"Property","max":[[120,"#,
    ///         r#""2018-08-01T12:00:00Z","2018-08-01T12:04:00Z"]]}}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid identifier, type and attribute name, every attribute
    /// given twice, and every `location`, `observationSpace` or `operationSpace`: clause 5.2.4 makes them
    /// `GeoProperty` attributes, which clause 4.5.19 does not aggregate (`GeoPropertyRequired`).
    #[builder]
    pub fn new(
        #[builder(field)] attributes: Vec<(AttributeNameDraft, AggregatedAttribute)>,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        #[builder(into)] context: Option<Context>,
    ) -> Result<AggregatedTemporalEntity, BuildError> {
        let mut problems = Vec::new();
        let id = id.resolve(&mut problems);
        let entity_type = entity_type.resolve(&mut problems);
        // Clause 4.5.19 aggregates Properties and Relationships, never a `GeoProperty`, so `location`,
        // `observationSpace` and `operationSpace`, which clause 5.2.4 reserves for one, are refused.
        let attributes = checked_attributes(attributes, |attribute| attribute.kind.attribute_kind(), &mut problems);
        // An unresolved member has recorded its problem, so the error is never empty.
        if let (Some(id), Some(entity_type)) = (id, entity_type) {
            BuildError::finish(
                problems,
                AggregatedTemporalEntity {
                    id,
                    entity_type,
                    attributes,
                    context,
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }
}

impl<S: aggregated_temporal_entity_builder::State> AggregatedTemporalEntityBuilder<S> {
    /// Gives the aggregated values of the attribute `name` (clause 4.5.19).
    pub fn attribute(mut self, name: impl Into<AttributeNameDraft>, attribute: AggregatedAttribute) -> Self {
        self.attributes.push((name.into(), attribute));
        self
    }
}
