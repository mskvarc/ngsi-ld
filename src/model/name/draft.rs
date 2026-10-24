//! Attribute names given to a builder: an [`AttributeName`], or text validated when the builder finishes.

use crate::model::{
    AttributeName,
    MemberProblem,
    NameError,
    draft::{Draft, resolve_drafts, resolve_required_drafts},
};

draft!(
    /// An attribute name given to a builder.
    ///
    /// It is an [`AttributeName`], or text validated when the builder finishes, so every invalid name is reported
    /// together. A list may mix both, each converted with `.into()`.
    ///
    /// ```
    /// use ngsi_ld::model::{AttributeName, AttributeNameDraft};
    ///
    /// let speed: AttributeName = "speed".parse()?;
    /// let names: [AttributeNameDraft; 2] = [speed.into(), "brand".into()];
    /// # let _ = names;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    AttributeNameDraft(AttributeName, NameError)
);

/// Resolves the names given for `member` in order, recording each invalid one with its position.
pub(crate) fn resolve_names<D: Draft<Error = NameError>, M: Copy, P: From<MemberProblem<M>>>(
    member: M,
    names: impl IntoIterator<Item = D>,
    problems: &mut Vec<P>,
) -> Vec<D::Value> {
    resolve_drafts(names, problems, |index, source| {
        P::from(MemberProblem::InvalidAttributeName { member, index, source })
    })
}

/// Resolves the names of a list the specification forbids to be empty: absent, it is empty with no problem; given
/// empty, [`MemberProblem::Empty`] is recorded.
pub(crate) fn resolve_required_names<D: Draft<Error = NameError>, M: Copy, P: From<MemberProblem<M>>>(
    member: M,
    names: Option<Vec<D>>,
    problems: &mut Vec<P>,
) -> Vec<D::Value> {
    resolve_required_drafts(member, names, problems, |index, source| {
        P::from(MemberProblem::InvalidAttributeName { member, index, source })
    })
}
