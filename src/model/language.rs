//! Language maps: one text per natural language (NGSI-LD clause 4.6.5).

use crate::model::{keyword::NONE, null::NGSI_LD_NULL};
use indexmap::IndexMap;
use jstrict::{Object, Print, Value as Json, object::Entry};
use langtag::LangTagBuf;
use std::{
    borrow::Cow,
    fmt::{self, Display},
    str::FromStr,
};

/// Why text is not a BCP 47 language tag (IETF RFC 5646).
///
/// The tag parser reports only that a tag is refused, so the reason is classified from the text: no text at all, a
/// character no tag holds, or subtags in an order or of a length the grammar refuses.
///
/// ```
/// use ngsi_ld::model::LanguageTagError;
///
/// assert_eq!(
///     LanguageTagError::InvalidCharacter {
///         found: ' ',
///         position: 3
///     }
///     .to_string(),
///     "the character ` ` at position 3 cannot appear in a language tag"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum LanguageTagError {
    /// The text is empty.
    #[error("a language tag cannot be empty")]
    Empty,
    /// A character is neither an ASCII letter, an ASCII digit nor `-`.
    #[error("the character `{found}` at position {position} cannot appear in a language tag")]
    InvalidCharacter {
        /// The character.
        found: char,
        /// Its position among the characters of the text, counting from 0.
        position: usize,
    },
    /// The subtags do not follow the grammar of IETF RFC 5646.
    #[error("the subtags do not form a language tag")]
    Malformed,
}

impl LanguageTagError {
    /// Why `text`, which the tag parser refused, is not a language tag.
    fn classify(text: &str) -> Self {
        if text.is_empty() {
            return Self::Empty;
        }
        text.chars()
            .enumerate()
            .find(|&(_, character)| !character.is_ascii_alphanumeric() && character != '-')
            .map_or(Self::Malformed, |(position, found)| Self::InvalidCharacter { found, position })
    }
}

/// Parses a BCP 47 language tag.
///
/// # Errors
///
/// Returns [`LanguageTagError`] when `text` is not one.
pub(crate) fn parse_language_tag(text: &str) -> Result<LangTagBuf, LanguageTagError> {
    // The parser's error holds nothing but the text, which the classification reads instead.
    text.parse::<LangTagBuf>().map_err(|_| LanguageTagError::classify(text))
}

/// A language tag as a caller gives it: either already a [`LangTagBuf`], or text the call parses as a BCP 47 tag
/// (IETF RFC 5646) and refuses with [`LanguageTagError`] when it is not one. Both convert with `From`, so a setter
/// taking `impl Into<LanguageTagInput>` accepts `&LangTagBuf` and `&str` alike.
///
/// The enum is exhaustive: a tag is given parsed or as text.
///
/// ```
/// use langtag::LangTagBuf;
/// use ngsi_ld::model::LanguageTagInput;
///
/// let french: LangTagBuf = "fr".parse()?;
/// assert_eq!(
///     LanguageTagInput::from(&french),
///     LanguageTagInput::Tag(&french)
/// );
/// assert_eq!(LanguageTagInput::from("fr"), LanguageTagInput::Text("fr"));
/// assert_eq!(*LanguageTagInput::from("fr").resolve()?, french);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::From)]
pub enum LanguageTagInput<'a> {
    /// A parsed tag.
    Tag(&'a LangTagBuf),
    /// Text, parsed when the call is made.
    Text(&'a str),
}

impl<'a> LanguageTagInput<'a> {
    /// The tag: borrowed when given parsed, parsed from the text otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`LanguageTagError`] when the text is not a language tag.
    pub fn resolve(self) -> Result<Cow<'a, LangTagBuf>, LanguageTagError> {
        match self {
            LanguageTagInput::Tag(tag) => Ok(Cow::Borrowed(tag)),
            LanguageTagInput::Text(text) => parse_language_tag(text).map(Cow::Owned),
        }
    }
}

/// Why data is not a valid language map.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum LanguageMapError {
    /// The map has no entries.
    #[error("a language map cannot be empty")]
    Empty,
    /// A key is neither `@none` nor a BCP 47 language tag.
    #[error("a language map key is not a BCP 47 language tag")]
    InvalidTag {
        /// Why the key is not a tag.
        #[source]
        source: LanguageTagError,
    },
    /// A language appears twice.
    #[error("the language `{key}` appears twice")]
    DuplicateLanguage {
        /// The repeated key.
        key: LanguageKey,
    },
    /// A text is neither a string nor an array of strings.
    #[error("a language map value must be a string or an array of strings")]
    NotText,
    /// The map is not a JSON object.
    #[error("a language map must be a JSON object")]
    NotAnObject,
}

/// The key of a language map entry: a BCP 47 language tag, or `@none` for the text used when no more specific
/// language matches (NGSI-LD clause 4.6.5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum LanguageKey {
    /// `@none`: the default text.
    #[display("@none")]
    Default,
    /// A language tag (IETF RFC 5646).
    #[display("{_0}")]
    Tag(LangTagBuf),
}

// No derive parses the `@none` keyword into the unit variant.
impl FromStr for LanguageKey {
    type Err = LanguageMapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == NONE {
            Ok(Self::Default)
        } else {
            parse_language_tag(value)
                .map(Self::Tag)
                .map_err(|source| LanguageMapError::InvalidTag { source })
        }
    }
}

/// The text of one language: a string or an array of strings (NGSI-LD clause 4.6.5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::From)]
pub enum LanguageText {
    /// One string.
    #[from(String, &str)]
    Single(String),
    /// Several strings.
    Multiple(Vec<String>),
}

/// A language map: a non-empty, ordered set of texts keyed by language (NGSI-LD clause 4.6.5), the payload of a
/// `LanguageProperty` (clause 4.5.18).
///
/// ```
/// use ngsi_ld::model::LanguageMap;
///
/// let street = LanguageMap::try_from_pairs([("fr", "Grand Place"), ("nl", "Grote Markt")])?;
/// assert_eq!(
///     street.to_string(),
///     r#"{"fr":"Grand Place","nl":"Grote Markt"}"#
/// );
/// assert!(LanguageMap::try_from_pairs([("not a tag", "x")]).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageMap(IndexMap<LanguageKey, LanguageText>);

impl LanguageMap {
    /// The NGSI-LD Null of a language map, `{"@none": "urn:ngsi-ld:null"}` (clauses 4.5.0 and 4.5.18).
    #[must_use]
    pub fn ngsi_ld_null() -> LanguageMap {
        LanguageMap(IndexMap::from_iter([(
            LanguageKey::Default,
            LanguageText::Single(NGSI_LD_NULL.as_str().to_owned()),
        )]))
    }

    /// Builds a language map from `(language, text)` pairs, in order.
    ///
    /// # Errors
    ///
    /// Returns [`LanguageMapError`] when there are no pairs, a key is not a language tag or `@none`, or a
    /// language repeats.
    pub fn try_from_pairs<K: AsRef<str>, T: Into<LanguageText>>(pairs: impl IntoIterator<Item = (K, T)>) -> Result<Self, LanguageMapError> {
        let mut entries = IndexMap::new();
        for (key, text) in pairs {
            let key: LanguageKey = key.as_ref().parse()?;
            if entries.contains_key(&key) {
                return Err(LanguageMapError::DuplicateLanguage { key });
            }
            entries.insert(key, text.into());
        }
        if entries.is_empty() {
            Err(LanguageMapError::Empty)
        } else {
            Ok(Self(entries))
        }
    }

    /// Returns the entries in order.
    pub fn iter(&self) -> impl Iterator<Item = (&LanguageKey, &LanguageText)> {
        self.0.iter()
    }

    /// Returns the text of `key`, if present.
    #[must_use]
    pub fn get(&self, key: &LanguageKey) -> Option<&LanguageText> {
        self.0.get(key)
    }

    /// Reads a language map from a JSON object.
    pub(crate) fn from_json(json: Json) -> Result<Self, LanguageMapError> {
        let Json::Object(object) = json else {
            return Err(LanguageMapError::NotAnObject);
        };
        let pairs = object
            .into_iter()
            .map(|Entry { key, value }| Ok((key, text_from_json(value)?)))
            .collect::<Result<Vec<_>, LanguageMapError>>()?;
        Self::try_from_pairs(pairs)
    }

    /// Writes the language map as a JSON object.
    pub(crate) fn to_json(&self) -> Json {
        Json::Object(
            self.0
                .iter()
                .map(|(key, text)| {
                    let value = match text {
                        LanguageText::Single(single) => Json::from(single.as_str()),
                        LanguageText::Multiple(many) => Json::Array(many.iter().map(|text| Json::from(text.as_str())).collect()),
                    };
                    (key.to_string().into(), value)
                })
                .collect::<Object>(),
        )
    }
}

/// Reads a string or an array of strings.
fn text_from_json(value: Json) -> Result<LanguageText, LanguageMapError> {
    match value {
        Json::String(single) => Ok(LanguageText::Single(single.into_string())),
        Json::Array(many) => many
            .into_iter()
            .map(|item| match item {
                Json::String(text) => Ok(text.into_string()),
                Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_) => Err(LanguageMapError::NotText),
            })
            .collect::<Result<_, _>>()
            .map(LanguageText::Multiple),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Object(_) => Err(LanguageMapError::NotText),
    }
}

// The text form is the compact JSON form, which only exists once the map is turned into a JSON tree.
impl Display for LanguageMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_json().compact_print(), f)
    }
}

#[cfg(test)]
mod tests;
