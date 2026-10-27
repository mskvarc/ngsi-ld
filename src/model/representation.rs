//! Representations: the JSON forms an entity takes on the wire (NGSI-LD clauses 4.5.1 to 4.5.4).

use crate::model::{
    Attribute,
    AttributeFragment,
    Change,
    Completeness,
    Deletion,
    DocumentError,
    DocumentErrorKind,
    Entity,
    EntityFragment,
    Input,
    InstanceDeletion,
    MemberError,
    NullMember,
    Output,
    Patch,
    Projected,
    RawJson,
    SimplifiedEntity,
    keyword_alias::unalias_attribute,
    reader::parse_json,
    representation::{
        read::{
            fragment::read_fragment,
            members::{FragmentShape, read_members},
            read_entity,
            read_instance_list,
            read_instances,
        },
        write::{
            Placement,
            direction::WriteEntityDirection,
            write_attribute,
            write_attribute_body,
            write_context,
            write_entity,
            write_fragment,
            write_fragment_object,
        },
    },
};
use jstrict::{Object, Value as Json};

pub(crate) mod kind_output;
pub(crate) mod member;
pub(crate) mod proof;
pub(crate) mod read;
pub(crate) mod shape;
pub(super) mod simplified_spelling;
pub(crate) mod write;

/// The representation of entities a request asks for, or a document is written in.
///
/// It is the `format` parameter of NGSI-LD clause 6.3.7. The specification closes the set, so the enum is exhaustive.
///
/// Normalized and concise are two lossless spellings of the same information; any [`Entity`] is written in
/// either and read from either. Simplified (clause 4.5.4) is lossy and has its own model,
/// [`SimplifiedEntity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum Representation {
    /// Every attribute with its `type` (clauses 4.5.2.2, 4.5.3.2).
    #[default]
    Normalized,
    /// The shortest lossless form (clauses 4.5.2.3, 4.5.3.3).
    Concise,
    /// Attribute values only (clause 4.5.4); also spelled `keyValues`.
    #[strum(to_string = "simplified", serialize = "keyValues")]
    Simplified,
}

/// The two lossless representations, normalized and concise.
///
/// An [`Entity`] can be read from and written in either.
///
/// Reading: in normalized form the `type` member decides an attribute's kind. In concise form the kind is inferred
/// by the rules of clauses 4.5.2.3, 4.5.3.3, 4.5.18.3, 4.5.20.3, 4.5.21.3, 4.5.22.3, 4.5.24.3 and 4.7.3, applied in
/// this order:
///
/// 1. an object whose `type` names an attribute kind is read as that kind (clause 4.5.2.3: "the whole
///    Attribute shall be treated as a normalized representation");
/// 2. an object that is a supported geometry is a `GeoProperty` (clause 4.7.3);
/// 3. any other object with a `type` is an error naming the unknown type;
/// 4. `object`, `objectList`, `languageMap`, `vocab`, `valueList` or `json` decides the kind; two of them are
///    an error, because no rule says which wins;
/// 5. `value` means a Property, or a `GeoProperty` when the value is a supported geometry (clause 4.7.3);
/// 6. anything else is the bare value of a Property (clause 4.5.2.3).
///
/// A JSON array is a list of instances when every element is an object one of the rules above reads as an
/// attribute; otherwise it is the bare array value of a Property.
///
/// Every member keeps its place: attributes and sub-attributes are kept in document order.
///
/// Clause 4.4 lets a request write the JSON-LD keywords `@id` and `@type` instead of the terms `id` and `type`. A
/// reader takes them for the terms, in the entity and, with `@type`, in each attribute whose `@type` names a kind of
/// attribute, so a JSON-LD value object keeps its `@type`; it refuses a term given beside its keyword as a member
/// given twice, and the document is written back with the terms. A term a user `@context` aliases to a keyword is a
/// term of that context, which a reader does not apply: the `@context` is kept and no name is expanded.
///
/// Writing produces the canonical member order, the one the specification's examples use (clauses 4.5.1, C.2.2
/// and C.5.15):
///
/// - an entity: `id`, `type`, `scope`, `expiresAt`, its attributes in their order, then `createdAt`,
///   `modifiedAt` and `deletedAt`, then `@context`;
/// - an attribute: `type`, then `objectType` for the relationship kinds, the payload (`value`, `languageMap`,
///   `vocab`, `valueList`, `json`, `object` or `objectList`), `valueType` and `unitCode` for the property kinds,
///   then `datasetId`, `observedAt`, `expiresAt`, the output-only members of the kind (its `previous*` member,
///   then `lang`, `entity` or `entityList`), the output-only members every kind shares (`createdAt`,
///   `modifiedAt`, `deletedAt`, `instanceId`), `ngsildproof`, and the sub-attributes in their order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum Lossless {
    /// Every attribute with its `type`.
    #[default]
    Normalized,
    /// The shortest lossless form.
    Concise,
}

/// Writes an entity in `form`, in canonical member order, with its own `@context` last when it has one.
pub(crate) fn entity_object<D: WriteEntityDirection, C: Completeness>(entity: &Entity<D, C>, form: Lossless) -> Object {
    let mut object = write_entity(entity, form);
    write_context(&mut object, entity.context.as_ref());
    object
}

/// Writes an entity in `representation`, in canonical member order, with its own `@context` last when it has one.
fn render_entity<D: WriteEntityDirection, C: Completeness>(entity: &Entity<D, C>, representation: Representation) -> RawJson {
    let lossless = |form| RawJson::from_json(Json::Object(entity_object(entity, form)));
    match representation {
        Representation::Normalized => lossless(Lossless::Normalized),
        Representation::Concise => lossless(Lossless::Concise),
        Representation::Simplified => SimplifiedEntity::from(entity).render(),
    }
}

impl Entity<Input> {
    /// Writes the entity in `representation`, with its members in the order `id`,
    /// `type`, `scope`, `expiresAt`, the attributes in their order, and the entity's own `@context` last when it
    /// has one. Simplified drops everything but the values (clause 4.5.4). A request body is written without the
    /// entity's `@context`, because the request pipeline delivers the endpoint's (clause 6.3.5).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::{Entity, Representation};
    ///
    /// let vehicle = Entity::builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
    ///     .entity_type("Vehicle")
    ///     .property("brandName", "Mercedes")
    ///     .build()?;
    /// assert_eq!(
    ///     vehicle.render(Representation::Concise).to_string(),
    ///     r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","brandName":"Mercedes"}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn render(&self, representation: Representation) -> RawJson {
        render_entity(self, representation)
    }

    /// Reads an entity a client could send, in normalized or concise representation. A concise reader also reads
    /// normalized attributes, as clause 4.5.2.3 requires. An `@context` member is kept (clause 4.5.1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including any output-only member, any attribute
    /// holding the NGSI-LD Null (clause 5.5.4), and any `location`, `observationSpace` or `operationSpace` that is
    /// not a `GeoProperty` (clause 5.2.4).
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_entity(parse_json(text)?, form)
    }
}

impl Entity<Output> {
    /// Writes the entity in `representation`, with its output-only members after the attributes and its own
    /// `@context` last when it has one.
    #[must_use]
    pub fn render(&self, representation: Representation) -> RawJson {
        render_entity(self, representation)
    }

    /// Reads an entity a broker returned, in normalized or concise representation, keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_entity(parse_json(text)?, form)
    }
}

impl Entity<Output, Projected> {
    /// Writes the entity in `representation`, writing `id` and `type` only when it holds them.
    #[must_use]
    pub fn render(&self, representation: Representation) -> RawJson {
        render_entity(self, representation)
    }

    /// Reads an entity a projection may have reduced (clause 4.21), in normalized or concise representation,
    /// keeping its `@context`; `id` and `type` may be missing.
    ///
    /// ```
    /// use ngsi_ld::model::{Entity, Lossless, Output, Projected};
    ///
    /// let speed = Entity::<Output, Projected>::parse(
    ///     r#"{"speed": {"type": "Property", "value": 80}}"#,
    ///     Lossless::Normalized,
    /// )?;
    /// assert_eq!(speed.id, None);
    /// assert_eq!(speed.entity_type, None);
    /// assert_eq!(speed.attributes.len(), 1);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_entity(parse_json(text)?, form)
    }
}

impl Attribute<Input> {
    /// Writes the attribute in normalized or concise representation, as Replace Attribute takes it (clause
    /// 6.7.3.3).
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(write_attribute(self, form, Placement::Alone))
    }
}

impl Attribute<Patch> {
    /// Writes the attribute fragment in normalized or concise representation, as Partial Attribute Update takes
    /// it (clause 6.7.3.1).
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(write_attribute(self, form, Placement::Alone))
    }
}

impl Attribute<Output> {
    /// Writes the attribute in normalized or concise representation, with its output-only members.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(write_attribute(self, form, Placement::Alone))
    }

    /// Reads one attribute instance from JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the text is not one attribute in `form`.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        single(read_instances::<Output>(parse_json(text)?, form)?.into_vec())
    }
}

impl Attribute<Input> {
    /// Reads one attribute instance a client could send.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the text is not one attribute in `form`, holds an output-only member, or
    /// holds the NGSI-LD Null (clause 5.5.4).
    pub fn parse_input(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        single(read_instances::<Input>(parse_json(text)?, form)?.into_vec())
    }
}

impl<C: Completeness> EntityFragment<C> {
    /// Writes the fragment in normalized or concise representation (clause 5.4), with its own `@context` last when
    /// it has one. A request body is written without it, because the request pipeline delivers the endpoint's
    /// (clause 6.3.5).
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        let mut object = write_fragment(self, form);
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads a fragment in normalized or concise representation, as a broker forwards it to a context source for
    /// Update Attributes, Append Attributes or Merge Entity (clauses 5.6.2, 5.6.3 and 5.6.17), keeping its
    /// `@context` and the order of its attributes. An attribute written as the bare NGSI-LD Null, or an instance
    /// whose payload is the Null in the shape of its kind, is read as a [`Change::Delete`]; every other attribute
    /// as a [`Change::Set`] (clauses 5.4, 5.5.8 and 5.5.12). `EntityFragment::<Projected>::parse` reads a fragment
    /// whose `id` and `type` are optional; `EntityFragment::<Complete>::parse` requires them, as a batch element holds
    /// them.
    ///
    /// [`Change::Delete`]: crate::model::Change::Delete
    /// [`Change::Set`]: crate::model::Change::Set
    ///
    /// ```
    /// use ngsi_ld::model::{AttributeName, EntityFragment, Lossless, Projected};
    ///
    /// let fragment = EntityFragment::<Projected>::parse(
    ///     r#"{"speed": {"type": "Property", "value": 0}, "isParked": "urn:ngsi-ld:null"}"#,
    ///     Lossless::Normalized,
    /// )?;
    /// let parked: AttributeName = "isParked".parse()?;
    /// assert!(
    ///     fragment
    ///         .attributes
    ///         .get(&parked)
    ///         .is_some_and(|instances| instances.first().is_delete())
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a system member (clause 5.2.2), an
    /// output-only member, and an instance that holds the NGSI-LD Null besides other members.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        read_fragment(parse_json(text)?, form)
    }
}

impl AttributeFragment {
    /// Writes the fragment in normalized or concise representation, as Partial Attribute Update takes it (clause
    /// 6.7.3.1): always an object, which can carry an `@context`.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(write_fragment_object(self, form)))
    }
}

impl Change<AttributeFragment, InstanceDeletion> {
    /// Reads the body of Partial Attribute Update (clauses 5.6.4 and 6.7.3.1): an attribute fragment, whose
    /// sub-attributes and optional members may be deleted with the NGSI-LD Null and whose Property value may hold a
    /// nested Null (clauses 5.5.8 and 5.5.12), or the deletion of the instance, an object whose payload is the Null
    /// in the shape of its kind, with no member besides `type` and `datasetId` (clauses 4.5.2.2 and 4.5.3.2).
    ///
    /// An object that names no payload (`value`, `object`, `languageMap`, ...) is an [`AttributeMembers`](crate::model::AttributeMembers), in either
    /// form: `{"value": 100}` sets the value, `{"observedAt": "2022-03-14T12:51:02Z"}` changes only the `observedAt`,
    /// and `{}` changes nothing.
    ///
    /// ```
    /// use ngsi_ld::model::{AttributeFragment, AttributeKind, Change, InstanceDeletion, Lossless};
    ///
    /// let deletion = Change::<AttributeFragment, InstanceDeletion>::parse(
    ///     r#"{"type": "Property", "value": "urn:ngsi-ld:null"}"#,
    ///     Lossless::Normalized,
    /// )?;
    /// assert_eq!(
    ///     deletion,
    ///     Change::Delete(InstanceDeletion::new(AttributeKind::Property))
    /// );
    /// let observed = Change::<AttributeFragment, InstanceDeletion>::parse(
    ///     r#"{"observedAt": "2022-03-14T12:51:02Z"}"#,
    ///     Lossless::Concise,
    /// )?;
    /// assert!(matches!(
    ///     observed,
    ///     Change::Set(AttributeFragment::WithoutPayload(_))
    /// ));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the text is not one attribute object in `form`, holds an output-only member,
    /// or is the bare NGSI-LD Null, which deletes a member of an entity and is no attribute object.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        let json = match parse_json(text)? {
            Json::Object(object) => {
                let object = unalias_attribute(object)?;
                match FragmentShape::of(&object) {
                    FragmentShape::Members(kind) => return read_members(kind, object, form).map(|members| Change::Set(members.into())),
                    FragmentShape::Payload => Json::Object(object),
                }
            }
            other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => other,
        };
        match single(read_instance_list::<Patch>(json, form)?)? {
            Change::Set(attribute) => Ok(Change::Set(attribute.into())),
            Change::Delete(Deletion::Instance(deletion)) => Ok(Change::Delete(deletion)),
            Change::Delete(Deletion::Member) => Err(DocumentError::top(DocumentErrorKind::NgsiLdNull { member: NullMember::Value })),
        }
    }

    /// Writes the body of Partial Attribute Update in normalized or concise representation (clause 6.7.3.1): always
    /// an object, which can carry an `@context`.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        RawJson::from_json(Json::Object(write_attribute_body(self, form)))
    }
}

/// The only instance of a one-instance document.
fn single<T>(instances: Vec<T>) -> Result<T, DocumentError> {
    <[T; 1]>::try_from(instances)
        .map(|[one]| one)
        .map_err(|_| DocumentError::top(MemberError::NotAnObject.into()))
}

#[cfg(test)]
mod tests;
