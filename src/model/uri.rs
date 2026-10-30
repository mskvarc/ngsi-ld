//! URIs, as NGSI-LD uses them to name and reference resources (NGSI-LD clauses 4.6.3 and 5.2.1).
//!
//! Clause 5.2.1 counts IRIs (IETF RFC 3987) as URIs. In practice an identifier is a URN
//! (`urn:ngsi-ld:Vehicle:A4567`) or a URL (`https://example.org/vehicles/1`), and occasionally an IRI with
//! characters beyond ASCII. Each of the three has its own normalisation rules, so a [`Uri`] keeps the form it
//! was recognised as and compares by that form's rules.

use iri_rs::{Iri, IriBuf};
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};
use url::Url;
use urn_rs::Urn;

pub(super) mod draft;

/// The string is not a URI.
///
/// It does not follow the grammar of IETF RFC 3986 (or RFC 3987 for non-ASCII input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("not a valid URI or IRI")]
pub struct UriError;

/// A URI or IRI.
///
/// The string must follow the URI grammar of IETF RFC 3986, or the IRI grammar of RFC 3987 when it has
/// non-ASCII characters; nothing is parsed into a more specific form before the whole string passes. Within
/// that grammar the value is held as:
///
/// - a URN (IETF RFC 8141) when it is one, so the case of the `urn:` prefix and the namespace identifier do
///   not distinguish two values;
/// - otherwise a URL when it is all ASCII, normalised the way the WHATWG URL standard normalises scheme and host;
/// - otherwise an IRI, kept verbatim, because URL parsing would percent-encode its non-ASCII characters.
///
/// ```
/// use ngsi_ld::model::Uri;
///
/// let urn: Uri = "urn:ngsi-ld:Vehicle:A4567".parse()?;
/// assert!(urn.as_urn().is_some());
/// let url: Uri = "https://example.org/vehicles/1".parse()?;
/// assert!(url.as_url().is_some());
/// assert!("Vehicle A4567".parse::<Uri>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// A URI built with [`Uri::verbatim`] writes the text it was given, so a caller can send a spelling the
/// normalisation would change; it still equals, hashes and orders as the same URI built from the canonical text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[display("{}", self.as_str())]
pub struct Uri(Form);

/// The form a [`Uri`] was recognised as.
#[derive(Debug, Clone)]
enum Form {
    /// A URN.
    Urn(Urn),
    /// An all-ASCII URL.
    Url(Url),
    /// A non-ASCII IRI, kept verbatim.
    Iri(IriBuf),
    /// A form written as the caller gave it. It is a variant of its own, not a field of the URI, so that the URI
    /// keeps the size it has without it: URIs travel in the `Err` variant of every reader.
    Spelled(Box<Spelled>),
}

/// A form together with the text it was built from.
#[derive(Debug, Clone)]
struct Spelled {
    /// The form the text is recognised as, never itself `Spelled`.
    form: Form,
    /// The text, which differs from the form's own.
    text: Box<str>,
}

// A spelling is not part of the value: equality, hash and order are those of the form beneath it. No derive can
// look through the `Spelled` variant.
impl PartialEq for Form {
    fn eq(&self, other: &Self) -> bool {
        match (self.recognised(), other.recognised()) {
            (Self::Urn(left), Self::Urn(right)) => left == right,
            (Self::Url(left), Self::Url(right)) => left == right,
            (Self::Iri(left), Self::Iri(right)) => left == right,
            (Self::Urn(_) | Self::Url(_) | Self::Iri(_) | Self::Spelled(_), Self::Urn(_) | Self::Url(_) | Self::Iri(_) | Self::Spelled(_)) => false,
        }
    }
}

// Reflexive, symmetric and transitive, as `PartialEq` compares the forms beneath.
impl Eq for Form {}

// Consistent with `PartialEq`: the form beneath is hashed, so a spelling does not change the hash.
impl Hash for Form {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.recognised() {
            Self::Urn(urn) => urn.hash(state),
            Self::Url(url) => url.hash(state),
            Self::Iri(iri) => iri.hash(state),
            Self::Spelled(spelled) => spelled.form.hash(state),
        }
    }
}

impl Form {
    /// The form without its spelling.
    /// The text the form writes.
    fn text(&self) -> &str {
        match self {
            Self::Urn(urn) => urn.as_str(),
            Self::Url(url) => url.as_str(),
            Self::Iri(iri) => iri.as_str(),
            Self::Spelled(spelled) => &spelled.text,
        }
    }

    const fn recognised(&self) -> &Self {
        match self {
            Self::Spelled(spelled) => &spelled.form,
            Self::Urn(_) | Self::Url(_) | Self::Iri(_) => self,
        }
    }

    /// Picks the most specific form for a string the URI or IRI grammar has already accepted.
    fn of_well_formed(iri: Iri<&str>) -> Self {
        let value = iri.as_str();
        if let Ok(urn) = value.parse::<Urn>() {
            return Self::Urn(urn);
        }
        if value.is_ascii()
            && let Ok(url) = value.parse::<Url>()
        {
            return Self::Url(url);
        }
        Self::Iri(iri.into())
    }
}

impl Uri {
    /// Parses `text` like [`FromStr`], and keeps the text so that it is written back as given.
    ///
    /// The text is validated exactly as a parsed one is. The URI equals, hashes and orders as the one parsed from
    /// `text`; only its written form differs, where the normalisation would have changed the text (the case of a
    /// scheme or host, for instance).
    ///
    /// ```
    /// use ngsi_ld::model::Uri;
    ///
    /// let verbatim = Uri::verbatim("URN:ngsi-ld:Vehicle:A4567")?;
    /// assert_eq!(verbatim.as_str(), "URN:ngsi-ld:Vehicle:A4567");
    /// assert_eq!(verbatim, "urn:ngsi-ld:Vehicle:A4567".parse()?);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`UriError`] when `text` is not a URI or IRI.
    pub fn verbatim(text: &str) -> Result<Self, UriError> {
        let Self(form) = text.parse()?;
        if form.text() == text {
            Ok(Self(form))
        } else {
            Ok(Self(Form::Spelled(Box::new(Spelled { form, text: text.into() }))))
        }
    }

    /// Returns the URI as written: the text given to [`Uri::verbatim`], otherwise the form's normalised text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.text()
    }

    /// Returns the URN, when the URI is one.
    #[must_use]
    pub const fn as_urn(&self) -> Option<&Urn> {
        match self.0.recognised() {
            Form::Urn(urn) => Some(urn),
            Form::Url(_) | Form::Iri(_) | Form::Spelled(_) => None,
        }
    }

    /// Returns the URL, when the URI was recognised as one.
    #[must_use]
    pub const fn as_url(&self) -> Option<&Url> {
        match self.0.recognised() {
            Form::Url(url) => Some(url),
            Form::Urn(_) | Form::Iri(_) | Form::Spelled(_) => None,
        }
    }
}

// `derive_more::AsRef` targets one field; the three forms each spell their accessor differently.
impl AsRef<str> for Uri {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

// No derive picks the form a parsed string is held as.
impl FromStr for Uri {
    type Err = UriError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Iri::parse(value).map(|iri| Self(Form::of_well_formed(iri))).map_err(|_| UriError)
    }
}

// No derive parses a string through `FromStr`.
impl TryFrom<String> for Uri {
    type Error = UriError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

// No derive parses a string through `FromStr`.
impl TryFrom<&str> for Uri {
    type Error = UriError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// A literal checked at compile time by `iri_rs::iri!` is already well formed; it gets the same form a parsed
/// string would, which no derive picks.
impl From<Iri<&str>> for Uri {
    fn from(iri: Iri<&str>) -> Self {
        Self(Form::of_well_formed(iri))
    }
}

// No derive wraps a value two levels deep, into the private form.
impl From<Urn> for Uri {
    fn from(urn: Urn) -> Self {
        Self(Form::Urn(urn))
    }
}

/// A WHATWG URL can hold what the URI grammar refuses (a literal space in an opaque path), so it is checked, which
/// no derive does.
impl TryFrom<Url> for Uri {
    type Error = UriError;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        Iri::parse(url.as_str()).map_err(|_| UriError)?;
        Ok(Self(Form::Url(url)))
    }
}

// The wire form is the display form; serde needs an owned `String` to serialise through `into`, and no derive
// converts through `Display`.
impl From<Uri> for String {
    fn from(uri: Uri) -> Self {
        uri.to_string()
    }
}

#[cfg(test)]
mod tests;
