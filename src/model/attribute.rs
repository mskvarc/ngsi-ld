//! Attributes: the eight kinds of Property and Relationship of NGSI-LD clause 5.2.4.

use crate::{
    model::{
        AttributeMeta,
        BuildError,
        BuildProblem,
        DatasetId,
        Direction,
        GeoProperty,
        Input,
        InstanceKey,
        JsonProperty,
        LanguageProperty,
        ListProperty,
        ListRelationship,
        Output,
        Patch,
        Plain,
        Property,
        Relationship,
        VocabProperty,
        attribute::instances::sealed::Sealed,
        null,
    },
    seal::Seal,
};

pub(super) mod attributes;
pub(super) mod deleted;
pub(super) mod fragment;
pub(super) mod geo_property;
pub(super) mod instances;
pub(super) mod json_property;
pub(super) mod language_property;
pub(super) mod list_property;
pub(super) mod list_relationship;
pub(super) mod meta;
#[cfg(feature = "linked-data")]
mod names;
pub(super) mod output;
pub(super) mod patch;
pub(crate) mod pending;
pub(super) mod proof;
pub(super) mod property;
pub(super) mod relationship;
pub(super) mod vocab_property;

/// The kind of an attribute, as its `type` member names it.
///
/// NGSI-LD clause 5.2.4.
///
/// The specification closes the set, so the enum is exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
pub enum AttributeKind {
    /// `Property` (clause 5.2.5).
    Property,
    /// `GeoProperty` (clause 5.2.7).
    GeoProperty,
    /// `LanguageProperty` (clause 5.2.32).
    LanguageProperty,
    /// `VocabProperty` (clause 5.2.35).
    VocabProperty,
    /// `ListProperty` (clause 5.2.36).
    ListProperty,
    /// `JsonProperty` (clause 5.2.38).
    JsonProperty,
    /// `Relationship` (clause 5.2.6).
    Relationship,
    /// `ListRelationship` (clause 5.2.37).
    ListRelationship,
}

impl AttributeKind {
    /// The member that holds this kind's value: `value`, `languageMap`, `vocab`, `valueList`, `json`, `object` or
    /// `objectList`.
    #[must_use]
    pub const fn payload_member(self) -> &'static str {
        match self {
            Self::Property | Self::GeoProperty => "value",
            Self::LanguageProperty => "languageMap",
            Self::VocabProperty => "vocab",
            Self::ListProperty => "valueList",
            Self::JsonProperty => "json",
            Self::Relationship => "object",
            Self::ListRelationship => "objectList",
        }
    }

    /// The type name as written in the `type` member.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

/// An attribute of any kind.
///
/// NGSI-LD clause 5.2.4.
///
/// Each kind is its own type, generic over the [`Direction`] of the document it belongs to; `Attribute` is the
/// closed sum of the eight.
///
/// ```
/// use ngsi_ld::model::{Attribute, AttributeKind, Property};
///
/// let brand: Attribute<_> = Property::builder("Mercedes").build()?.into();
/// assert_eq!(brand.kind(), AttributeKind::Property);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum Attribute<D: Direction> {
    /// A Property.
    Property(Property<D>),
    /// A `GeoProperty`.
    GeoProperty(GeoProperty<D>),
    /// A `LanguageProperty`.
    LanguageProperty(LanguageProperty<D>),
    /// A `VocabProperty`.
    VocabProperty(VocabProperty<D>),
    /// A `ListProperty`.
    ListProperty(ListProperty<D>),
    /// A `JsonProperty`.
    JsonProperty(JsonProperty<D>),
    /// A Relationship.
    Relationship(Relationship<D>),
    /// A `ListRelationship`.
    ListRelationship(ListRelationship<D>),
}

impl<D: Direction> Attribute<D> {
    /// Returns the kind.
    #[must_use]
    pub const fn kind(&self) -> AttributeKind {
        match self {
            Self::Property(_) => AttributeKind::Property,
            Self::GeoProperty(_) => AttributeKind::GeoProperty,
            Self::LanguageProperty(_) => AttributeKind::LanguageProperty,
            Self::VocabProperty(_) => AttributeKind::VocabProperty,
            Self::ListProperty(_) => AttributeKind::ListProperty,
            Self::JsonProperty(_) => AttributeKind::JsonProperty,
            Self::Relationship(_) => AttributeKind::Relationship,
            Self::ListRelationship(_) => AttributeKind::ListRelationship,
        }
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        match self {
            Self::Property(attribute) => &attribute.meta,
            Self::GeoProperty(attribute) => &attribute.meta,
            Self::LanguageProperty(attribute) => &attribute.meta,
            Self::VocabProperty(attribute) => &attribute.meta,
            Self::ListProperty(attribute) => &attribute.meta,
            Self::JsonProperty(attribute) => &attribute.meta,
            Self::Relationship(attribute) => &attribute.meta,
            Self::ListRelationship(attribute) => &attribute.meta,
        }
    }

    /// Returns the shared members for modification.
    pub(crate) const fn meta_mut(&mut self) -> &mut AttributeMeta<D> {
        match self {
            Self::Property(attribute) => &mut attribute.meta,
            Self::GeoProperty(attribute) => &mut attribute.meta,
            Self::LanguageProperty(attribute) => &mut attribute.meta,
            Self::VocabProperty(attribute) => &mut attribute.meta,
            Self::ListProperty(attribute) => &mut attribute.meta,
            Self::JsonProperty(attribute) => &mut attribute.meta,
            Self::Relationship(attribute) => &mut attribute.meta,
            Self::ListRelationship(attribute) => &mut attribute.meta,
        }
    }
}

impl<D: Plain> Attribute<D> {
    /// Converts the attribute to another direction by converting its shared members.
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> Attribute<E> {
        match self {
            Self::Property(attribute) => Attribute::Property(attribute.map_meta(map)),
            Self::GeoProperty(attribute) => Attribute::GeoProperty(attribute.map_meta(map)),
            Self::LanguageProperty(attribute) => Attribute::LanguageProperty(attribute.map_meta(map)),
            Self::VocabProperty(attribute) => Attribute::VocabProperty(attribute.map_meta(map)),
            Self::ListProperty(attribute) => Attribute::ListProperty(attribute.map_meta(map)),
            Self::JsonProperty(attribute) => Attribute::JsonProperty(attribute.map_meta(map)),
            Self::Relationship(attribute) => Attribute::Relationship(attribute.map_meta(map)),
            Self::ListRelationship(attribute) => Attribute::ListRelationship(attribute.map_meta(map)),
        }
    }
}

impl<D: Direction> Sealed for Attribute<D> {}

impl<D: Direction> InstanceKey for Attribute<D> {
    fn dataset_id(&self) -> Option<&DatasetId> {
        self.meta().dataset_id.as_ref()
    }
}

/// What occupies an attribute's place in its parent.
///
/// It is an attribute, or, in a fragment, a change to one.
pub trait AttributeSlot: InstanceKey {
    /// The kind the slot writes; `None` for a deletion that names no kind.
    #[doc(hidden)]
    fn written_kind(&self, seal: Seal) -> Option<AttributeKind>;
}

impl<D: Direction> AttributeSlot for Attribute<D> {
    /// A broker reports a deleted attribute of any kind as a Property holding the NGSI-LD Null (clause 5.8.6), so
    /// such a Property names no kind; a request never holds one (clause 5.5.4).
    fn written_kind(&self, _: Seal) -> Option<AttributeKind> {
        match self {
            Self::Property(property) if null::is_null_value(property.value()) => None,
            Self::Property(_)
            | Self::GeoProperty(_)
            | Self::LanguageProperty(_)
            | Self::VocabProperty(_)
            | Self::ListProperty(_)
            | Self::JsonProperty(_)
            | Self::Relationship(_)
            | Self::ListRelationship(_) => Some(self.kind()),
        }
    }
}

impl Attribute<Output> {
    /// Whether the instance is the record of a deleted attribute: its payload is the NGSI-LD Null in the shape its
    /// kind needs (clause 4.5.0), as a notification reports a deleted attribute (clause 5.8.6) and a temporal
    /// evolution records one (clauses 4.5.7 and 4.5.8). The record may carry its `datasetId`, `deletedAt` and
    /// previous payload as well; a Property whose value only holds the Null as a member is not deleted.
    ///
    /// ```
    /// use ngsi_ld::model::{Attribute, Lossless, Output};
    ///
    /// let bare = Attribute::<Output>::parse(r#""urn:ngsi-ld:null""#, Lossless::Normalized)?;
    /// assert!(bare.is_deleted());
    /// let record = Attribute::<Output>::parse(
    ///     concat!(
    ///         r#"{"type": "Relationship", "object": "urn:ngsi-ld:null","#,
    ///         r#" "previousObject": "urn:ngsi-ld:Person:Bob","#,
    ///         r#" "deletedAt": "2024-01-02T00:00:00Z"}"#
    ///     ),
    ///     Lossless::Normalized,
    /// )?;
    /// assert!(record.is_deleted());
    /// let speed =
    ///     Attribute::<Output>::parse(r#"{"type": "Property", "value": 80}"#, Lossless::Normalized)?;
    /// assert!(!speed.is_deleted());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        null::is_null_payload(self)
    }

    /// Drops the output-only members, at every level, giving an attribute a client can send back (NGSI-LD
    /// clause 5.2.2).
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] when the attribute or a sub-attribute holds the NGSI-LD Null, which a broker writes
    /// for a deleted attribute and a request cannot carry (clause 5.5.4).
    pub fn into_input(self) -> Result<Attribute<Input>, BuildError> {
        let mut problems = Vec::new();
        let input = self.into_input_reporting(&mut problems);
        BuildError::finish(problems, input)
    }

    /// Drops the output-only members, recording every NGSI-LD Null the request attribute would hold.
    pub(crate) fn into_input_reporting(self, problems: &mut Vec<BuildProblem>) -> Attribute<Input> {
        let input = self.map_meta(|meta| meta.into_input(problems));
        null::report(null::check_attribute(&input), problems);
        input
    }
}

impl Attribute<Input> {
    /// Turns the attribute into one a fragment writes, every sub-attribute becoming a [`Change::Set`].
    ///
    /// [`Change::Set`]: crate::model::Change::Set
    #[must_use]
    pub fn into_patch(self) -> Attribute<Patch> {
        self.map_meta(AttributeMeta::into_patch)
    }

    /// Turns the attribute into one a response carries, at every level, with no output-only member: the attribute
    /// a context source or a broker returns when nothing but its content is asked for (clause 5.2.2). The per-kind
    /// `output_builder`s (such as [`Property::output_builder`]) add the output-only members.
    ///
    /// ```
    /// use ngsi_ld::model::{Attribute, Lossless, Property};
    ///
    /// let speed = Attribute::from(Property::builder(80).build()?).into_output();
    /// assert_eq!(
    ///     speed.render(Lossless::Normalized).to_string(),
    ///     r#"{"type":"Property","value":80}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn into_output(self) -> Attribute<Output> {
        self.map_meta(AttributeMeta::into_output)
    }
}

/// Something that becomes an attribute of kind `T` when a builder finishes.
///
/// It is an attribute already built, an attribute builder whose required members are set, or a value the kind can be
/// made from directly.
///
/// Builders take drafts so a caller never finishes an inner builder by hand; every problem found in any of
/// them is reported once, by the outermost `build()`.
pub trait AttributeDraft<T> {
    /// Builds the attribute.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] with every problem found.
    fn into_attribute(self) -> Result<T, BuildError>;
}

#[cfg(test)]
mod tests;
