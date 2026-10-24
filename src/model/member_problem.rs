//! The problems every document family meets in its members: an invalid attribute name, entity type, URL or positive
//! number, an empty or missing member, and a member a fragment both sets and deletes. Each family's problem holds
//! them for its own members, `M`.

use crate::{
    model::{NameError, NotPositive},
    query::type_selection::SelectableTypeError,
};

/// A problem with the member `member` of a document, found by a builder or a reader.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification restrict more members.
///
/// ```
/// use ngsi_ld::{model::MemberProblem, subscription::{Problem, Endpoint, EndpointMember}};
///
/// let error = Endpoint::builder("not a url").build().err();
/// let problems = error.as_ref().map(|error| error.problems());
/// assert!(matches!(
///     problems,
///     Some([Problem::Member(MemberProblem::InvalidUrl { member, .. })]) if *member == EndpointMember::Uri.into()
/// ));
/// ```
// `url::ParseError` carries no `Hash`, so neither can the problem that chains it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MemberProblem<M> {
    /// An element of a list of attribute names is not an attribute name.
    #[error("element {index} of `{member}` is not a valid attribute name")]
    InvalidAttributeName {
        /// The member.
        member: M,
        /// The position of the element, counting from 0.
        index: usize,
        /// Why the name is invalid.
        #[source]
        source: NameError,
    },
    /// An element of a list of entity selectors names an entity type a type selection cannot hold.
    #[error("element {index} of `{member}` names no entity type a selector can hold")]
    InvalidSelectedType {
        /// The member.
        member: M,
        /// The position of the element, counting from 0.
        index: usize,
        /// Why the entity type is invalid.
        #[source]
        source: SelectableTypeError,
    },
    /// An element of a list of entities names no valid entity type.
    #[error("element {index} of `{member}` is not a valid entity type name")]
    InvalidEntityTypeName {
        /// The member.
        member: M,
        /// The position of the element, counting from 0.
        index: usize,
        /// Why the name is invalid.
        #[source]
        source: NameError,
    },
    /// The member is not a URL where a dereferenceable URI is needed.
    #[error("`{member}` is not a valid URL")]
    InvalidUrl {
        /// The member.
        member: M,
        /// Why the URL is invalid.
        #[source]
        source: url::ParseError,
    },
    /// A number is not greater than 0 where the member requires it.
    #[error("`{member}` must be greater than 0")]
    NotPositive {
        /// The member.
        member: M,
        /// The number refused.
        #[source]
        source: NotPositive,
    },
    /// A list or text the specification forbids to be empty is empty.
    #[error("`{member}` cannot be empty")]
    Empty {
        /// The member.
        member: M,
    },
    /// A member the document requires is missing.
    #[error("missing member `{member}`")]
    Missing {
        /// The member.
        member: M,
    },
    /// A fragment both sets and deletes the member.
    #[error("the fragment both sets and deletes `{member}`")]
    SetAndDeleted {
        /// The member.
        member: M,
    },
}
