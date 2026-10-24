//! Building the simplified temporal representation of an entity a broker or a context source returns to
//! `format=temporalValues` (NGSI-LD clauses 4.5.9 and 6.3.12), where a data point may be the NGSI-LD Null of a
//! deleted instance.

use crate::model::{
    AttributeNameDraft,
    BuildError,
    Context,
    IdDraft,
    TypeDraft,
    evolution::{ScopeValue, SimplifiedTemporalEntity, TemporalValue, TemporalValues, checked_names::checked_attributes},
};
use bon::bon;

#[bon]
impl SimplifiedTemporalEntity {
    /// Starts the builder of an entity in simplified temporal representation as a broker returns it. `id` and
    /// `entity_type` are required, which the compiler checks; each attribute's data points are given once with
    /// `values`, the scope's with `scope`, one data point a call. Unlike a
    /// [`SimplifiedTemporalInput`](crate::model::evolution::SimplifiedTemporalInput), a data point
    /// may be the NGSI-LD Null of a deleted instance or a removed scope (clause 4.5.0).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::{
    ///     Value,
    ///     evolution::{SimplifiedTemporalEntity, TemporalValue, TemporalValues},
    /// };
    ///
    /// let speed = TemporalValues::Property(vec![
    ///     TemporalValue {
    ///         value: Value::from(120),
    ///         time: "2018-08-01T12:03:00Z".parse()?,
    ///     },
    ///     TemporalValue {
    ///         value: Value::from("urn:ngsi-ld:null"),
    ///         time: "2018-08-01T12:05:00Z".parse()?,
    ///     },
    /// ]);
    /// let vehicle = SimplifiedTemporalEntity::builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
    ///     .entity_type("Vehicle")
    ///     .values("speed", speed)
    ///     .build()?;
    /// assert_eq!(
    ///     vehicle.render().to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
    ///         r#""speed":{"type":"Property","values":[[120,"#,
    ///         r#""2018-08-01T12:03:00Z"],["urn:ngsi-ld:null","#,
    ///         r#""2018-08-01T12:05:00Z"]]}}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid identifier, type and attribute name, every attribute
    /// given twice, and every `location`, `observationSpace` or `operationSpace` whose values are not a
    /// `GeoProperty`'s (clause 5.2.4).
    #[builder]
    pub fn new(
        #[builder(field)] attributes: Vec<(AttributeNameDraft, TemporalValues)>,
        #[builder(field)] scope: Vec<TemporalValue<ScopeValue>>,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        #[builder(into)] context: Option<Context>,
    ) -> Result<SimplifiedTemporalEntity, BuildError> {
        let mut problems = Vec::new();
        let id = id.resolve(&mut problems);
        let entity_type = entity_type.resolve(&mut problems);
        let attributes = checked_attributes(attributes, TemporalValues::kind, &mut problems);
        // An unresolved member has recorded its problem, so the error is never empty.
        if let (Some(id), Some(entity_type)) = (id, entity_type) {
            BuildError::finish(
                problems,
                SimplifiedTemporalEntity {
                    id,
                    entity_type,
                    scope,
                    attributes,
                    context,
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }
}

impl<S: simplified_temporal_entity_builder::State> SimplifiedTemporalEntityBuilder<S> {
    /// Gives the data points of the attribute `name`, all of its kind (clause 4.5.9).
    pub fn values(mut self, name: impl Into<AttributeNameDraft>, values: TemporalValues) -> Self {
        self.attributes.push((name.into(), values));
        self
    }

    /// Adds a data point to the values of the scope (Annex C.5.16), after the ones already given; its value may be
    /// [`ScopeValue::Removed`].
    pub fn scope(mut self, point: TemporalValue<ScopeValue>) -> Self {
        self.scope.push(point);
        self
    }
}
