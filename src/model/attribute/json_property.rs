//! JSON Properties (NGSI-LD clauses 4.5.24 and 5.2.38).

use crate::model::{
    AttributeDraft,
    AttributeMeta,
    AttributeNameDraft,
    BuildError,
    BuildProblem,
    DatasetId,
    DateTime,
    Direction,
    Input,
    MemberChange,
    Output,
    Patch,
    Plain,
    ProblemKind,
    Proof,
    RawJson,
    VocabTerm,
    attribute::pending::PendingAttributes,
};
use bon::bon;

/// A `JsonProperty`: raw JSON that is never expanded as JSON-LD (NGSI-LD clauses 4.5.24 and 5.2.38).
///
/// Build one with [`JsonProperty::builder`]; read one from a document through the representation parsers. Its
/// `json` is always an object or an array of objects.
///
/// ```
/// use ngsi_ld::model::{JsonProperty, RawJson};
///
/// let ticket =
///     JsonProperty::builder(r#"{"id": "85a6cc52-0589-45f9"}"#.parse::<RawJson>()?).build()?;
/// assert!(ticket.json().is_object());
/// assert!(
///     JsonProperty::builder("[1, 2]".parse::<RawJson>()?)
///         .build()
///         .is_err()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonProperty<D: Direction> {
    /// `json`: an object or an array of objects (clause 4.5.24.2).
    pub(crate) json: RawJson,
    /// `valueType` (Table 5.2.38-1).
    pub(crate) value_type: D::Member<VocabTerm>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<JsonPropertyOutputMembers>,
}

#[bon]
impl JsonProperty<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, and JSON
    /// that is neither an object nor an array of objects.
    #[builder]
    pub fn new(
        #[builder(start_fn)] json: RawJson,
        #[builder(field)] attributes: PendingAttributes,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        if !json.is_json_property_payload() {
            problems.push(BuildProblem::top(ProblemKind::JsonShape));
        }
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        BuildError::finish(
            problems,
            Self {
                json,
                value_type,
                meta,
                output: (),
            },
        )
    }
}

impl<D: Direction> JsonProperty<D> {
    /// Returns the raw JSON (`json`).
    #[must_use]
    pub const fn json(&self) -> &RawJson {
        &self.json
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> JsonProperty<D> {
    /// Returns the datatype (`valueType`).
    #[must_use]
    pub fn value_type(&self) -> Option<&VocabTerm> {
        D::member(&self.value_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> JsonProperty<E> {
        JsonProperty {
            json: self.json,
            value_type: E::set_member(D::into_member(self.value_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl JsonProperty<Patch> {
    /// Returns the change the fragment makes to the datatype of the value (`valueType`): set, deleted with the NGSI-LD
    /// Null, or absent.
    #[must_use]
    pub fn value_type(&self) -> Option<MemberChange<&VocabTerm>> {
        self.value_type.as_ref().map(MemberChange::as_ref)
    }
}

impl JsonProperty<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &JsonPropertyOutputMembers {
        &self.output
    }
}

/// The members a `JsonProperty` has only in a response (NGSI-LD clauses 4.5.24.2 and Table 5.2.38-2).
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct JsonPropertyOutputMembers {
    /// `previousJson`: the JSON before the change that triggered a notification, sent only when the subscription
    /// asks for `showChanges` (clause 4.5.24.2). It is written like `json`, verbatim.
    pub previous_json: Option<RawJson>,
}

attribute_methods!(JsonPropertyBuilder, json_property_builder::State, attributes);

impl AttributeDraft<JsonProperty<Input>> for JsonProperty<Input> {
    fn into_attribute(self) -> Result<JsonProperty<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: json_property_builder::IsComplete> AttributeDraft<JsonProperty<Input>> for JsonPropertyBuilder<S> {
    fn into_attribute(self) -> Result<JsonProperty<Input>, BuildError> {
        self.build()
    }
}

/// Raw JSON is a `JsonProperty` with nothing but that JSON.
impl AttributeDraft<JsonProperty<Input>> for RawJson {
    fn into_attribute(self) -> Result<JsonProperty<Input>, BuildError> {
        JsonProperty::builder(self).build()
    }
}

/// JSON text, parsed when the document is built.
impl AttributeDraft<JsonProperty<Input>> for &str {
    fn into_attribute(self) -> Result<JsonProperty<Input>, BuildError> {
        JsonProperty::builder(self.parse::<RawJson>()?).build()
    }
}
