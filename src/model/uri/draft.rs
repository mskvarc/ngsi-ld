//! URLs given to a builder: a [`Url`], or text validated when the builder finishes.

use crate::{
    model::{MemberProblem, draft::Draft},
    seal::Seal,
};
use url::Url;

draft!(
    /// A URL given to a builder.
    ///
    /// It is a [`Url`], or text validated when the builder finishes, so every invalid input is reported together.
    ///
    /// ```
    /// use ngsi_ld::model::UrlDraft;
    /// use url::Url;
    ///
    /// let parsed = UrlDraft::from(Url::parse("http://my.endpoint.org/notify")?);
    /// let text = UrlDraft::from("http://my.endpoint.org/notify");
    /// # let _ = (parsed, text);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    UrlDraft(Url, url::ParseError)
);

impl UrlDraft {
    /// Returns the URL, or records why the text given for `member` is not one.
    pub(crate) fn resolve<M, P: From<MemberProblem<M>>>(self, member: M, problems: &mut Vec<P>) -> Option<Url> {
        self.into_result(Seal::new())
            .map_err(|source| problems.push(P::from(MemberProblem::InvalidUrl { member, source })))
            .ok()
    }
}
