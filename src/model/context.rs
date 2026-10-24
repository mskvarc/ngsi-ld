//! JSON-LD `@context` values (NGSI-LD clauses 4.4 and 5.2.3).

use crate::model::{InvalidJson, RawJson};
use iri_rs::{InvalidIri, Iri, IriBuf, IriRefBuf, iri};
use jstrict::Value as Json;
use std::{slice, str::FromStr};

/// The URL of the NGSI-LD core context this crate implements, v1.9 (NGSI-LD clause 4.4).
///
/// The core context is implicitly part of every `@context`, as its last entry: a body that must carry an
/// `@context` names it when the client has none of its own (clause 6.3.5).
///
/// ```
/// use ngsi_ld::model::CORE_CONTEXT;
///
/// assert_eq!(
///     CORE_CONTEXT.as_str(),
///     "https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"
/// );
/// ```
pub const CORE_CONTEXT: Iri<&'static str> = iri!("https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld");

/// [`CORE_CONTEXT`] as an IRI reference, the target of a `Link` to the core context (clause 6.3.5); only the HTTP
/// binding writes such a link.
#[cfg(feature = "http")]
pub(crate) fn core_context_reference() -> IriRefBuf {
    // Every IRI is an IRI reference, so the core context's IRI needs no second validation.
    IriRefBuf::parse_unchecked(CORE_CONTEXT.as_str().to_owned())
}

/// One entry of an `@context` (NGSI-LD clause 5.2.3): a reference to a context document, or an inline context
/// definition.
///
/// The enum is exhaustive: Table 5.2.3-1 types an `@context` as a URI, a JSON object or an array of them. JSON-LD
/// also allows `null`, but NGSI-LD does not: clause 4.5.0 rules out `null` as a value anywhere in NGSI-LD.
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::model::ContextEntry;
///
/// let relative = ContextEntry::from(IriRefBuf::new("contexts/vehicle.jsonld")?);
/// assert!(relative.is_reference());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum ContextEntry {
    /// A reference to a context document: an absolute IRI, or an IRI reference resolved against the document's base
    /// (JSON-LD 1.1 section 4.1.7).
    Reference(IriRefBuf),
    /// An inline context definition, a JSON object.
    Inline(InlineContext),
}

/// An inline context definition: a JSON object (JSON-LD 1.1 section 9.15), the only JSON value besides a reference
/// that an `@context` entry may be (Table 5.2.3-1).
///
/// ```
/// use ngsi_ld::model::{InlineContext, InlineContextError};
///
/// let vocabulary: InlineContext = r#"{"@vocab": "https://example.org/terms/"}"#.parse()?;
/// assert_eq!(
///     vocabulary.to_string(),
///     r#"{"@vocab":"https://example.org/terms/"}"#
/// );
/// assert!(matches!(
///     "null".parse::<InlineContext>(),
///     Err(InlineContextError::NotAnObject)
/// ));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{_0}")]
pub struct InlineContext(RawJson);

/// Why a value is not an inline context definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum InlineContextError {
    /// The text is not JSON.
    #[error("an inline context must be JSON")]
    Json(#[from] InvalidJson),
    /// The JSON is not an object: `null`, a scalar or an array, none of which is a context definition.
    #[error("an inline context must be a JSON object")]
    NotAnObject,
}

// The wrapped JSON must be an object, which no derive checks.
impl TryFrom<RawJson> for InlineContext {
    type Error = InlineContextError;

    fn try_from(json: RawJson) -> Result<Self, InlineContextError> {
        if json.is_object() {
            Ok(Self(json))
        } else {
            Err(InlineContextError::NotAnObject)
        }
    }
}

// Parsing goes through the JSON check and the object check, which no derive chains.
impl FromStr for InlineContext {
    type Err = InlineContextError;

    fn from_str(text: &str) -> Result<Self, InlineContextError> {
        Self::try_from(text.parse::<RawJson>()?)
    }
}

/// An `@context` member: one entry or an array of entries (NGSI-LD clause 5.2.3).
///
/// The data model carries contexts as given; resolving and applying them is the job of `ngsi_ld::linked_data`,
/// behind the `linked-data` feature.
///
/// The enum is exhaustive: JSON-LD 1.1 writes a context as one entry or an array of them.
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::model::{Context, ContextEntry, InlineContext};
///
/// let context = Context::from(vec![
///     ContextEntry::from(IriRefBuf::new("https://example.org/context.jsonld")?),
///     ContextEntry::from(r#"{"@vocab": "https://example.org/terms/"}"#.parse::<InlineContext>()?),
/// ]);
/// assert_eq!(context.references().count(), 1);
/// assert_eq!(context.link_target(), None);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum Context {
    /// A single entry, written without an array.
    Single(ContextEntry),
    /// An array of entries.
    Multiple(Vec<ContextEntry>),
}

// No derive wraps a value two levels deep, into a single reference entry.
impl From<IriRefBuf> for Context {
    fn from(reference: IriRefBuf) -> Self {
        Self::Single(ContextEntry::Reference(reference))
    }
}

// No derive wraps a value two levels deep. Every IRI is an IRI reference (RFC 3987 section 2.2), so its text needs
// no second validation.
impl From<IriBuf> for Context {
    fn from(iri: IriBuf) -> Self {
        Self::Single(ContextEntry::Reference(IriRefBuf::parse_unchecked(iri.into_inner())))
    }
}

/// Why JSON is not an `@context` value.
// `InvalidIri` carries no `Clone` or equality, so neither can the error that chains it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContextError {
    /// An entry is neither a string nor an object (`null` included, which clause 4.5.0 rules out in NGSI-LD), or an
    /// array is nested in an array.
    #[error("an @context entry must be an IRI reference or an object")]
    NotAnEntry,
    /// A string entry is not an IRI reference.
    #[error("an @context reference is not an IRI reference")]
    Reference {
        /// Why.
        #[source]
        source: InvalidIri<String>,
    },
}

impl Context {
    /// The entries, in order.
    #[must_use]
    pub const fn entries(&self) -> &[ContextEntry] {
        match self {
            Self::Single(entry) => slice::from_ref(entry),
            Self::Multiple(entries) => entries.as_slice(),
        }
    }

    /// The referenced context documents, in order.
    pub fn references(&self) -> impl Iterator<Item = &IriRefBuf> {
        self.entries().iter().filter_map(|entry| match entry {
            ContextEntry::Reference(reference) => Some(reference),
            ContextEntry::Inline(_) => None,
        })
    }

    /// The target of the `Link` header that can carry this context, when it is a single reference: clause 6.3.5
    /// names exactly one context document in that header, so anything else travels in an `application/ld+json`
    /// body.
    #[must_use]
    pub const fn link_target(&self) -> Option<&IriRefBuf> {
        match self {
            Self::Single(ContextEntry::Reference(reference)) => Some(reference),
            Self::Single(ContextEntry::Inline(_)) | Self::Multiple(_) => None,
        }
    }

    /// Reads an `@context` value.
    pub(crate) fn from_json(json: Json) -> Result<Self, ContextError> {
        match json {
            Json::Array(items) => items.into_iter().map(entry_from_json).collect::<Result<_, _>>().map(Self::Multiple),
            Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_) => entry_from_json(json).map(Self::Single),
        }
    }

    /// Writes the `@context` value.
    pub(crate) fn to_json(&self) -> Json {
        match self {
            Self::Single(entry) => entry_to_json(entry),
            Self::Multiple(entries) => Json::Array(entries.iter().map(entry_to_json).collect()),
        }
    }
}

/// Reads one entry.
fn entry_from_json(json: Json) -> Result<ContextEntry, ContextError> {
    match json {
        Json::String(reference) => IriRefBuf::new(reference.as_str())
            .map(ContextEntry::Reference)
            .map_err(|source| ContextError::Reference { source }),
        Json::Object(_) => Ok(ContextEntry::Inline(InlineContext(RawJson::from_json(json)))),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) => Err(ContextError::NotAnEntry),
    }
}

/// Writes one entry.
fn entry_to_json(entry: &ContextEntry) -> Json {
    match entry {
        ContextEntry::Reference(reference) => Json::from(reference.as_str()),
        // The context is written from a borrow, and the JSON tree owns its members: a stack-safe copy.
        ContextEntry::Inline(inline) => inline.0.to_json(),
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Context, ContextError, InlineContext, InlineContextError, RawJson};
    use iri_rs::IriRefBuf;
    use jstrict::Value as Json;
    use pretty_assertions::assert_eq;

    fn read(text: &str) -> Result<Context, ContextError> {
        Context::from_json(text.parse::<Json>().unwrap())
    }

    #[test]
    fn relative_absolute_and_inline_entries_are_read_and_written_back() {
        for text in [
            r#""context.jsonld""#,
            r#""../contexts/a.jsonld""#,
            r#""https://example.org/context.jsonld""#,
            r#"{"@vocab":"https://example.org/terms/"}"#,
            r#"["https://example.org/a.jsonld",{"speed":"https://example.org/speed"},"b.jsonld"]"#,
        ] {
            let context = read(text).unwrap();
            assert_eq!(context.to_json(), text.parse::<Json>().unwrap(), "{text}");
        }
    }

    #[test]
    fn only_a_single_reference_has_a_link_target() {
        assert_eq!(read(r#""a.jsonld""#).unwrap().link_target().map(IriRefBuf::as_str), Some("a.jsonld"));
        for text in [r#"["a.jsonld"]"#, r#"{"a":"https://example.org/a"}"#] {
            assert_eq!(read(text).unwrap().link_target(), None, "{text}");
        }
    }

    #[test]
    fn references_skip_inline_entries() {
        let context = read(r#"["a.jsonld",{"x":"https://example.org/x"},"https://example.org/b"]"#).unwrap();
        assert_eq!(
            context.references().map(IriRefBuf::as_str).collect::<Vec<_>>(),
            ["a.jsonld", "https://example.org/b"]
        );
    }

    #[test]
    fn null_booleans_numbers_nested_arrays_and_invalid_references_are_refused() {
        for text in ["null", "[null]", r#"["a.jsonld",null]"#, "true", "3", "[[]]", "[1]"] {
            assert!(matches!(read(text), Err(ContextError::NotAnEntry)), "{text}");
        }
        assert!(matches!(read(r#""http://exa mple.org""#), Err(ContextError::Reference { .. })));
    }

    #[test]
    fn an_inline_context_is_an_object_and_nothing_else() {
        assert!(r#"{"@vocab":"https://example.org/terms/"}"#.parse::<InlineContext>().is_ok());
        for text in ["null", "3", "true", r#""a.jsonld""#, "[]", r#"[{"a":"https://example.org/a"}]"#] {
            assert!(matches!(text.parse::<InlineContext>(), Err(InlineContextError::NotAnObject)), "{text}");
            assert!(InlineContext::try_from(text.parse::<RawJson>().unwrap()).is_err(), "{text}");
        }
        assert!(matches!("{".parse::<InlineContext>(), Err(InlineContextError::Json(_))));
    }
}
