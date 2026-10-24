//! The change a fragment makes to one member of a document (NGSI-LD clause 5.4).

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping},
};
use crate::model::{MemberProblem, null::NGSI_LD_NULL};
use jstrict::Value as Json;

/// What a fragment does to one member of the document it changes: replaces the member's value, or deletes the member
/// with the NGSI-LD Null (NGSI-LD clause 5.4).
///
/// Clause 5.4 knows no third change, so the enum is exhaustive.
///
/// ```
/// use ngsi_ld::model::MemberChange;
///
/// let throttling = MemberChange::Set(5);
/// assert_eq!(throttling.as_ref(), MemberChange::Set(&5));
/// assert!(MemberChange::<u32>::Delete.is_delete());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::IsVariant)]
pub enum MemberChange<T> {
    /// The member takes this value.
    Set(T),
    /// The member is removed: the fragment writes the NGSI-LD Null for it.
    Delete,
}

impl<T> MemberChange<T> {
    /// Borrows the value of a set member.
    #[must_use]
    pub const fn as_ref(&self) -> MemberChange<&T> {
        match self {
            Self::Set(value) => MemberChange::Set(value),
            Self::Delete => MemberChange::Delete,
        }
    }

    /// Converts the value of a set member with `convert`.
    #[must_use]
    pub fn map<U>(self, convert: impl FnOnce(T) -> U) -> MemberChange<U> {
        match self {
            Self::Set(value) => MemberChange::Set(convert(value)),
            Self::Delete => MemberChange::Delete,
        }
    }

    /// Writes the change: the JSON `value` writes for a set member, which may write nothing, or the NGSI-LD Null for
    /// a deleted one.
    pub(crate) fn write(&self, value: impl FnOnce(&T) -> Option<Json>) -> Option<Json> {
        match self {
            Self::Set(set) => value(set),
            Self::Delete => Some(Json::from(NGSI_LD_NULL.as_str())),
        }
    }
}

/// Whether a member is deleted.
pub(crate) const fn deleted<T>(change: Option<&MemberChange<T>>) -> bool {
    matches!(change, Some(MemberChange::Delete))
}

/// Deletes a member of a fragment, recording [`MemberProblem::SetAndDeleted`] when the fragment also sets it.
pub(crate) fn delete_member<T, M, P: From<MemberProblem<M>>>(slot: &mut Option<MemberChange<T>>, member: M, problems: &mut Vec<P>) {
    if matches!(slot, Some(MemberChange::Set(_))) {
        problems.push(P::from(MemberProblem::SetAndDeleted { member }));
    }
    *slot = Some(MemberChange::Delete);
}

/// Generates, on a fragment, the two methods over its optional members from one table of `Member::Variant =>
/// field` pairs after `slots:`: `deletes(&self, member) -> bool`, public, and `delete(&mut self, member, problems)`,
/// which records a member the fragment also sets as [`MemberProblem::SetAndDeleted`] of the field the member
/// converts into. The members the fragment keeps in a part of its own are listed after `delegate:`, with that
/// part's field, which provides the same two methods. The generated `match`es list every variant, so a new member
/// is one more row.
macro_rules! member_slots {
    (
        impl $(<$($param:ident $(: $bound:path)?),+>)? for $fragment:ty;
        member: $member:ident, field: $field:ty, problem: $problem:ty;
        slots: $($variant:ident => $slot:ident),+ $(,)?;
        delegate: $($delegated:ident)|+ => $part:ident;
    ) => {
        impl$(<$($param $(: $bound)?),+>)? $fragment {
            /// Whether the fragment deletes `member`.
            #[must_use]
            pub fn deletes(&self, member: $member) -> bool {
                match member {
                    $($member::$variant => $crate::model::member_change::deleted(self.$slot.as_ref()),)+
                    $($member::$delegated)|+ => self.$part.deletes($crate::seal::Seal::new(), member),
                }
            }

            /// Deletes `member`, recording a member the fragment also sets.
            fn delete(&mut self, member: $member, problems: &mut Vec<$problem>) {
                match member {
                    $($member::$variant => $crate::model::member_change::delete_member(&mut self.$slot, <$field>::from(member), problems),)+
                    $($member::$delegated)|+ => self.$part.delete($crate::seal::Seal::new(), member, problems),
                }
            }
        }
    };
    (
        impl $(<$($param:ident $(: $bound:path)?),+>)? for $fragment:ty;
        member: $member:ident, field: $field:ty, problem: $problem:ty;
        slots: $($variant:ident => $slot:ident),+ $(,)?;
    ) => {
        impl$(<$($param $(: $bound)?),+>)? $fragment {
            /// Whether the fragment deletes `member`.
            #[must_use]
            pub const fn deletes(&self, member: $member) -> bool {
                match member {
                    $($member::$variant => $crate::model::member_change::deleted(self.$slot.as_ref()),)+
                }
            }

            /// Deletes `member`, recording a member the fragment also sets.
            fn delete(&mut self, member: $member, problems: &mut Vec<$problem>) {
                match member {
                    $($member::$variant => $crate::model::member_change::delete_member(&mut self.$slot, <$field>::from(member), problems),)+
                }
            }
        }
    };
}

#[cfg(feature = "linked-data")]
impl<T: MapNames> MapNames for MemberChange<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Set(value) => value.map_names(mapping).map(Self::Set),
            Self::Delete => Ok(Self::Delete),
        }
    }
}

#[cfg(test)]
mod tests;
