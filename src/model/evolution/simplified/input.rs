//! The simplified temporal representation of an entity as a client sends it (NGSI-LD clauses 5.2.20 and 5.6.11.3).

use crate::model::{
    AttributeNameDraft,
    BuildError,
    BuildProblem,
    Context,
    IdDraft,
    OneOrMany,
    ProblemKind,
    RawJson,
    Scope,
    TypeDraft,
    evolution::{ScopeValue, SimplifiedTemporalEntity, TemporalValue, TemporalValues, checked_names::checked_attributes},
    null::check_temporal_values,
};
use bon::bon;

/// The simplified temporal evolution of an entity, as a client sends it.
///
/// The client sends it in simplified temporal representation to Create or Update Temporal Evolution of an Entity
/// (NGSI-LD clauses 5.2.20 and 5.6.11.3). It holds `id`, `type`, the scope's values over time, and each attribute's
/// [`TemporalValues`].
///
/// Clause 5.2.20 allows this representation as an alternative to the normalized one
/// ([`TemporalEntity`](crate::model::evolution::TemporalEntity)), which carries everything it can and more. Unlike
/// what a broker answers ([`SimplifiedTemporalEntity`]), it holds no NGSI-LD Null (clause 5.5.4): no data point of an
/// attribute and no scope is the Null of a deleted instance.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{
///     Value,
///     evolution::{SimplifiedTemporalInput, TemporalValue, TemporalValues},
/// };
///
/// let speed = TemporalValues::Property(vec![
///     TemporalValue {
///         value: Value::from(120),
///         time: "2018-08-01T12:03:00Z".parse()?,
///     },
///     TemporalValue {
///         value: Value::from(80),
///         time: "2018-08-01T12:05:00Z".parse()?,
///     },
/// ]);
/// let vehicle = SimplifiedTemporalInput::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///     .entity_type("Vehicle")
///     .values("speed", speed)
///     .build()?;
/// assert_eq!(
///     vehicle.render().to_string(),
///     concat!(
///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
///         r#""speed":{"type":"Property","values":[[120,"#,
///         r#""2018-08-01T12:03:00Z"],[80,"2018-08-01T12:05:00Z"]]}}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimplifiedTemporalInput {
    /// The entity, checked to hold no NGSI-LD Null.
    entity: SimplifiedTemporalEntity,
}

#[bon]
impl SimplifiedTemporalInput {
    /// Starts a builder. `id` and `entity_type` are required, which the compiler checks; each attribute's data points
    /// are given once with `values`, and the scope's with `scope`, one data point a call. Every problem is reported by
    /// `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid identifier, type and attribute name, every attribute
    /// given twice, every `location`, `observationSpace` or `operationSpace` whose values are not a `GeoProperty`'s
    /// (clause 5.2.4), and every data point holding the NGSI-LD Null.
    #[builder]
    pub fn new(
        #[builder(field)] attributes: Vec<(AttributeNameDraft, TemporalValues)>,
        #[builder(field)] scope: Vec<TemporalValue<OneOrMany<Scope>>>,
        #[builder(into)] id: IdDraft,
        #[builder(into)] entity_type: TypeDraft,
        #[builder(into)] context: Option<Context>,
    ) -> Result<SimplifiedTemporalInput, BuildError> {
        let mut problems = Vec::new();
        let id = id.resolve(&mut problems);
        let entity_type = entity_type.resolve(&mut problems);
        let attributes = checked_attributes(attributes, TemporalValues::kind, &mut problems);
        for (name, values) in &attributes {
            if let Err(member) = check_temporal_values(values) {
                problems.push(BuildProblem::of(name, ProblemKind::NgsiLdNull { member }));
            }
        }
        let scope = scope
            .into_iter()
            .map(|point| TemporalValue {
                value: ScopeValue::Scopes(point.value),
                time: point.time,
            })
            .collect();
        // An unresolved member has recorded its problem, so the error is never empty.
        if let (Some(id), Some(entity_type)) = (id, entity_type) {
            BuildError::finish(
                problems,
                SimplifiedTemporalInput {
                    entity: SimplifiedTemporalEntity {
                        id,
                        entity_type,
                        scope,
                        attributes,
                        context,
                    },
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }

    /// The entity, as the simplified temporal representation the broker answers with models it.
    #[must_use]
    pub const fn as_entity(&self) -> &SimplifiedTemporalEntity {
        &self.entity
    }

    /// Writes the entity in simplified temporal representation, with its own `@context` last when it has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        self.entity.render()
    }

    /// The members without the `@context`: the body of Create or Update Temporal Evolution (clause 6.3.5).
    #[cfg(feature = "http")]
    pub(crate) fn to_object(&self) -> jstrict::Object {
        self.entity.object()
    }
}

impl<S: simplified_temporal_input_builder::State> SimplifiedTemporalInputBuilder<S> {
    /// Gives the data points of the attribute `name`, all of its kind (clause 4.5.9).
    pub fn values(mut self, name: impl Into<AttributeNameDraft>, values: TemporalValues) -> Self {
        self.attributes.push((name.into(), values));
        self
    }

    /// Adds a data point to the values of the scope (Annex C.5.16), after the ones already given.
    pub fn scope(mut self, point: TemporalValue<OneOrMany<Scope>>) -> Self {
        self.scope.push(point);
        self
    }
}
