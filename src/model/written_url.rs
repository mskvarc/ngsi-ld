//! A URL together with the text it was built from.

use crate::model::written::Written;
use url::Url;

/// A [`Url`] that remembers the text it was built from when that text differs from the normalised URL.
///
/// A [`Url`] cannot give its source back (`http://localhost:8080` becomes `http://localhost:8080/`), so a member
/// that must be written as the caller gave it holds this instead. Equality, hash and order are those of the URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct WrittenUrl {
    url: Url,
    written: Written,
}

impl WrittenUrl {
    /// Holds `url`, written in its normalised form.
    pub(crate) const fn canonical(url: Url) -> Self {
        Self { url, written: Written::none() }
    }

    /// Parses `text`, and keeps it when the URL is written differently.
    pub(crate) fn verbatim(text: &str) -> Result<Self, url::ParseError> {
        let url: Url = text.parse()?;
        let written = Written::of(text, url.as_str());
        Ok(Self { url, written })
    }

    /// Returns the URL.
    pub(crate) const fn url(&self) -> &Url {
        &self.url
    }

    /// Returns the text to write: the one given to [`WrittenUrl::verbatim`], otherwise the normalised URL.
    pub(crate) fn as_str(&self) -> &str {
        self.written.as_deref().unwrap_or_else(|| self.url.as_str())
    }
}

impl From<Url> for WrittenUrl {
    fn from(url: Url) -> Self {
        Self::canonical(url)
    }
}

#[cfg(test)]
mod tests {
    use super::WrittenUrl;

    #[test]
    fn a_verbatim_url_writes_its_text_and_equals_the_normalised_one() {
        let verbatim = WrittenUrl::verbatim("http://localhost:8080").unwrap();
        assert_eq!(verbatim.as_str(), "http://localhost:8080");
        assert_eq!(verbatim.url().as_str(), "http://localhost:8080/");
        assert_eq!(verbatim, WrittenUrl::verbatim("http://localhost:8080/").unwrap());
    }

    #[test]
    fn a_verbatim_url_that_is_already_normalised_keeps_nothing() {
        assert_eq!(WrittenUrl::verbatim("http://localhost:8080/").unwrap().as_str(), "http://localhost:8080/");
        assert!(WrittenUrl::verbatim("not a url").is_err());
    }
}
