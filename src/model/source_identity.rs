//! The identity of a context source (NGSI-LD clauses 5.2.40 and 5.15): what Retrieve Context Source Identity
//! Information answers, naming a broker, or one tenant of it, uniquely.

#[cfg(feature = "linked-data")]
use crate::model::name::mapping::{MapNames, NameMapping};
use crate::model::{
    Context,
    DateTime,
    DocumentError,
    Duration,
    RawJson,
    keyword::CONTEXT,
    reader::{context, context_source_alias, date_time, duration, fixed_token, identifier, object, once, parse_json, required, unexpected, within},
    representation::write::write_context,
    token::is_token,
};
use jstrict::{Object, Value as Json, object::Entry};

/// The `type` of a context source identity (Table 5.2.40-1).
const IDENTITY_TYPE: &str = "ContextSourceIdentity";

uri_identifier!(
    /// The identifier of a context source: a URI (NGSI-LD Table 5.2.40-1).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::model::ContextSourceId;
    ///
    /// let id = ContextSourceId::from(iri!("urn:ngsi-ld:ContextSource:cs1"));
    /// assert_eq!(
    ///     id,
    ///     "urn:ngsi-ld:ContextSource:cs1".parse::<ContextSourceId>()?
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ContextSourceId
);

/// Why a string is not a context source alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ContextSourceAliasError {
    /// The alias is empty.
    #[error("a context source alias must not be empty")]
    Empty,
    /// The alias holds a character an IETF RFC 7230 token cannot hold.
    #[error("a context source alias may only hold the characters of an HTTP token")]
    InvalidCharacter,
}

/// Checks a context source alias: an IETF RFC 7230 `pseudonym` (section 5.7.1), which is a `token`, one or more
/// `tchar` (section 3.2.6, carried unchanged into RFC 9110 section 5.6.2).
fn validate_alias(alias: &str) -> Result<(), ContextSourceAliasError> {
    if alias.is_empty() {
        Err(ContextSourceAliasError::Empty)
    } else if is_token(alias) {
        Ok(())
    } else {
        Err(ContextSourceAliasError::InvalidCharacter)
    }
}

/// The alias of a context source (NGSI-LD Tables 5.2.9-1 and 5.2.40-1): a unique id a broker uses to recognise a
/// context source, or one tenant of it, and so to detect a loop of distributed operations. The tables make it a
/// non-empty "Pseudonym field as defined in IETF RFC 7230", which is an HTTP token.
///
/// ```
/// use ngsi_ld::model::{ContextSourceAlias, ContextSourceAliasError};
///
/// let alias: ContextSourceAlias = "broker-1".parse()?;
/// assert_eq!(alias.to_string(), "broker-1");
/// assert_eq!(
///     "broker 1".parse::<ContextSourceAlias>(),
///     Err(ContextSourceAliasError::InvalidCharacter)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_alias, error = ContextSourceAliasError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct ContextSourceAlias(String);

/// The identity of a context source (NGSI-LD clause 5.2.40): what Retrieve Context Source Identity Information
/// answers. In a broker serving several tenants, it identifies the tenant the request named (clause 5.15.1.1).
///
/// Only a broker writes it; it is read strictly, a member Table 5.2.40-1 does not define failing the read.
///
/// ```
/// use ngsi_ld::model::ContextSourceIdentity;
///
/// let text = concat!(
///     r#"{"id":"urn:ngsi-ld:ContextSource:cs1","type":"ContextSourceIdentity","contextSourceAlias":"broker-1","#,
///     r#""contextSourceUptime":"P1DT2H","contextSourceTimeAt":"2024-05-01T10:00:00Z"}"#
/// );
/// let identity = ContextSourceIdentity::parse(text)?;
/// assert_eq!(identity.alias.as_ref(), "broker-1");
/// assert_eq!(identity.uptime.to_string(), "P1DT2H");
/// assert_eq!(identity.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContextSourceIdentity {
    /// `id`: the context source.
    pub id: ContextSourceId,
    /// `contextSourceAlias`: the alias used to detect loops.
    pub alias: ContextSourceAlias,
    /// `contextSourceUptime`: how long the context source has been available.
    pub uptime: Duration,
    /// `contextSourceTimeAt`: the current time at the context source (clause 4.8).
    pub time_at: DateTime,
    /// `contextSourceExtras`: information on the configuration of the context source, raw JSON never interpreted as
    /// JSON-LD, kept as written.
    pub extras: Option<RawJson>,
    /// The `@context` the document carried.
    pub context: Option<Context>,
}

impl ContextSourceIdentity {
    /// The identity of the context source `id`, known as `alias`, available for `uptime` at `time_at`, with no
    /// extras and no `@context`.
    #[must_use]
    pub const fn new(id: ContextSourceId, alias: ContextSourceAlias, uptime: Duration, time_at: DateTime) -> ContextSourceIdentity {
        ContextSourceIdentity {
            id,
            alias,
            uptime,
            time_at,
            extras: None,
            context: None,
        }
    }

    /// Reads the identity.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the identity in the member order of Table 5.2.40-1: `id`, `type`, `contextSourceAlias`,
    /// `contextSourceUptime`, `contextSourceTimeAt`, `contextSourceExtras`, then `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Object::new();
        object.push("id".into(), Json::from(self.id.as_str()));
        object.push("type".into(), Json::from(IDENTITY_TYPE));
        object.push("contextSourceAlias".into(), Json::from(self.alias.as_ref()));
        object.push("contextSourceUptime".into(), Json::from(self.uptime.to_string()));
        object.push("contextSourceTimeAt".into(), Json::from(self.time_at.to_string()));
        if let Some(extras) = &self.extras {
            // The tree is kept verbatim; writing it into a new document needs its own copy.
            object.push("contextSourceExtras".into(), extras.to_json());
        }
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads the identity from JSON already parsed, such as the body of Retrieve Context Source Identity
    /// Information.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut alias = None;
        let mut uptime = None;
        let mut time_at = None;
        let mut extras = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, identifier(value, "id"))?)?,
                "type" => once(&mut typed, key, within(key, fixed_token(value, "type", IDENTITY_TYPE))?)?,
                "contextSourceAlias" => once(&mut alias, key, within(key, context_source_alias(value))?)?,
                "contextSourceUptime" => once(&mut uptime, key, within(key, duration(value, "contextSourceUptime"))?)?,
                "contextSourceTimeAt" => once(&mut time_at, key, within(key, date_time(value, "contextSourceTimeAt"))?)?,
                "contextSourceExtras" => once(&mut extras, key, RawJson::from_json(value))?,
                CONTEXT => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            alias: required(alias, "contextSourceAlias")?,
            uptime: required(uptime, "contextSourceUptime")?,
            time_at: required(time_at, "contextSourceTimeAt")?,
            extras,
            context: document_context,
        })
    }
}

#[cfg(feature = "linked-data")]
/// A context source identity describes a context source, never a name (clause 5.2.40).
impl MapNames for ContextSourceIdentity {
    fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(self)
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    impl for ContextSourceIdentity => context
);

#[cfg(test)]
mod tests;
