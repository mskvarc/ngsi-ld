//! Vocabulary Properties (NGSI-LD clauses 4.5.20 and 5.2.35).

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
        OneOrMany,
        OneOrManyDraft,
        Output,
        Patch,
        Plain,
        Proof,
        ReservedMember,
        VocabTerm,
        attribute::pending::PendingAttributes,
        null,
    },
    seal::Seal,
};
use bon::bon;

/// A `VocabProperty`: one or more terms the `@context` coerces into IRIs.
///
/// NGSI-LD clauses 4.5.20 and 5.2.35.
///
/// Build one with [`VocabProperty::builder`] from a term or a `Vec` of terms; read one from a document through the
/// representation parsers.
///
/// ```
/// use ngsi_ld::model::{VocabProperty, VocabTerm};
///
/// let category = VocabProperty::builder("non-commercial".parse::<VocabTerm>()?).build()?;
/// assert_eq!(category.vocab().iter().count(), 1);
/// assert!(
///     VocabProperty::builder(Vec::<VocabTerm>::new())
///         .build()
///         .is_err()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabProperty<D: Direction> {
    /// `vocab`: the term or terms.
    pub(crate) vocab: OneOrMany<VocabTerm>,
    /// `valueType` (Table 5.2.35-1).
    pub(crate) value_type: D::Member<VocabTerm>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<VocabPropertyOutputMembers>,
}

#[bon]
impl VocabProperty<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set, an empty
    /// list of terms, and a `vocab` that is the NGSI-LD Null (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] vocab: OneOrManyDraft<VocabTerm>,
        #[builder(field)] attributes: PendingAttributes,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let vocab = vocab.resolve(ReservedMember::Vocab, &mut problems);
        if let Some(vocab) = &vocab {
            null::report(null::check_vocab(vocab), &mut problems);
        }
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        // An unresolved member has recorded its problem, so the error is never empty.
        match vocab {
            Some(vocab) => BuildError::finish(
                problems,
                Self {
                    vocab,
                    value_type,
                    meta,
                    output: (),
                },
            ),
            None => Err(BuildError::new(problems)),
        }
    }
}

impl<D: Direction> VocabProperty<D> {
    /// Returns the term or terms (`vocab`).
    #[must_use]
    pub const fn vocab(&self) -> &OneOrMany<VocabTerm> {
        &self.vocab
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> VocabProperty<D> {
    /// Returns the datatype (`valueType`).
    #[must_use]
    pub fn value_type(&self) -> Option<&VocabTerm> {
        D::member(Seal::new(), &self.value_type)
    }

    /// Converts the attribute to another direction by converting its shared members and dropping the output-only
    /// members of its kind: it serves the conversions out of a response and into a fragment, which keep none.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> VocabProperty<E> {
        VocabProperty {
            vocab: self.vocab,
            value_type: E::set_member(Seal::new(), D::into_member(Seal::new(), self.value_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl VocabProperty<Patch> {
    /// Returns the change the fragment makes to the datatype of the value (`valueType`): set, deleted with the NGSI-LD
    /// Null, or absent.
    #[must_use]
    pub fn value_type(&self) -> Option<MemberChange<&VocabTerm>> {
        self.value_type.as_ref().map(MemberChange::as_ref)
    }
}

impl VocabProperty<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &VocabPropertyOutputMembers {
        &self.output
    }
}

/// The members a `VocabProperty` has only in a response.
///
/// NGSI-LD clauses 4.5.20.2 and Table 5.2.35-2.
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct VocabPropertyOutputMembers {
    /// `previousVocab`: the vocab before the change that triggered a notification, sent only when the
    /// subscription asks for `showChanges` (clause 4.5.20.2). It is written like `vocab`.
    pub previous_vocab: Option<OneOrMany<VocabTerm>>,
}

attribute_methods!(VocabPropertyBuilder, vocab_property_builder::State, attributes);

impl AttributeDraft<VocabProperty<Input>> for VocabProperty<Input> {
    fn into_attribute(self) -> Result<VocabProperty<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: vocab_property_builder::IsComplete> AttributeDraft<VocabProperty<Input>> for VocabPropertyBuilder<S> {
    fn into_attribute(self) -> Result<VocabProperty<Input>, BuildError> {
        self.build()
    }
}

/// A term is a `VocabProperty` with nothing but that term.
impl AttributeDraft<VocabProperty<Input>> for VocabTerm {
    fn into_attribute(self) -> Result<VocabProperty<Input>, BuildError> {
        VocabProperty::builder(self).build()
    }
}

/// A term as a string, validated when the document is built.
impl AttributeDraft<VocabProperty<Input>> for &str {
    fn into_attribute(self) -> Result<VocabProperty<Input>, BuildError> {
        VocabProperty::builder(self.parse::<VocabTerm>()?).build()
    }
}
