//! Language Properties (NGSI-LD clauses 4.5.18 and 5.2.32).

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
        LanguageMap,
        LanguageText,
        MemberChange,
        Output,
        Patch,
        Plain,
        Proof,
        VocabTerm,
        attribute::pending::PendingAttributes,
        null,
    },
    seal::Seal,
};
use bon::bon;

/// A `LanguageProperty`: text in several natural languages (NGSI-LD clauses 4.5.18 and 5.2.32).
///
/// Build one with [`LanguageProperty::builder`]; read one from a document through the representation parsers.
///
/// ```
/// use ngsi_ld::model::{LanguageMap, LanguageProperty};
///
/// let street = LanguageProperty::builder(LanguageMap::try_from_pairs([
///     ("fr", "Grand Place"),
///     ("nl", "Grote Markt"),
/// ])?)
/// .build()?;
/// assert_eq!(street.language_map().iter().count(), 2);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProperty<D: Direction> {
    /// `languageMap`: the texts by language (clause 4.6.5).
    pub(crate) language_map: LanguageMap,
    /// `valueType`; Table 5.2.32-1 fixes it to `langString` when present.
    pub(crate) value_type: D::Member<VocabTerm>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<LanguagePropertyOutputMembers>,
}

#[bon]
impl LanguageProperty<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, and a
    /// language map whose `@none` text is the NGSI-LD Null (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn)] language_map: LanguageMap,
        #[builder(field)] attributes: PendingAttributes,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        null::report(null::check_language_map(&language_map), &mut problems);
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        BuildError::finish(
            problems,
            Self {
                language_map,
                value_type,
                meta,
                output: (),
            },
        )
    }
}

impl<D: Direction> LanguageProperty<D> {
    /// Returns the texts by language (`languageMap`).
    #[must_use]
    pub const fn language_map(&self) -> &LanguageMap {
        &self.language_map
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> LanguageProperty<D> {
    /// Returns the datatype (`valueType`).
    #[must_use]
    pub fn value_type(&self) -> Option<&VocabTerm> {
        D::member(Seal::new(), &self.value_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> LanguageProperty<E> {
        LanguageProperty {
            language_map: self.language_map,
            value_type: E::set_member(Seal::new(), D::into_member(Seal::new(), self.value_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl LanguageProperty<Patch> {
    /// Returns the change the fragment makes to the datatype of the value (`valueType`): set, deleted with the NGSI-LD
    /// Null, or absent.
    #[must_use]
    pub fn value_type(&self) -> Option<MemberChange<&VocabTerm>> {
        self.value_type.as_ref().map(MemberChange::as_ref)
    }
}

impl LanguageProperty<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &LanguagePropertyOutputMembers {
        &self.output
    }
}

/// The members a `LanguageProperty` has only in a response (NGSI-LD clauses 4.5.18.2 and Table 5.2.32-2).
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct LanguagePropertyOutputMembers {
    /// `previousLanguageMap`: the language map before the change that triggered a notification, sent only when
    /// the subscription asks for `showChanges` (clause 4.5.18.2). It is written like `languageMap`, so it may be
    /// the NGSI-LD Null `{"@none": "urn:ngsi-ld:null"}` (clause 4.5.0).
    pub previous_language_map: Option<LanguageMap>,
}

attribute_methods!(LanguagePropertyBuilder, language_property_builder::State, attributes);

impl AttributeDraft<LanguageProperty<Input>> for LanguageProperty<Input> {
    fn into_attribute(self) -> Result<LanguageProperty<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: language_property_builder::IsComplete> AttributeDraft<LanguageProperty<Input>> for LanguagePropertyBuilder<S> {
    fn into_attribute(self) -> Result<LanguageProperty<Input>, BuildError> {
        self.build()
    }
}

/// A language map is a `LanguageProperty` with nothing but that map.
impl AttributeDraft<LanguageProperty<Input>> for LanguageMap {
    fn into_attribute(self) -> Result<LanguageProperty<Input>, BuildError> {
        LanguageProperty::builder(self).build()
    }
}

/// `(language, text)` pairs, validated when the document is built.
impl<K: AsRef<str>, T: Into<LanguageText>, const N: usize> AttributeDraft<LanguageProperty<Input>> for [(K, T); N] {
    fn into_attribute(self) -> Result<LanguageProperty<Input>, BuildError> {
        LanguageProperty::builder(LanguageMap::try_from_pairs(self)?).build()
    }
}
