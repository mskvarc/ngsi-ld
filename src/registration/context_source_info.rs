//! The `contextSourceInfo` of a registration (NGSI-LD Table 5.2.9-1): key-value pairs a broker sends a context source
//! with every distributed request, whose keys clause 4.3.6.6 gives a meaning and whose value clause 4.3.6.5 lets
//! stand for the value of the request that triggered it.

use crate::{
    model::{ConformanceVersion, ConformanceVersionError, NgsiLdMediaType},
    registration::Problem,
};
use jstrict::{Object, Value as Json};
use url::Url;

/// The value clause 4.3.6.5 lets stand for the value of the triggering request.
const FROM_REQUEST: &str = "urn:ngsi-ld:request";

/// A key of `contextSourceInfo` that the specification defines.
///
/// NGSI-LD clause 4.3.6.6 defines the keys.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification define more keys.
///
/// ```
/// use ngsi_ld::registration::ContextSourceInfoKey;
///
/// assert_eq!(
///     ContextSourceInfoKey::JsonldContext.to_string(),
///     "jsonldContext"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum ContextSourceInfoKey {
    /// `accept`: the media type the context source answers in.
    Accept,
    /// `contentType`: the media type of the request body.
    ContentType,
    /// `jsonldContext`: the `@context` the context source is sent.
    JsonldContext,
    /// `ngsildConformance`: the version of the specification the context source conforms to.
    NgsildConformance,
}

/// A media type `accept` and `contentType` allow.
///
/// It is one of the NGSI-LD media types, whose text it is written and read as (NGSI-LD clause 4.3.6.6).
///
/// The enum is exhaustive: clause 4.3.6.6 allows these two.
///
/// ```
/// use ngsi_ld::{model::NgsiLdMediaType, registration::ContextSourceMediaType};
///
/// assert_eq!(
///     ContextSourceMediaType::from_media_type(NgsiLdMediaType::JsonLd),
///     Some(ContextSourceMediaType::LdJson)
/// );
/// assert_eq!(
///     ContextSourceMediaType::from_media_type(NgsiLdMediaType::GeoJson),
///     None
/// );
/// assert_eq!(ContextSourceMediaType::Json.to_string(), "application/json");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{}", self.media_type())]
pub enum ContextSourceMediaType {
    /// `application/json`.
    Json,
    /// `application/ld+json`.
    LdJson,
}

impl ContextSourceMediaType {
    /// The NGSI-LD media type.
    #[must_use]
    pub const fn media_type(self) -> NgsiLdMediaType {
        match self {
            ContextSourceMediaType::Json => NgsiLdMediaType::Json,
            ContextSourceMediaType::LdJson => NgsiLdMediaType::JsonLd,
        }
    }

    /// The context source media type `media_type` is, if clause 4.3.6.6 allows it.
    #[must_use]
    pub const fn from_media_type(media_type: NgsiLdMediaType) -> Option<ContextSourceMediaType> {
        match media_type {
            NgsiLdMediaType::Json => Some(ContextSourceMediaType::Json),
            NgsiLdMediaType::JsonLd => Some(ContextSourceMediaType::LdJson),
            NgsiLdMediaType::GeoJson => None,
        }
    }
}

/// The value of a `contextSourceInfo` pair: given, or `urn:ngsi-ld:request`.
///
/// `urn:ngsi-ld:request` stands for the value of the request that triggered the distributed operation (NGSI-LD clause
/// 4.3.6.5).
///
/// The enum is exhaustive: clause 4.3.6.5 adds this one alternative.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ContextSourceInfoValue<T> {
    /// A value.
    #[display("{_0}")]
    Given(T),
    /// `urn:ngsi-ld:request`: the value of the triggering request.
    #[display("urn:ngsi-ld:request")]
    FromRequest,
}

impl<T> ContextSourceInfoValue<T> {
    /// Reads `value`, or `urn:ngsi-ld:request`.
    fn parse<E>(value: &str, given: impl FnOnce(&str) -> Result<T, E>) -> Result<Self, E> {
        if value == FROM_REQUEST {
            Ok(Self::FromRequest)
        } else {
            given(value).map(Self::Given)
        }
    }
}

/// A pair of `contextSourceInfo` whose key the specification does not define.
///
/// NGSI-LD clause 4.3.6.6 defines the keys; such a pair is, for instance, an HTTP header for the context source. Its
/// key is never one of [`ContextSourceInfoKey`]; it is made only by [`ContextSourceInfo::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OtherContextSourceInfo {
    /// The key, free text (Table 5.2.22-1).
    key: String,
    /// The value, free text unless it stands for the triggering request's.
    value: ContextSourceInfoValue<String>,
}

impl OtherContextSourceInfo {
    /// Returns the key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the value.
    #[must_use]
    pub const fn value(&self) -> &ContextSourceInfoValue<String> {
        &self.value
    }
}

/// One pair of a registration's `contextSourceInfo`, typed by its key.
///
/// NGSI-LD clauses 4.3.6.5 and 4.3.6.6.
///
/// The enum is `#[non_exhaustive]`: a key a later version of the specification defines becomes a variant.
///
/// ```
/// use ngsi_ld::registration::{
///     ContextSourceInfo,
///     ContextSourceInfoValue,
///     ContextSourceMediaType,
/// };
///
/// let accept = ContextSourceInfo::parse("accept", "application/ld+json")?;
/// assert_eq!(
///     accept,
///     ContextSourceInfo::Accept(ContextSourceInfoValue::Given(
///         ContextSourceMediaType::LdJson
///     ))
/// );
/// assert!(ContextSourceInfo::parse("accept", "text/plain").is_err());
/// assert_eq!(
///     ContextSourceInfo::parse("Authorization", "Bearer abc")?.key(),
///     "Authorization"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextSourceInfo {
    /// `accept`.
    Accept(ContextSourceInfoValue<ContextSourceMediaType>),
    /// `contentType`.
    ContentType(ContextSourceInfoValue<ContextSourceMediaType>),
    /// `jsonldContext`.
    JsonldContext(ContextSourceInfoValue<Url>),
    /// `ngsildConformance`.
    NgsildConformance(ContextSourceInfoValue<ConformanceVersion>),
    /// A key clause 4.3.6.6 does not define.
    Other(OtherContextSourceInfo),
}

/// Why the value of a `contextSourceInfo` pair is not one its key allows.
///
/// NGSI-LD clause 4.3.6.6.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ContextSourceInfoError {
    /// The value of `accept` or `contentType` is neither `application/json` nor `application/ld+json`.
    #[error("`{key}` must be application/json or application/ld+json")]
    MediaType {
        /// The key.
        key: ContextSourceInfoKey,
    },
    /// The value of `jsonldContext` is not a URL.
    #[error("`jsonldContext` must be a URL")]
    Url {
        /// Why.
        #[source]
        source: url::ParseError,
    },
    /// The value of `ngsildConformance` is not a `major.minor` version.
    #[error("`ngsildConformance` must be a major.minor version")]
    Conformance {
        /// Why.
        #[source]
        source: ConformanceVersionError,
    },
}

impl ContextSourceInfo {
    /// Reads the pair `key`, `value`, typing the value of a key clause 4.3.6.6 defines.
    ///
    /// # Errors
    ///
    /// Returns [`ContextSourceInfoError`] when the key is one clause 4.3.6.6 defines and the value is neither one it
    /// allows nor `urn:ngsi-ld:request`.
    pub fn parse(key: &str, value: &str) -> Result<ContextSourceInfo, ContextSourceInfoError> {
        let media_type = |key| {
            move |value: &str| {
                value
                    .parse::<NgsiLdMediaType>()
                    .ok()
                    .and_then(ContextSourceMediaType::from_media_type)
                    .ok_or(ContextSourceInfoError::MediaType { key })
            }
        };
        match key.parse::<ContextSourceInfoKey>() {
            Ok(ContextSourceInfoKey::Accept) => ContextSourceInfoValue::parse(value, media_type(ContextSourceInfoKey::Accept)).map(Self::Accept),
            Ok(ContextSourceInfoKey::ContentType) => ContextSourceInfoValue::parse(value, media_type(ContextSourceInfoKey::ContentType)).map(Self::ContentType),
            Ok(ContextSourceInfoKey::JsonldContext) => {
                ContextSourceInfoValue::parse(value, |url| url.parse::<Url>().map_err(|source| ContextSourceInfoError::Url { source })).map(Self::JsonldContext)
            }
            Ok(ContextSourceInfoKey::NgsildConformance) => ContextSourceInfoValue::parse(value, |version| {
                version
                    .parse::<ConformanceVersion>()
                    .map_err(|source| ContextSourceInfoError::Conformance { source })
            })
            .map(Self::NgsildConformance),
            Err(_) => Ok(Self::Other(OtherContextSourceInfo {
                key: key.to_owned(),
                value: ContextSourceInfoValue::parse(value, |text| Ok::<_, ContextSourceInfoError>(text.to_owned()))?,
            })),
        }
    }

    /// Returns the key.
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Accept(_) => ContextSourceInfoKey::Accept.into(),
            Self::ContentType(_) => ContextSourceInfoKey::ContentType.into(),
            Self::JsonldContext(_) => ContextSourceInfoKey::JsonldContext.into(),
            Self::NgsildConformance(_) => ContextSourceInfoKey::NgsildConformance.into(),
            Self::Other(other) => other.key(),
        }
    }

    /// Returns the value as written.
    fn value_text(&self) -> String {
        match self {
            Self::Accept(value) | Self::ContentType(value) => value.to_string(),
            Self::JsonldContext(value) => value.to_string(),
            Self::NgsildConformance(value) => value.to_string(),
            Self::Other(other) => other.value().to_string(),
        }
    }

    /// Writes the pair as `{"key": ..., "value": ...}` (clause 5.2.22).
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push("key".into(), Json::from(self.key()));
        object.push("value".into(), Json::from(self.value_text()));
        Json::Object(object)
    }
}

/// A `contextSourceInfo` pair given to a builder.
///
/// It is a [`ContextSourceInfo`], or a key and a value read when the builder finishes, so every invalid pair is
/// reported together.
///
/// ```
/// use ngsi_ld::registration::ContextSourceInfoDraft;
///
/// let header = ContextSourceInfoDraft::from(("Authorization", "Bearer abc"));
/// # let _ = header;
/// ```
#[derive(Debug, Clone)]
pub struct ContextSourceInfoDraft(Result<ContextSourceInfo, ContextSourceInfoError>);

// No derive: the conversion wraps the pair into the draft's `Result`.
impl From<ContextSourceInfo> for ContextSourceInfoDraft {
    fn from(info: ContextSourceInfo) -> Self {
        Self(Ok(info))
    }
}

// No derive: the conversion reads the key and the value into the draft's `Result`.
impl From<(&str, &str)> for ContextSourceInfoDraft {
    fn from((key, value): (&str, &str)) -> Self {
        Self(ContextSourceInfo::parse(key, value))
    }
}

/// Resolves the pairs given to a builder, recording each invalid one with its position.
pub(crate) fn resolve(drafts: Vec<ContextSourceInfoDraft>, problems: &mut Vec<Problem>) -> Vec<ContextSourceInfo> {
    drafts
        .into_iter()
        .enumerate()
        .filter_map(|(index, ContextSourceInfoDraft(info))| info.map_err(|source| problems.push(Problem::ContextSourceInfo { index, source })).ok())
        .collect()
}
