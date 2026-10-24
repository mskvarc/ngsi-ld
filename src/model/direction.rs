//! Which way a document travels: a request a client builds, a fragment that changes an existing element, or a
//! response a broker returns.

use crate::{
    model::{
        Attribute,
        AttributeKind,
        AttributeSlot,
        DatasetId,
        DateTime,
        InstanceId,
        InstanceKey,
        MemberChange,
        attribute::instances::sealed::Sealed as InstanceSealed,
    },
    seal::Seal,
};
use std::fmt::Debug;

#[cfg(feature = "linked-data")]
pub(crate) mod names;

/// The private supertrait that keeps `Direction` implemented by this crate alone.
mod sealed {
    /// Seals [`Direction`](super::Direction): the three directions are the only ones the specification has.
    pub trait Sealed {}
}

/// The direction of a document: [`Input`], [`Patch`] or [`Output`].
///
/// NGSI-LD clause 5.2.2 makes `createdAt`, `modifiedAt` and `deletedAt` read-only, and clauses 5.2.5 to 5.2.7
/// add output-only members such as `instanceId`. A document a client sends cannot carry them; a document a
/// broker returns may. Clauses 5.5.8 and 5.5.12 allow the NGSI-LD Null, which deletes a member, only in
/// fragments. Every entity and attribute type is therefore parameterised by a [`Direction`], whose associated
/// types decide what the document can hold. A fragment may also delete an optional member of an attribute, such as
/// its `observedAt` or `unitCode`, with the Null (clause 5.5.8), so it holds a change to each of them.
///
/// The trait is sealed; its associated types are what a document of that direction may carry.
pub trait Direction: sealed::Sealed + Debug + Clone + Copy + PartialEq + Eq + Default + 'static {
    /// What occupies an attribute's place in its parent: the attribute itself, or, in a fragment, a
    /// [`Change`] to it.
    type Slot: Debug + Clone + PartialEq + Eq + AttributeSlot;
    /// The output-only members of an attribute: `()` for requests, [`AttributeOutputMembers`] for responses.
    type AttributeOutput: Debug + Clone + PartialEq + Eq + Default;
    /// The output-only members of an entity: `()` for requests, [`EntityOutputMembers`] for responses.
    type EntityOutput: Debug + Clone + PartialEq + Eq + Default;
    /// The output-only members one attribute kind adds to the shared ones, `K` (such as
    /// [`PropertyOutputMembers`](crate::model::PropertyOutputMembers)): `()` for requests,
    /// `K` itself for responses. A kind's `previous*` member, the linked entities of inline retrieval and the
    /// `lang` of the language filter therefore exist only on a response attribute of the kind that owns them.
    type KindOutput<K: Debug + Clone + PartialEq + Eq + Default>: Debug + Clone + PartialEq + Eq + Default;
    /// An optional member of an attribute besides its `datasetId`: `observedAt`, `expiresAt`, `unitCode`,
    /// `valueType` and `objectType`. A request or a response holds it or not (`Option<T>`); a fragment holds a
    /// [`MemberChange`] to it (`Option<MemberChange<T>>`), since a member whose value is the NGSI-LD Null is deleted
    /// (clauses 5.5.8 and 5.5.12), which clause 5.5.8 refuses for `datasetId` alone.
    type Member<T: Debug + Clone + PartialEq + Eq>: Debug + Clone + PartialEq + Eq + Default;

    /// Places an attribute in a slot of this direction.
    #[doc(hidden)]
    fn slot(seal: Seal, attribute: Attribute<Self>) -> Self::Slot;

    /// The member that sets `value`, or that is absent.
    #[doc(hidden)]
    fn set_member<T: Debug + Clone + PartialEq + Eq>(seal: Seal, value: Option<T>) -> Self::Member<T>;

    /// Whether the member is absent: neither set nor, in a fragment, deleted.
    #[doc(hidden)]
    fn lacks<T: Debug + Clone + PartialEq + Eq>(seal: Seal, member: &Self::Member<T>) -> bool;
}

/// A direction whose attributes hold their optional members as they are.
///
/// It is [`Input`] or [`Output`], unlike a [`Patch`], which holds changes to them.
///
/// The trait is sealed, as [`Direction`] is.
pub trait Plain: Direction {
    /// Returns the value of the member, if it is present.
    #[doc(hidden)]
    fn member<T: Debug + Clone + PartialEq + Eq>(seal: Seal, member: &Self::Member<T>) -> Option<&T>;

    /// Takes the value of the member, if it is present.
    #[doc(hidden)]
    fn into_member<T: Debug + Clone + PartialEq + Eq>(seal: Seal, member: Self::Member<T>) -> Option<T>;
}

/// A document a client sends to create or replace an element.
///
/// Create Entity, Replace Entity, Replace Attribute and Append Attributes send it. It holds neither system members nor
/// the NGSI-LD Null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Input;

/// A fragment that changes an existing element.
///
/// Merge Entity, Update Attributes and Partial Attribute Update send it (NGSI-LD clause 5.4). Every attribute is a
/// [`Change`], so a fragment can delete what it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Patch;

/// A document a broker returns.
///
/// Retrieve Entity, Query Entities and notifications carry it. It may hold the system members of NGSI-LD clause 5.2.2
/// and the other output-only members: the `previous*` members of notifications, the linked entities of inline retrieval
/// (clause 4.5.23.2) and the `lang` of the language filter (clause 4.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Output;

impl sealed::Sealed for Input {}
impl sealed::Sealed for Patch {}
impl sealed::Sealed for Output {}

impl Direction for Input {
    type AttributeOutput = ();
    type EntityOutput = ();
    type KindOutput<K: Debug + Clone + PartialEq + Eq + Default> = ();
    type Member<T: Debug + Clone + PartialEq + Eq> = Option<T>;
    type Slot = Attribute<Self>;

    fn slot(_: Seal, attribute: Attribute<Self>) -> Self::Slot {
        attribute
    }

    fn set_member<T: Debug + Clone + PartialEq + Eq>(_: Seal, value: Option<T>) -> Option<T> {
        value
    }

    fn lacks<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: &Option<T>) -> bool {
        member.is_none()
    }
}

impl Plain for Input {
    fn member<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: &Option<T>) -> Option<&T> {
        member.as_ref()
    }

    fn into_member<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: Option<T>) -> Option<T> {
        member
    }
}

impl Direction for Patch {
    type AttributeOutput = ();
    type EntityOutput = ();
    type KindOutput<K: Debug + Clone + PartialEq + Eq + Default> = ();
    type Member<T: Debug + Clone + PartialEq + Eq> = Option<MemberChange<T>>;
    type Slot = Change<Attribute<Self>>;

    fn slot(_: Seal, attribute: Attribute<Self>) -> Self::Slot {
        Change::Set(attribute)
    }

    fn set_member<T: Debug + Clone + PartialEq + Eq>(_: Seal, value: Option<T>) -> Option<MemberChange<T>> {
        value.map(MemberChange::Set)
    }

    fn lacks<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: &Option<MemberChange<T>>) -> bool {
        member.is_none()
    }
}

impl Direction for Output {
    type AttributeOutput = AttributeOutputMembers;
    type EntityOutput = EntityOutputMembers;
    type KindOutput<K: Debug + Clone + PartialEq + Eq + Default> = K;
    type Member<T: Debug + Clone + PartialEq + Eq> = Option<T>;
    type Slot = Attribute<Self>;

    fn slot(_: Seal, attribute: Attribute<Self>) -> Self::Slot {
        attribute
    }

    fn set_member<T: Debug + Clone + PartialEq + Eq>(_: Seal, value: Option<T>) -> Option<T> {
        value
    }

    fn lacks<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: &Option<T>) -> bool {
        member.is_none()
    }
}

impl Plain for Output {
    fn member<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: &Option<T>) -> Option<&T> {
        member.as_ref()
    }

    fn into_member<T: Debug + Clone + PartialEq + Eq>(_: Seal, member: Option<T>) -> Option<T> {
        member
    }
}

/// The members of an attribute only a broker writes.
///
/// NGSI-LD clauses 5.2.2, 5.2.5-2, 5.2.6-2, 5.2.7-2.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AttributeOutputMembers {
    /// `createdAt`.
    pub created_at: Option<DateTime>,
    /// `modifiedAt`.
    pub modified_at: Option<DateTime>,
    /// `deletedAt`.
    pub deleted_at: Option<DateTime>,
    /// `instanceId`, only in temporal representations (clause 4.5.7).
    pub instance_id: Option<InstanceId>,
}

/// The members of an entity only a broker writes.
///
/// NGSI-LD clause 5.2.2.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EntityOutputMembers {
    /// `createdAt`.
    pub created_at: Option<DateTime>,
    /// `modifiedAt`.
    pub modified_at: Option<DateTime>,
    /// `deletedAt`.
    pub deleted_at: Option<DateTime>,
}

/// One change a fragment makes to an attribute.
///
/// NGSI-LD clauses 5.4, 5.5.8 and 5.5.12.
///
/// `X` is the deletion the fragment can write: a [`Deletion`] in an entity fragment, where an attribute is a member of
/// the entity; an [`InstanceDeletion`] in the attribute fragment of Partial Attribute Update (clause 6.7.3.1), whose
/// body is an object.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum Change<T, X = Deletion> {
    /// Write the attribute: add it, or replace (or merge into) the instance with the same dataset id.
    Set(T),
    /// Delete the attribute, or one instance of it, by writing the NGSI-LD Null.
    Delete(X),
}

/// A deletion in a fragment: the NGSI-LD Null.
///
/// The Null is the one of NGSI-LD clause 4.5.0, written in the shape the attribute kind needs.
///
/// The caller never spells `urn:ngsi-ld:null` nor picks the shape: the representation writes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::IsVariant, derive_more::From)]
pub enum Deletion {
    /// The default instance of an attribute of any kind, written as the bare `"urn:ngsi-ld:null"` member value
    /// clauses 4.5.3.3 and 4.5.18.3 give for fragments, and clause 5.4 example 2 uses for a sub-attribute. It is the
    /// whole member, so it never stands among other instances of the attribute.
    #[from(skip)]
    Member,
    /// An instance of an attribute of a known kind.
    Instance(InstanceDeletion),
}

impl Deletion {
    /// The dataset id of the deleted instance, if one is named.
    #[must_use]
    pub const fn dataset_id(&self) -> Option<&DatasetId> {
        match self {
            Self::Member => None,
            Self::Instance(instance) => instance.dataset_id.as_ref(),
        }
    }
}

/// The deletion of one instance of an attribute of a known kind.
///
/// It is written `{"type": kind, <payload>: NGSI-LD Null}`, with the dataset id when one instance among several is
/// meant (NGSI-LD clauses 4.5.0 and 5.5.8).
///
/// Clauses 4.5.2.2, 4.5.3.2 and their counterparts let the Null stand as the payload of a partial update patch or a
/// merge patch to delete the attribute; the shape of the Null follows the kind: `"urn:ngsi-ld:null"`,
/// `["urn:ngsi-ld:null"]` for the list kinds, `{"@none": "urn:ngsi-ld:null"}` for a language map.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{
///     Attribute,
///     AttributeKind,
///     DatasetId,
///     InstanceDeletion,
///     Lossless,
///     Property,
/// };
///
/// let gps = InstanceDeletion::new(AttributeKind::Relationship)
///     .with_dataset_id(DatasetId::from(iri!("urn:ngsi-ld:dataset:gps")));
/// let level = Attribute::from(Property::builder(4).build()?)
///     .into_patch()
///     .delete_sub_attribute("hasWeatherObserved", gps)?;
/// assert_eq!(
///     level.render(Lossless::Normalized).to_string(),
///     concat!(
///         r#"{"type":"Property","value":4,"#,
///         r#""hasWeatherObserved":{"type":"Relationship","#,
///         r#""object":"urn:ngsi-ld:null","#,
///         r#""datasetId":"urn:ngsi-ld:dataset:gps"}}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceDeletion {
    /// The kind of the attribute, which decides the shape of the NGSI-LD Null.
    pub kind: AttributeKind,
    /// The instance to delete; `None` for the default instance.
    pub dataset_id: Option<DatasetId>,
}

impl InstanceDeletion {
    /// The deletion of the default instance of an attribute of `kind`.
    #[must_use]
    pub const fn new(kind: AttributeKind) -> InstanceDeletion {
        InstanceDeletion { kind, dataset_id: None }
    }

    /// The same deletion aimed at the instance `dataset_id` names.
    #[must_use]
    pub fn with_dataset_id(self, dataset_id: impl Into<DatasetId>) -> Self {
        Self {
            dataset_id: Some(dataset_id.into()),
            ..self
        }
    }
}

impl<T: InstanceKey> InstanceSealed for Change<T> {}

impl<T: InstanceKey> InstanceKey for Change<T> {
    fn dataset_id(&self) -> Option<&DatasetId> {
        match self {
            Self::Set(attribute) => attribute.dataset_id(),
            Self::Delete(deletion) => deletion.dataset_id(),
        }
    }

    fn excludes_other_instances(&self, _: Seal) -> bool {
        match self {
            Self::Set(attribute) => attribute.excludes_other_instances(Seal::new()),
            Self::Delete(Deletion::Member) => true,
            Self::Delete(Deletion::Instance(_)) => false,
        }
    }
}

impl<T: AttributeSlot> AttributeSlot for Change<T> {
    fn written_kind(&self, _: Seal) -> Option<AttributeKind> {
        match self {
            Self::Set(attribute) => attribute.written_kind(Seal::new()),
            Self::Delete(Deletion::Member) => None,
            Self::Delete(Deletion::Instance(instance)) => Some(instance.kind),
        }
    }
}

// `derive_more::From` on `Change<T>` would generate `From<T>` for every `T`, which overlaps `From<Deletion>` when
// `T` is `Deletion`; the two conversions are therefore written for the one slot type that uses them.
impl From<Attribute<Patch>> for Change<Attribute<Patch>> {
    fn from(attribute: Attribute<Patch>) -> Self {
        Self::Set(attribute)
    }
}

// See the conversion above for why this is not derived.
impl From<Deletion> for Change<Attribute<Patch>> {
    fn from(deletion: Deletion) -> Self {
        Self::Delete(deletion)
    }
}

// See the conversions above for why this is not derived: the body of Partial Attribute Update is an attribute, or the
// deletion of one instance.
impl From<Attribute<Patch>> for Change<Attribute<Patch>, InstanceDeletion> {
    fn from(attribute: Attribute<Patch>) -> Self {
        Self::Set(attribute)
    }
}

// See the conversions above for why this is not derived.
impl From<InstanceDeletion> for Change<Attribute<Patch>, InstanceDeletion> {
    fn from(deletion: InstanceDeletion) -> Self {
        Self::Delete(deletion)
    }
}
