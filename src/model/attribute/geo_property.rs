//! Geospatial Properties (NGSI-LD clauses 4.7 and 5.2.7).

use crate::model::{
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
    VocabTerm,
    attribute::pending::PendingAttributes,
    geometry::{Geometry, GeometryError, Shape},
    null::NGSI_LD_NULL,
};
use bon::bon;
use cefact_units::UnitCode;
use jstrict::Value as Json;

/// The value of a `GeoProperty`: a geometry (NGSI-LD clause 4.7.1) or, in a notification reporting a deleted
/// `GeoProperty`, the NGSI-LD Null (clauses 4.5.0 and 5.8.6).
///
/// A builder takes a [`Geometry`], so a client never builds the Null; a request document holding it is refused
/// (clause 5.5.4).
///
/// ```
/// use ngsi_ld::model::{
///     GeoValue,
///     geometry::{Geometry, Position, Shape},
/// };
///
/// let point = GeoValue::from(Geometry::from(Shape::Point(Position::try_new(8.0, 40.0)?)));
/// assert!(
///     point
///         .as_geometry()
///         .is_some_and(|geometry| geometry.shape().is_point())
/// );
/// assert!(GeoValue::Null.as_geometry().is_none());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum GeoValue {
    /// A geometry.
    Geometry(Geometry),
    /// The NGSI-LD Null: the `GeoProperty` was deleted.
    #[from(skip)]
    Null,
}

impl GeoValue {
    /// Returns the geometry, `None` for the NGSI-LD Null.
    #[must_use]
    pub const fn as_geometry(&self) -> Option<&Geometry> {
        match self {
            Self::Geometry(geometry) => Some(geometry),
            Self::Null => None,
        }
    }

    /// Reads a geometry, or the NGSI-LD Null written as its URI string.
    pub(crate) fn from_json(json: &Json) -> Result<Self, GeometryError> {
        match json {
            Json::String(text) if text.as_str() == NGSI_LD_NULL.as_str() => Ok(Self::Null),
            Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) | Json::Object(_) => {
                Geometry::from_json(json).map(Self::Geometry)
            }
        }
    }

    /// Writes the geometry, or the NGSI-LD Null as its URI string.
    pub(crate) fn to_json(&self) -> Json {
        match self {
            Self::Geometry(geometry) => geometry.to_json(),
            Self::Null => Json::from(NGSI_LD_NULL.as_str()),
        }
    }
}

/// A `GeoProperty`: a Property whose value is a geometry (NGSI-LD clauses 4.7 and 5.2.7).
///
/// Build one with [`GeoProperty::builder`]; read one from a document through the representation parsers.
///
/// ```
/// use ngsi_ld::model::{
///     GeoProperty,
///     geometry::{Position, Shape},
/// };
///
/// let location =
///     GeoProperty::builder(Shape::Point(Position::try_new(13.3986, 52.5547)?)).build()?;
/// assert!(
///     location
///         .value()
///         .as_geometry()
///         .is_some_and(|geometry| geometry.shape().is_point())
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoProperty<D: Direction> {
    /// `value`: the geometry (clause 4.7.1), or the NGSI-LD Null of a deleted `GeoProperty` in a notification.
    pub(crate) value: GeoValue,
    /// `unitCode`, which clause 4.7.3 carries over from Property.
    pub(crate) unit_code: D::Member<UnitCode>,
    /// `valueType` (Table 5.2.7-1).
    pub(crate) value_type: D::Member<VocabTerm>,
    /// The members every kind shares.
    pub(crate) meta: AttributeMeta<D>,
    /// The output-only members of this kind; `()` outside responses.
    pub(crate) output: D::KindOutput<GeoPropertyOutputMembers>,
}

#[bon]
impl GeoProperty<Input> {
    /// Starts a builder from the kind's payload. The optional members of the kind and of every attribute
    /// (`datasetId`, `observedAt`, `expiresAt`) have setters; sub-attributes are added with the per-kind
    /// methods (`property`, `relationship`, ...), and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing every invalid sub-attribute name, value or instance set.
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] value: Geometry,
        #[builder(field)] attributes: PendingAttributes,
        unit_code: Option<UnitCode>,
        value_type: Option<VocabTerm>,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        expires_at: Option<DateTime>,
        proof: Option<Proof<Input>>,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let meta = AttributeMeta::build(dataset_id, observed_at, expires_at, proof, attributes, &mut problems);
        BuildError::finish(
            problems,
            Self {
                value: GeoValue::Geometry(value),
                unit_code,
                value_type,
                meta,
                output: (),
            },
        )
    }
}

impl<D: Direction> GeoProperty<D> {
    /// Returns the value (`value`): a geometry, or the NGSI-LD Null of a deleted `GeoProperty` in a notification.
    #[must_use]
    pub const fn value(&self) -> &GeoValue {
        &self.value
    }

    /// Returns the members every kind shares.
    #[must_use]
    pub const fn meta(&self) -> &AttributeMeta<D> {
        &self.meta
    }
}

impl<D: Plain> GeoProperty<D> {
    /// Returns the unit (`unitCode`).
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
    pub(crate) fn map_meta<E: Direction>(self, map: impl FnOnce(AttributeMeta<D>) -> AttributeMeta<E>) -> GeoProperty<E> {
        GeoProperty {
            value: self.value,
            unit_code: E::set_member(D::into_member(self.unit_code)),
            value_type: E::set_member(D::into_member(self.value_type)),
            meta: map(self.meta),
            output: Default::default(),
        }
    }
}

impl GeoProperty<Patch> {
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

impl GeoProperty<Output> {
    /// Returns the members only a response carries for this kind.
    #[must_use]
    pub const fn output(&self) -> &GeoPropertyOutputMembers {
        &self.output
    }
}

/// The members a `GeoProperty` has only in a response (NGSI-LD clauses 4.5.2.2 and Table 5.2.7-2).
///
/// A broker writes them and a request never carries them, so the type exists only on the [`Output`] direction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct GeoPropertyOutputMembers {
    /// `previousValue`: the geometry before the change that triggered a notification, sent only when the
    /// subscription asks for `showChanges` (clause 4.5.2.2). It is written like `value`, so it may be the NGSI-LD
    /// Null (clause 4.5.0).
    pub previous_value: Option<GeoValue>,
}

attribute_methods!(GeoPropertyBuilder, geo_property_builder::State, attributes);

impl AttributeDraft<GeoProperty<Input>> for GeoProperty<Input> {
    fn into_attribute(self) -> Result<GeoProperty<Input>, BuildError> {
        Ok(self)
    }
}

impl<S: geo_property_builder::IsComplete> AttributeDraft<GeoProperty<Input>> for GeoPropertyBuilder<S> {
    fn into_attribute(self) -> Result<GeoProperty<Input>, BuildError> {
        self.build()
    }
}

/// A geometry is a `GeoProperty` with nothing but that geometry.
impl AttributeDraft<GeoProperty<Input>> for Geometry {
    fn into_attribute(self) -> Result<GeoProperty<Input>, BuildError> {
        GeoProperty::builder(self).build()
    }
}

/// A shape is a `GeoProperty` with nothing but that shape, without a bounding box.
impl AttributeDraft<GeoProperty<Input>> for Shape {
    fn into_attribute(self) -> Result<GeoProperty<Input>, BuildError> {
        GeoProperty::builder(self).build()
    }
}
