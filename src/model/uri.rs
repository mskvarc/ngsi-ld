//! URIs, as NGSI-LD uses them to name and reference resources (NGSI-LD clauses 4.6.3 and 5.2.1).
//!
//! Clause 5.2.1 counts IRIs (IETF RFC 3987) as URIs. In practice an identifier is a URN
//! (`urn:ngsi-ld:Vehicle:A4567`) or a URL (`https://example.org/vehicles/1`), and occasionally an IRI with
//! characters beyond ASCII. A [`Uri`] keeps the form it was recognised as, and compares the way clause 5.5.1
//! asks, as IETF RFC 3986 section 6 recommends: a URN by the equivalence of RFC 8141 section 3, anything else by
//! the URL its syntax-based normal form maps to.

use iri_rs::{Iri, IriBuf, UriBuf};
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
/// - a URN (IETF RFC 8141) when it is one, so the case of the `urn:` prefix and the namespace identifier and the
///   case of percent-encoding hex digits do not distinguish two values;
/// - otherwise in its RFC 3987 section 5.3.2 syntax-based normal form (scheme and host in lowercase,
///   percent-encoded unreserved characters decoded, other escapes in uppercase hex, dot segments removed), and
///   compared by the URL its RFC 3987 section 3.1 URI (every non-ASCII character as percent-encoded UTF-8)
///   parses to under the WHATWG URL standard, which also drops a default port and maps the host of an `http` or
///   `https` URL to its IDNA form. A URI with non-ASCII characters is written as the IRI, not as the URL, so
///   `https://example.org/Par%C3%ADs` and `https://example.org/París` are one URI written two ways (RFC 3987
///   section 5.3.2.3).
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
/// normalisation would change; it still equals and hashes as the same URI built from the canonical text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[display("{}", self.as_str())]
pub struct Uri(Form);

/// The form a [`Uri`] was recognised as.
#[derive(Debug, Clone)]
enum Form {
    /// A URN.
    Urn(Urn),
    /// A URL whose text is all ASCII.
    Url(Url),
    /// An IRI the URL form does not write as it is: one with non-ASCII characters, or one the WHATWG URL standard
    /// refuses. Boxed, as `Spelled` is, so that the URI keeps the size of its URL form.
    Iri(Box<IriForm>),
    /// A form written as the caller gave it. It is a variant of its own, not a field of the URI, so that the URI
    /// keeps the size it has without it: URIs travel in the `Err` variant of every reader.
    Spelled(Box<Spelled>),
}

/// An IRI held for its text, with what it is compared by.
#[derive(Debug, Clone)]
struct IriForm {
    /// The IRI in its syntax-based normal form.
    text: IriBuf,
    /// What the URI the normal form maps to is compared by.
    mapped: MappedUri,
}

/// The URI an IRI maps to, as it is compared.
#[derive(Debug, Clone)]
enum MappedUri {
    /// The URL the WHATWG URL standard parses the URI to.
    Url(Url),
    /// The URI itself, which the WHATWG URL standard refuses (a port beyond 65535, for instance).
    Uri(UriBuf),
}

impl MappedUri {
    /// What the RFC 3987 section 3.1 URI of `iri` is compared by.
    fn of(iri: &IriBuf) -> Self {
        let uri = iri.to_uri();
        uri.as_str()
            .parse::<Url>()
            .map_or_else(|_| Self::Uri(UriBuf::parse_unchecked(uri.as_str().to_owned())), Self::Url)
    }
}

/// A form together with the text it was built from.
#[derive(Debug, Clone)]
struct Spelled {
    /// The form the text is recognised as, never itself `Spelled`.
    form: Form,
    /// The text, which differs from the form's own.
    text: Box<str>,
}

/// What a form is compared and hashed by: an IRI and the URL it maps to are one identity, and a spelling is not
/// part of it.
#[derive(PartialEq, Eq, Hash)]
enum Identity<'a> {
    /// A URN, by RFC 8141 equivalence.
    Urn(&'a Urn),
    /// A URL, or an IRI by the URL its URI parses to.
    Url(&'a Url),
    /// An IRI by its URI, when no URL holds that.
    Uri(&'a UriBuf),
}

// No derive compares a form through the identity it maps to.
impl PartialEq for Form {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

// Reflexive, symmetric and transitive, as `PartialEq` compares identities.
impl Eq for Form {}

// Consistent with `PartialEq`: the identity is hashed.
impl Hash for Form {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identity().hash(state);
    }
}

impl Form {
    /// The text the form writes.
    fn text(&self) -> &str {
        match self {
            Self::Urn(urn) => urn.as_str(),
            Self::Url(url) => url.as_str(),
            Self::Iri(iri) => iri.text.as_str(),
            Self::Spelled(spelled) => &spelled.text,
        }
    }

    /// The form without its spelling.
    const fn recognised(&self) -> &Self {
        match self {
            Self::Spelled(spelled) => &spelled.form,
            Self::Urn(_) | Self::Url(_) | Self::Iri(_) => self,
        }
    }

    /// What the form is compared and hashed by.
    fn identity(&self) -> Identity<'_> {
        match self.recognised() {
            Self::Urn(urn) => Identity::Urn(urn),
            Self::Url(url) => Identity::Url(url),
            Self::Iri(iri) => match &iri.mapped {
                MappedUri::Url(url) => Identity::Url(url),
                MappedUri::Uri(uri) => Identity::Uri(uri),
            },
            Self::Spelled(spelled) => spelled.form.identity(),
        }
    }

    /// Picks the most specific form for a string the URI or IRI grammar has already accepted.
    ///
    /// A URN keeps its own equivalence (RFC 8141 section 3 normalises the case of an escape and decodes none).
    /// Anything else is normalised before the URL parse, which does not decode an escaped unreserved character or
    /// remove an escaped dot segment by itself.
    fn of_well_formed(iri: Iri<&str>) -> Self {
        if let Ok(urn) = iri.as_str().parse::<Urn>() {
            return Self::Urn(urn);
        }
        let text = iri.normalized();
        match MappedUri::of(&text) {
            MappedUri::Url(url) if text.as_str().is_ascii() => Self::Url(url),
            mapped @ (MappedUri::Url(_) | MappedUri::Uri(_)) => Self::Iri(Box::new(IriForm { text, mapped })),
        }
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

/// A WHATWG URL can hold what the URI grammar refuses (a literal space in an opaque path), so it is checked, and
/// it gets the form its text would, a URN included, which no derive does.
impl TryFrom<Url> for Uri {
    type Error = UriError;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        url.as_str().parse()
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
