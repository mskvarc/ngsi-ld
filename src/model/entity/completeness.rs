//! Whether a document holds an entity's identifying members, `id` and `type`, or may lack them: because a
//! projection removed them (NGSI-LD clauses 4.21 and 6.4.3.2), or because the document is a fragment whose
//! entity the request path names (clause 5.4).

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, member},
};
use crate::seal::Seal;
use std::fmt::Debug;

/// The private supertrait that keeps `Completeness` implemented by this crate alone.
mod sealed {
    /// Seals [`Completeness`](super::Completeness): an entity either holds its identifying members or may lack
    /// them.
    pub trait Sealed {}
}

/// What an entity holds of its `id` and `type`: [`Complete`] or [`Projected`].
///
/// The `pick` and `omit` parameters (Tables 6.4.3.2-1 and 6.5.3.1-1) name the entity members `id` and `type`
/// like any attribute: `pick` keeps only what it names, `omit` removes what it names. A response to such a
/// projection is still a list of entities, but an entity of it may lack `id`, `type` or both. A fragment sent to
/// one entity may lack them too, while a fragment in a batch must hold them (clauses 5.6.9.3 and 5.6.20.3). Every
/// entity and fragment type is therefore parameterised by a [`Completeness`]: [`Complete`] holds both members,
/// [`Projected`] holds each one only if the document carries it.
///
/// The trait is sealed; its associated type is the field type of an identifying member.
pub trait Completeness: sealed::Sealed + Debug + Clone + Copy + PartialEq + Eq + Default + 'static {
    /// How an identifying member of type `T` is held: `T` itself, or an `Option<T>`; shareable across threads
    /// whenever `T` is, so a request holding one can be sent from any task.
    type Member<T: Debug + Clone + PartialEq + Eq + Send + Sync>: Debug + Clone + PartialEq + Eq + Send + Sync;

    /// The member, if the entity holds it.
    #[doc(hidden)]
    fn get<T: Debug + Clone + PartialEq + Eq + Send + Sync>(seal: Seal, member: &Self::Member<T>) -> Option<&T>;

    /// The member as read from a document: `None` when the document lacks a member this completeness requires.
    #[doc(hidden)]
    fn from_read<T: Debug + Clone + PartialEq + Eq + Send + Sync>(seal: Seal, member: Option<T>) -> Option<Self::Member<T>>;
}

/// A document that holds its `id` and its `type`: every entity unless a projection removed them, and every
/// fragment in a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Complete;

/// A document whose `id` and `type` are each present only if it carries them: an entity a projection may have
/// reduced (`pick` without `id` or `type`, or `omit` with them; Table 6.4.3.2-1), or a fragment of one entity,
/// whose request path names the entity (clause 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Projected;

impl sealed::Sealed for Complete {}
impl sealed::Sealed for Projected {}

impl Completeness for Complete {
    type Member<T: Debug + Clone + PartialEq + Eq + Send + Sync> = T;

    fn get<T: Debug + Clone + PartialEq + Eq + Send + Sync>(_: Seal, member: &T) -> Option<&T> {
        Some(member)
    }

    fn from_read<T: Debug + Clone + PartialEq + Eq + Send + Sync>(_: Seal, member: Option<T>) -> Option<T> {
        member
    }
}

impl Completeness for Projected {
    type Member<T: Debug + Clone + PartialEq + Eq + Send + Sync> = Option<T>;

    fn get<T: Debug + Clone + PartialEq + Eq + Send + Sync>(_: Seal, member: &Option<T>) -> Option<&T> {
        member.as_ref()
    }

    fn from_read<T: Debug + Clone + PartialEq + Eq + Send + Sync>(_: Seal, member: Option<T>) -> Option<Option<T>> {
        Some(member)
    }
}

/// A completeness whose identifying members a [`NameMapping`] rewrites: `type` holds entity type names.
#[cfg(feature = "linked-data")]
pub(crate) trait MapCompleteness: Completeness {
    /// Maps the names an identifying member holds, placing an error under the member `name`.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error at the member.
    fn map_member<T, M>(value: Self::Member<T>, name: &str, mapping: &M) -> Result<Self::Member<T>, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq + Send + Sync,
        M: NameMapping;
}

#[cfg(feature = "linked-data")]
impl MapCompleteness for Complete {
    fn map_member<T, M>(value: T, name: &str, mapping: &M) -> Result<T, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq + Send + Sync,
        M: NameMapping,
    {
        member(value, name, mapping)
    }
}

#[cfg(feature = "linked-data")]
impl MapCompleteness for Projected {
    fn map_member<T, M>(value: Option<T>, name: &str, mapping: &M) -> Result<Option<T>, DocumentError<M::Error>>
    where
        T: MapNames + Debug + Clone + PartialEq + Eq + Send + Sync,
        M: NameMapping,
    {
        member(value, name, mapping)
    }
}
