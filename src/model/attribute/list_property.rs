//! List Properties (NGSI-LD clauses 4.5.21 and 5.2.36).

use crate::{
    model::{
        AttributeDraft,
        AttributeMeta,
        AttributeNameDraft,
        BuildError,
        DatasetId,
        DateTime,
        Direction,
        Input,
        MemberChange,
        Output,
        Patch,
        Plain,
        Proof,
        Value,
        VocabTerm,
        attribute::pending::PendingAttributes,
        null,
    },
    seal::Seal,
};
use bon::bon;
use cefact_units::UnitCode;

/// A `ListProperty`: an ordered list of values.
///
/// NGSI-LD clauses 4.5.21 and 5.2.36.
///
/// Build one with [`ListProperty::builder`]; read one from a document through the representation parsers.
///
/// ```
/// use cefact_units::UnitCode;
/// use ngsi_ld::model::{ListProperty, Value};
///
/// let depths = ListProperty::builder(vec![Value::from(300), Value::from(120)])
///     .unit_code(UnitCode::Mmt)
///     .build()?;
/// assert_eq!(depths.value_list().len(), 2);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListProperty<D: Direction> {
    /// `valueList`: the ordered values.
    pub(crate) value_list: Vec<Value>,
    /// `unitCode`, which clause 4.5.21.2 carries over from Property.
    pub(crate) unit_code: D::Member<UnitCode>,
    /// `valueType` (Table 5.2.36-1).
    pub(crate) value_type: D::Member<VocabTerm>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<ListPropertyOutputMembers>,
}

#[bon]
impl ListProperty<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, and a
    /// `valueList` that is the NGSI-LD Null alone (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn)] value_list: Vec<Value>,
        #[builder(field)] attributes: PendingAttributes,
        unit_code: Option<UnitCode>,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        null::report(null::check_value_list(&value_list), &mut problems);
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        BuildError::finish(
            problems,
            Self {
                value_list,
                unit_code,
                value_type,
                meta,
                output: (),
            },
        )
    }
}

impl<D: Direction> ListProperty<D> {
    /// Returns the ordered values (`valueList`).
    #[must_use]
    pub fn value_list(&self) -> &[Value] {
        &self.value_list
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> ListProperty<D> {
    /// Returns the unit (`unitCode`).
    #[must_use]
    pub fn unit_code(&self) -> Option<&UnitCode> {
        D::member(Seal::new(), &self.unit_code)
    }

    /// Returns the datatype of the values (`valueType`).
    #[must_use]
    pub fn value_type(&self) -> Option<&VocabTerm> {
        D::member(Seal::new(), &self.value_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> ListProperty<E> {
        ListProperty {
            value_list: self.value_list,
            unit_code: E::set_member(Seal::new(), D::into_member(Seal::new(), self.unit_code)),
            value_type: E::set_member(Seal::new(), D::into_member(Seal::new(), self.value_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl ListProperty<Patch> {
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

impl ListProperty<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &ListPropertyOutputMembers {
        &self.output
    }
}

/// The members a `ListProperty` has only in a response.
///
/// NGSI-LD clauses 4.5.21.2 and Table 5.2.36-2.
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ListPropertyOutputMembers {
    /// `previousValueList`: the list before the change that triggered a notification, sent only when the
    /// subscription asks for `showChanges` (clause 4.5.21.2). It is written like `valueList`.
    pub previous_value_list: Option<Vec<Value>>,
}

attribute_methods!(ListPropertyBuilder, list_property_builder::State, attributes);

impl AttributeDraft<ListProperty<Input>> for ListProperty<Input> {
    fn into_attribute(self) -> Result<ListProperty<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: list_property_builder::IsComplete> AttributeDraft<ListProperty<Input>> for ListPropertyBuilder<S> {
    fn into_attribute(self) -> Result<ListProperty<Input>, BuildError> {
        self.build()
    }
}

/// A list of values is a `ListProperty` with nothing but that list.
impl<V: Into<Value>> AttributeDraft<ListProperty<Input>> for Vec<V> {
    fn into_attribute(self) -> Result<ListProperty<Input>, BuildError> {
        ListProperty::builder(self.into_iter().map(Into::into).collect()).build()
    }
}

/// An array of values is a `ListProperty` with nothing but those values.
impl<V: Into<Value>, const N: usize> AttributeDraft<ListProperty<Input>> for [V; N] {
    fn into_attribute(self) -> Result<ListProperty<Input>, BuildError> {
        ListProperty::builder(self.into_iter().map(Into::into).collect()).build()
    }
}
