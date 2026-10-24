//! Properties (NGSI-LD clauses 4.5.2 and 5.2.5).

use crate::model::{
    AttributeDraft,
    AttributeMeta,
    AttributeNameDraft,
    BuildError,
    DatasetId,
    DateTime,
    Direction,
    Input,
    LanguageKey,
    MemberChange,
    NullMember,
    Output,
    Patch,
    Plain,
    Proof,
    Value,
    ValuePatch,
    VocabTerm,
    attribute::pending::PendingAttributes,
    null,
};
use bon::bon;
use cefact_units::UnitCode;

/// A Property: a value of an entity or attribute (NGSI-LD clauses 4.5.2 and 5.2.5).
///
/// Build one with [`Property::builder`]; read one from a document through the representation parsers. Its members
/// are read through getters, so a built or read Property keeps the rules it was checked against.
///
/// ```
/// use cefact_units::UnitCode;
/// use ngsi_ld::model::{Property, Value};
///
/// let speed = Property::builder(80).unit_code(UnitCode::Kmh).build()?;
/// assert_eq!(speed.value(), &Value::from(80));
/// assert_eq!(speed.unit_code(), Some(&UnitCode::Kmh));
/// assert!(Property::builder("urn:ngsi-ld:null").build().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property<D: Direction> {
    /// `value`: an NGSI-LD Value (clause 3.1).
    pub(crate) value: Value,
    /// `unitCode`: a UN/CEFACT common code (clause 4.5.2.2).
    pub(crate) unit_code: D::Member<UnitCode>,
    /// `valueType` (clause 4.5.2.2).
    pub(crate) value_type: D::Member<VocabTerm>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<PropertyOutputMembers>,
}

#[bon]
impl Property<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, and a
    /// value that is, or holds as a member, the NGSI-LD Null (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] value: Value,
        #[builder(field)] attributes: PendingAttributes,
        unit_code: Option<UnitCode>,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        null::report(null::check_value(&value), &mut problems);
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        BuildError::finish(
            problems,
            Self {
                value,
                unit_code,
                value_type,
                meta,
                output: (),
            },
        )
    }
}

#[bon]
impl Property<Patch> {
    /// Starts the builder of a Property fragment whose `value` is merged into the target's (clause 5.5.12): the
    /// members `patch` sets or merges into are changed, those it deletes are written as the NGSI-LD Null and removed,
    /// and the others are kept. Merge Entity and Batch Entity Merge merge it so (clauses 5.6.17 and 5.6.20); Partial
    /// Attribute Update and Update Attributes replace the whole value (clause 5.5.8), so they send it as written. The
    /// optional members of the kind and of every attribute have setters; the fragment is turned into an
    /// [`Attribute`](crate::model::Attribute), which deletes members with `delete_member` and
    /// sub-attributes with `delete_sub_attribute`, and is added to a fragment with `patch`.
    ///
    /// ```
    /// use cefact_units::UnitCode;
    /// use ngsi_ld::model::{Attribute, EntityFragment, Lossless, Property, ValuePatch};
    ///
    /// let settings = ValuePatch::new().merge("display", ValuePatch::new().delete("brightness"));
    /// let config = Property::merge_builder(settings)
    ///     .unit_code(UnitCode::C62)
    ///     .build()?;
    /// let fragment = EntityFragment::builder()
    ///     .patch("config", Attribute::from(config))
    ///     .build()?;
    /// assert_eq!(
    ///     fragment.render(Lossless::Concise).to_string(),
    ///     r#"{"config":{"value":{"display":{"brightness":"urn:ngsi-ld:null"}},"unitCode":"C62"}}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] when a value the patch sets is, or holds as a member, the NGSI-LD Null, which
    /// only a deletion writes.
    #[builder(start_fn = merge_builder, finish_fn = build)]
    pub fn merge(
        #[builder(start_fn)] patch: ValuePatch,
        unit_code: Option<UnitCode>,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        for value in patch.set_values() {
            null::report(null::check_value(value).map_err(|_| NullMember::ValueMember), &mut problems);
        }
        let meta = AttributeMeta {
            dataset_id,
            observed_at: Patch::set_member(observed_at),
            expires_at: Patch::set_member(expires_at),
            ..AttributeMeta::empty()
        };
        BuildError::finish(
            problems,
            Self {
                value: patch.into_value(),
                unit_code: Patch::set_member(unit_code),
                value_type: Patch::set_member(value_type),
                meta,
                output: (),
            },
        )
    }
}

impl<D: Direction> Property<D> {
    /// A Property with nothing but its value: the bare concise form (clause 4.5.2.3), and the value of an
    /// `ngsildproof`.
    pub(crate) fn plain(value: Value) -> Self {
        Self {
            value,
            unit_code: Default::default(),
            value_type: Default::default(),
            meta: AttributeMeta::empty(),
            output: Default::default(),
        }
    }

    /// Returns the value (`value`).
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> Property<D> {
    /// Returns the unit of the value (`unitCode`).
    #[must_use]
    pub fn unit_code(&self) -> Option<&UnitCode> {
        D::member(&self.unit_code)
    }

    /// Returns the datatype of the value (`valueType`).
    #[must_use]
    pub fn value_type(&self) -> Option<&VocabTerm> {
        D::member(&self.value_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> Property<E> {
        Property {
            value: self.value,
            unit_code: E::set_member(D::into_member(self.unit_code)),
            value_type: E::set_member(D::into_member(self.value_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl Property<Patch> {
    /// Returns the change the fragment makes to the unit (`unitCode`): set, deleted with the NGSI-LD Null, or absent.
    #[must_use]
    pub fn unit_code(&self) -> Option<MemberChange<&UnitCode>> {
        self.unit_code.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change the fragment makes to the datatype of the value (`valueType`): set, deleted with the NGSI-LD
    /// Null, or absent.
    #[must_use]
    pub fn value_type(&self) -> Option<MemberChange<&VocabTerm>> {
        self.value_type.as_ref().map(MemberChange::as_ref)
    }
}

impl Property<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &PropertyOutputMembers {
        &self.output
    }
}

/// The members a Property has only in a response (NGSI-LD clauses 4.5.2.2, 4.15 and Table 5.2.5-2).
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PropertyOutputMembers {
    /// `previousValue`: the value before the change that triggered a notification, sent only when the
    /// subscription asks for `showChanges` (clause 4.5.2.2). It is written like `value`, so it may be the
    /// NGSI-LD Null (clause 4.5.0).
    pub previous_value: Option<Value>,
    /// `lang`: the language the language filter picked when it returned a `LanguageProperty` as this Property
    /// (clause 4.15).
    pub lang: Option<LanguageKey>,
}

attribute_methods!(PropertyBuilder, property_builder::State, attributes);

impl AttributeDraft<Property<Input>> for Property<Input> {
    fn into_attribute(self) -> Result<Property<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: property_builder::IsComplete> AttributeDraft<Property<Input>> for PropertyBuilder<S> {
    fn into_attribute(self) -> Result<Property<Input>, BuildError> {
        self.build()
    }
}

/// A value is a Property with nothing but that value.
impl<V: Into<Value>> AttributeDraft<Property<Input>> for V {
    fn into_attribute(self) -> Result<Property<Input>, BuildError> {
        Property::builder(self).build()
    }
}
