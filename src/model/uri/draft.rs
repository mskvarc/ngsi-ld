//! URLs given to a builder: a [`Url`], or text validated when the builder finishes.

use crate::model::{MemberProblem, written_url::WrittenUrl};
use url::{ParseError, Url};

/// A URL given to a builder.
///
/// It is a [`Url`], or text validated when the builder finishes, so every invalid input is reported together. A
/// URL is written in its normalised form (`http://localhost:8080` becomes `http://localhost:8080/`); a draft made by
/// [`UrlDraft::verbatim`] is written as it was given.
///
/// ```
/// use ngsi_ld::model::UrlDraft;
/// use url::Url;
///
/// let parsed = UrlDraft::from(Url::parse("http://my.endpoint.org/notify")?);
/// let text = UrlDraft::from("http://my.endpoint.org/notify");
/// let as_given = UrlDraft::verbatim("http://my.endpoint.org");
/// # let _ = (parsed, text, as_given);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct UrlDraft(Result<WrittenUrl, ParseError>);

impl UrlDraft {
    /// Text validated when the builder finishes, and written as given: no trailing `/` is added to a URL that is
    /// only an origin. The URL the finished document returns is the normalised one, and it equals the one built
    /// from the normalised text.
    #[must_use]
    pub fn verbatim(text: &str) -> Self {
        Self(WrittenUrl::verbatim(text))
    }

    /// Returns the URL, or records why the text given for `member` is not one.
    pub(crate) fn resolve<M, P: From<MemberProblem<M>>>(self, member: M, problems: &mut Vec<P>) -> Option<WrittenUrl> {
        self.0
            .map_err(|source| problems.push(P::from(MemberProblem::InvalidUrl { member, source })))
            .ok()
    }
}

// No derive: the conversion wraps the value into the draft's `Result`.
impl From<Url> for UrlDraft {
    fn from(url: Url) -> Self {
        Self(Ok(WrittenUrl::from(url)))
    }
}

// No derive: the conversion parses the text into the draft's `Result`.
impl From<&str> for UrlDraft {
    fn from(text: &str) -> Self {
        Self(text.parse::<Url>().map(WrittenUrl::from))
    }
}

// No derive: the conversion parses the text into the draft's `Result`.
impl From<String> for UrlDraft {
    fn from(text: String) -> Self {
        Self::from(text.as_str())
    }
}
