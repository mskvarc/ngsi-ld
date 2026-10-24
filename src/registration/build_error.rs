//! Why a context source registration, its information, its management information or a fragment of a registration
//! could not be built, and the rules of NGSI-LD Tables 5.2.9-1 and 5.2.10-1 and of clauses 4.3.6.2, 4.3.6.3 and
//! 4.3.6.6 a read document breaks.

use crate::{
    model::MemberProblem,
    registration::{ContextSourceInfoError, ContextSourcePropertyName, ContextSourcePropertyNameError, RegistrationField},
};

/// One rule a registration document breaks, or one invalid input given to its builders.
///
/// `build()` returns every problem at once as a [`BuildError`](crate::model::BuildError) of `Problem`.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members and restrictions.
///
/// ```
/// use ngsi_ld::{
///     model::MemberProblem,
///     registration::{Problem, RegistrationField, RegistrationInfo, RegistrationInfoMember},
/// };
///
/// let error = RegistrationInfo::builder()
///     .property_names(["1x"])
///     .build()
///     .err();
/// let problems = error.as_ref().map(|error| error.problems());
/// let Some(
///     [
///         Problem::Member(MemberProblem::InvalidAttributeName {
///             member, index: 0, ..
///         }),
///     ],
/// ) = problems
/// else {
///     anyhow::bail!("expected one invalid attribute name");
/// };
/// assert_eq!(
///     *member,
///     RegistrationField::from(RegistrationInfoMember::PropertyNames)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Problem {
    /// A member is invalid, empty, missing, or both set and deleted.
    #[error(transparent)]
    Member(#[from] MemberProblem<RegistrationField>),
    /// A `RegistrationInfo` names no entity, Property or Relationship (Table 5.2.10-1).
    #[error("a registration information needs entities, property names or relationship names")]
    EmptyRegistrationInfo,
    /// An `auxiliary` registration does not limit its `operations` to `retrieveOps`, `retrieveEntity` and
    /// `queryEntity` (clauses 4.3.6.2 and 5.9.2.4); without `operations` it supports the default
    /// `federationOps`, which goes beyond them.
    #[error("an auxiliary registration supports only retrieveOps, retrieveEntity and queryEntity")]
    AuxiliaryOperations,
    /// An `exclusive` registration does not name an entity id and attributes in each of its `information`
    /// elements (clause 4.3.6.3).
    #[error("an exclusive registration needs an entity id and attribute names")]
    ExclusiveWithoutEntityIdAndAttributes,
    /// A `contextSourceInfo` pair holds a value its key does not allow (clause 4.3.6.6).
    #[error("context source information {index} is invalid")]
    ContextSourceInfo {
        /// The position of the pair, counting from 0.
        index: usize,
        /// Why.
        #[source]
        source: ContextSourceInfoError,
    },
    /// A Context Source Property given to a builder has a name that cannot name one (clause 4.9).
    #[error("the name of context source property {index} is invalid")]
    ContextSourceProperty {
        /// The position of the property among those given, counting from 0.
        index: usize,
        /// Why.
        #[source]
        source: ContextSourcePropertyNameError,
    },
    /// A Context Source Property is given twice, which one JSON object cannot hold.
    #[error("context source property `{name}` is given twice")]
    DuplicateContextSourceProperty {
        /// The property.
        name: ContextSourcePropertyName,
    },
}
