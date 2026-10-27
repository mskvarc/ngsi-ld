//! The `@context`s a broker stores and serves (NGSI-LD clause 5.13): the identifier a broker gives a stored
//! context, its kind, the metadata it reports about it, and the local context document Add and Serve @context
//! carry.

use crate::model::{
    Context,
    DateTime,
    DocumentError,
    ExtraMemberName,
    OtherContextKind,
    RawJson,
    Value,
    keyword::CONTEXT,
    reader::{context, count, date_time, duplicate, identifier, object, once, parse_json, required, token, unexpected, url, value, within},
};
use indexmap::IndexMap;
use jstrict::{Object, Value as Json, object::Entry};
use std::str::FromStr;
use url::Url;

pub(super) mod extra_member;
pub(super) mod other_kind;

uri_identifier!(
    /// The local identifier of a stored `@context`.
    ///
    /// It is a URI the broker generates when it stores the context (NGSI-LD clause 5.13.2.4). A context of kind
    /// [`Cached`](StoredContextKind::Cached) is also named by the URL it was downloaded from (Table 6.30.2-1), which is
    /// a URI too.
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::ContextId;
    ///
    /// let id = ContextId::from(iri!("urn:ngsi-ld:context:c1"));
    /// assert_eq!(id, "urn:ngsi-ld:context:c1".parse::<ContextId>()?);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ContextId
);

/// The kind of a stored `@context`.
///
/// It is written as NGSI-LD clause 5.13.1 spells it in the `kind` member and parameter.
///
/// Clause 5.13.3.3 lists the kinds "currently" possible, so a kind the clause does not define is read into
/// [`Other`](Self::Other) with its token rather than refused, and written back as read. The enum is
/// `#[non_exhaustive]`: a kind a later version defines becomes a variant of its own.
///
/// ```
/// use ngsi_ld::model::StoredContextKind;
///
/// assert_eq!(
///     "ImplicitlyCreated".parse(),
///     Ok(StoredContextKind::ImplicitlyCreated)
/// );
/// assert_eq!(StoredContextKind::Hosted.to_string(), "Hosted");
/// assert!("".parse::<StoredContextKind>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::Display)]
#[non_exhaustive]
pub enum StoredContextKind {
    /// Downloaded by the broker from the URL a request named, and used as a cache; never served.
    Cached,
    /// Added by a client with Add @context, and served on demand.
    Hosted,
    /// Created by the broker for a request, such as the array context of a subscription it notifies in
    /// `application/json`, and served on demand.
    ImplicitlyCreated,
    /// A kind clause 5.13.1 does not define, such as the `ImplicitlyGenerated` clause 5.13.4.4 writes once.
    #[strum(to_string = "{0}")]
    Other(OtherContextKind),
}

/// Why text is not the kind of a stored `@context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum StoredContextKindError {
    /// The text is empty.
    #[error("the kind of a stored @context cannot be empty")]
    Empty,
}

// No derive: strum's catch-all variant converts any text, a defined kind included, into `Other`; only text that
// names no kind of clause 5.13.1 becomes `Other` here, so each kind has one representation.
impl FromStr for StoredContextKind {
    type Err = StoredContextKindError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let known = [Self::Cached, Self::Hosted, Self::ImplicitlyCreated];
        if text.is_empty() {
            Err(StoredContextKindError::Empty)
        } else {
            Ok(known
                .into_iter()
                .find(|kind| kind.to_string() == text)
                .unwrap_or_else(|| Self::Other(OtherContextKind::new(text.to_owned()))))
        }
    }
}

/// What a broker reports about one stored `@context`.
///
/// It is an element of the detailed answer of List @contexts, and the detailed answer of Serve @context (NGSI-LD clause
/// 5.13.3.5).
///
/// Clause 5.13.3.5 closes the member set ("a list of resulting JSON objects containing the following fields") and
/// gives implementations `extraInfo` for anything else, so a reader refuses any other member. An optional member
/// (`expiresAt`, `lastUsage`, `numberOfHits`) that holds the JSON null is read as one that is left out, and is not
/// written back. The set of kinds is
/// open (clause 5.13.3.3), so an unknown `kind` is read into [`StoredContextKind::Other`].
///
/// ```
/// use ngsi_ld::model::{ContextMetadata, StoredContextKind};
///
/// let text = concat!(
///     r#"{"URL":"https://example.org/context.jsonld","#,
///     r#""localId":"urn:ngsi-ld:context:c1","kind":"Cached","#,
///     r#""createdAt":"2024-05-01T10:00:00Z","numberOfHits":3}"#
/// );
/// let metadata = ContextMetadata::parse(text)?;
/// assert_eq!(metadata.kind, StoredContextKind::Cached);
/// assert_eq!(metadata.number_of_hits, Some(3));
/// assert_eq!(metadata.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContextMetadata {
    /// `URL`: where the context can be downloaded; for a `Cached` context, the URL the broker downloaded it from.
    pub url: Url,
    /// `localId`: the identifier the broker gave the context.
    pub local_id: ContextId,
    /// `kind`.
    pub kind: StoredContextKind,
    /// `createdAt`: when the broker stored the context.
    pub created_at: DateTime,
    /// `expiresAt`: when the broker drops the context.
    pub expires_at: Option<DateTime>,
    /// `lastUsage`: when the broker last used the context.
    pub last_usage: Option<DateTime>,
    /// `numberOfHits`: how many times the broker found the context in its storage.
    pub number_of_hits: Option<u64>,
    /// `extraInfo`: information clause 5.13.3.5 leaves to implementations, kept as written.
    pub extra_info: Option<RawJson>,
}

impl ContextMetadata {
    /// The metadata of the context `local_id` of `kind`, downloadable at `url` and stored at `created_at`, with
    /// none of the optional members.
    #[must_use]
    pub const fn new(url: Url, local_id: ContextId, kind: StoredContextKind, created_at: DateTime) -> ContextMetadata {
        ContextMetadata {
            url,
            local_id,
            kind,
            created_at,
            expires_at: None,
            last_usage: None,
            number_of_hits: None,
            extra_info: None,
        }
    }

    /// Reads the metadata.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the metadata in the member order of clause 5.13.3.5: `URL`, `localId`, `kind`, `createdAt`,
    /// `expiresAt`, `lastUsage`, `numberOfHits`, `extraInfo`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Object::new();
        object.push("URL".into(), Json::from(self.url.as_str()));
        object.push("localId".into(), Json::from(self.local_id.as_str()));
        object.push("kind".into(), Json::from(self.kind.to_string()));
        object.push("createdAt".into(), Json::from(self.created_at.to_string()));
        if let Some(expires_at) = &self.expires_at {
            object.push("expiresAt".into(), Json::from(expires_at.to_string()));
        }
        if let Some(last_usage) = &self.last_usage {
            object.push("lastUsage".into(), Json::from(last_usage.to_string()));
        }
        if let Some(hits) = self.number_of_hits {
            object.push("numberOfHits".into(), Json::from(hits));
        }
        if let Some(extra_info) = &self.extra_info {
            // The tree is kept verbatim; writing it into a new document needs its own copy.
            object.push("extraInfo".into(), extra_info.to_json());
        }
        RawJson::from_json(Json::Object(object))
    }

    /// Reads the metadata from JSON already parsed, such as an element of the detailed answer of List @contexts.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut url_member = None;
        let mut local_id = None;
        let mut kind = None;
        let mut created_at = None;
        let mut expires_at = None;
        let mut last_usage = None;
        let mut number_of_hits = None;
        let mut extra_info = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "URL" => once(&mut url_member, key, within(key, url(value, "URL"))?)?,
                "localId" => once(&mut local_id, key, within(key, identifier(value, "localId"))?)?,
                "kind" => once(&mut kind, key, within(key, token(value, "kind"))?)?,
                "createdAt" => once(&mut created_at, key, within(key, date_time(value, "createdAt"))?)?,
                "expiresAt" => once(&mut expires_at, key, within(key, unless_null(value, |json| date_time(json, "expiresAt")))?)?,
                "lastUsage" => once(&mut last_usage, key, within(key, unless_null(value, |json| date_time(json, "lastUsage")))?)?,
                "numberOfHits" => once(&mut number_of_hits, key, within(key, unless_null(value, |json| count(json, "numberOfHits")))?)?,
                "extraInfo" => once(&mut extra_info, key, RawJson::from_json(value))?,
                other => return Err(unexpected(other)),
            }
        }
        Ok(Self {
            url: required(url_member, "URL")?,
            local_id: required(local_id, "localId")?,
            kind: required(kind, "kind")?,
            created_at: required(created_at, "createdAt")?,
            expires_at: expires_at.flatten(),
            last_usage: last_usage.flatten(),
            number_of_hits: number_of_hits.flatten(),
            extra_info,
        })
    }
}

/// Reads `value` with `read`, or reads nothing when it is the JSON null. Clause 5.13.3.5 marks `expiresAt`,
/// `lastUsage` and `numberOfHits` optional and says nothing of null, so a member that holds null is one that holds no
/// value, as when it is left out.
fn unless_null<T>(value: Json, read: impl FnOnce(Json) -> Result<T, DocumentError>) -> Result<Option<T>, DocumentError> {
    if matches!(value, Json::Null) { Ok(None) } else { read(value).map(Some) }
}

/// A JSON-LD local context document.
///
/// It is "a JSON object that has a root node named @context" (NGSI-LD Tables 6.29.3.1-1 and 6.30.3.1-2), what Add
/// @context sends and Serve @context answers with.
///
/// Beside the `@context` member, the object may carry other members, which clause 5.13.2.3 has the broker discard:
/// a stored context is the `@context` subtree alone. They are kept in order, by name, as NGSI-LD Values (clause
/// 3.1), so a document carrying them round-trips.
///
/// ```
/// use ngsi_ld::model::{Context, LocalContext};
///
/// let text = concat!(
///     r#"{"@context":{"speed":"https://example.org/speed"},"#,
///     r#""note":"discarded by the broker"}"#
/// );
/// let document = LocalContext::parse(text)?;
/// assert!(matches!(document.context, Context::Single(_)));
/// assert_eq!(document.extra_members.len(), 1);
/// assert_eq!(document.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LocalContext {
    /// The `@context` member.
    pub context: Context,
    /// The members beside `@context`, in order, which the broker discards (clause 5.13.2.3).
    pub extra_members: IndexMap<ExtraMemberName, Value>,
}

impl LocalContext {
    /// The document of `context`, with no other member.
    #[must_use]
    pub fn new(context: Context) -> LocalContext {
        LocalContext {
            context,
            extra_members: IndexMap::new(),
        }
    }

    /// Reads the document.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the document: `{"@context": ...}`, then the other members in order.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(Self::object(&self.context, &self.extra_members)))
    }

    /// The document of `context` and the members beside it as a JSON object, the body of Add @context.
    pub(crate) fn object<'a>(context: &Context, extra_members: impl IntoIterator<Item = (&'a ExtraMemberName, &'a Value)>) -> Object {
        let mut object = Object::new();
        object.push(CONTEXT.into(), context.to_json());
        for (name, value) in extra_members {
            object.push(name.as_ref().into(), value.to_json());
        }
        object
    }

    /// Reads the document from JSON already parsed, such as the body of Serve @context.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut local = None;
        let mut extra_members = IndexMap::new();
        for Entry { key, value: member } in object(json)? {
            let key = key.as_str();
            match key.parse::<ExtraMemberName>() {
                Err(_) if key == CONTEXT => once(&mut local, key, within(key, context(member))?)?,
                Err(_) => return Err(unexpected(key)),
                Ok(name) => {
                    let value = within(key, value(member))?;
                    if extra_members.insert(name, value).is_some() {
                        return Err(duplicate(key));
                    }
                }
            }
        }
        Ok(Self {
            context: required(local, CONTEXT)?,
            extra_members,
        })
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// The metadata of a stored context carries no `@context` of its own.
    impl for ContextMetadata
);

#[cfg(test)]
mod tests;
