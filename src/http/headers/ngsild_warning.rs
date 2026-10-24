//! The `NGSILD-Warning` header (NGSI-LD clause 6.3.17).

use crate::http::headers::{InvalidFieldValue, OptionalHeader, field_value::encode_text};
use http::{HeaderName, HeaderValue};

/// The header name.
static NGSILD_WARNING: HeaderName = HeaderName::from_static("ngsild-warning");

/// The IANA warning code of an `NGSILD-Warning`.
///
/// NGSI-LD Table 6.3.17-1.
///
/// The enum is `#[non_exhaustive]`: the codes come from the IANA warning code registry, of which Table 6.3.17-1 uses
/// four, and a later version of the specification may use more.
///
/// ```
/// use ngsi_ld::http::headers::WarningCode;
///
/// assert_eq!(
///     "110".parse::<WarningCode>().ok(),
///     Some(WarningCode::ResponseIsStale)
/// );
/// assert_eq!(WarningCode::ResponseIsStale.to_string(), "110");
/// assert_eq!(WarningCode::ResponseIsStale.phrase(), "Response is Stale");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString)]
#[non_exhaustive]
pub enum WarningCode {
    /// `110`: no request was made to a registration endpoint; cached data was used instead.
    #[strum(serialize = "110")]
    ResponseIsStale,
    /// `111`: the registration endpoint answered in time with an invalid payload.
    #[strum(serialize = "111")]
    RevalidationFailed,
    /// `199`: the registration endpoint did not answer in time, or a registration loop was detected.
    #[strum(serialize = "199")]
    MiscellaneousWarning,
    /// `299`: the registration endpoint answered in time with an error.
    #[strum(serialize = "299")]
    MiscellaneousPersistentWarning,
}

impl WarningCode {
    /// The phrase Table 6.3.17-1 gives the code.
    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::ResponseIsStale => "Response is Stale",
            Self::RevalidationFailed => "Revalidation Failed",
            Self::MiscellaneousWarning => "Miscellaneous Warning",
            Self::MiscellaneousPersistentWarning => "Miscellaneous Persistent Warning",
        }
    }

    /// The field value of the code as clause 6.3.17 writes it: the code, ` - `, and its phrase.
    const fn canonical(self) -> &'static str {
        match self {
            Self::ResponseIsStale => "110 - Response is Stale",
            Self::RevalidationFailed => "111 - Revalidation Failed",
            Self::MiscellaneousWarning => "199 - Miscellaneous Warning",
            Self::MiscellaneousPersistentWarning => "299 - Miscellaneous Persistent Warning",
        }
    }
}

/// One warning of an `NGSILD-Warning` header.
///
/// It says what went wrong in the distributed part of a GET on `/entities` or `/entities/{entityId}` (NGSI-LD clause
/// 6.3.17).
///
/// The warning is its [`WarningCode`] and the text the broker wrote after it, such as `- Response is Stale`, kept as
/// received: clause 6.3.17 writes the code and its phrase, while the IANA registry the codes come from also allows a
/// warning agent and a quoted text (RFC 7234 section 5.5). The field value is rendered on construction, so encoding
/// cannot fail.
///
/// ```
/// use ngsi_ld::http::headers::{NgsiLdWarning, WarningCode};
///
/// let stale = NgsiLdWarning::new(WarningCode::ResponseIsStale);
/// assert_eq!(stale.code(), WarningCode::ResponseIsStale);
/// assert_eq!(stale.text(), "- Response is Stale");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NgsiLdWarning {
    /// The warning code.
    code: WarningCode,
    /// The text after the code, without surrounding whitespace.
    text: String,
    /// The code and the text, rendered as a field value.
    encoded: HeaderValue,
}

impl NgsiLdWarning {
    /// The warning `code` as clause 6.3.17 writes it, such as `110 - Response is Stale`.
    #[must_use]
    pub fn new(code: WarningCode) -> NgsiLdWarning {
        NgsiLdWarning {
            code,
            text: format!("- {}", code.phrase()),
            encoded: HeaderValue::from_static(code.canonical()),
        }
    }

    /// The warning `code` followed by `text`, such as a warning agent and a quoted text (RFC 7234 section 5.5).
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the text cannot be carried in a header field.
    pub fn try_with_text(code: WarningCode, text: &str) -> Result<NgsiLdWarning, InvalidFieldValue> {
        let text = text.trim_matches([' ', '\t']);
        let encoded = if text.is_empty() {
            encode_text(&code.to_string())?
        } else {
            encode_text(&format!("{code} {text}"))?
        };
        Ok(NgsiLdWarning {
            code,
            text: text.to_owned(),
            encoded,
        })
    }

    /// The warning code.
    #[must_use]
    pub const fn code(&self) -> WarningCode {
        self.code
    }

    /// The text the broker wrote after the code, such as `- Response is Stale`; empty when it wrote the code alone.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Reads one warning: a three-digit code, then nothing or whitespace and the text.
    fn read(warning: &str) -> Option<NgsiLdWarning> {
        let warning = warning.trim_matches([' ', '\t']);
        let (code, text) = warning.split_at_checked(3)?;
        if !(text.is_empty() || text.starts_with([' ', '\t'])) {
            return None;
        }
        NgsiLdWarning::try_with_text(code.parse().ok()?, text).ok()
    }
}

/// The `NGSILD-Warning` header: the warnings a broker attaches to an answer.
///
/// The broker attaches them to the answer of a GET on `/entities` or `/entities/{entityId}` when a distributed part of
/// it behaved abnormally or was served from a cache (NGSI-LD clause 6.3.17).
///
/// The header holds at least one warning. Clause 6.3.17 sends one warning per header field; a field line holding
/// several, separated by commas as the HTTP list syntax allows, is read too: a comma starts a new warning when a
/// three-digit code follows it, and is part of the text otherwise. A header holding a code Table 6.3.17-1 does not
/// list does not decode, so a response keeps it as [`OptionalHeader::Malformed`] with its raw lines.
///
/// ```
/// use headers::{Header, HeaderMapExt};
/// use http::{HeaderMap, HeaderValue};
/// use ngsi_ld::http::headers::{NgsiLdWarnings, WarningCode};
///
/// let mut headers = HeaderMap::new();
/// headers.append(
///     NgsiLdWarnings::name(),
///     HeaderValue::from_static("110 - Response is Stale"),
/// );
/// headers.append(
///     NgsiLdWarnings::name(),
///     HeaderValue::from_static("199 - Miscellaneous Warning"),
/// );
/// let warnings = headers.typed_get::<NgsiLdWarnings>();
/// let codes: Vec<WarningCode> = warnings
///     .iter()
///     .flat_map(NgsiLdWarnings::iter)
///     .map(|warning| warning.code())
///     .collect();
/// assert_eq!(
///     codes,
///     [
///         WarningCode::ResponseIsStale,
///         WarningCode::MiscellaneousWarning
///     ]
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NgsiLdWarnings(Vec<NgsiLdWarning>);

impl NgsiLdWarnings {
    /// A header holding `first`.
    #[must_use]
    pub fn new(first: NgsiLdWarning) -> NgsiLdWarnings {
        NgsiLdWarnings(vec![first])
    }

    /// This header with `warning` added after the others.
    #[must_use]
    pub fn and(mut self, warning: NgsiLdWarning) -> NgsiLdWarnings {
        self.0.push(warning);
        self
    }

    /// The warnings, in the order received.
    pub fn iter(&self) -> impl Iterator<Item = &NgsiLdWarning> {
        self.0.iter()
    }

    /// The warnings, in the order received.
    #[must_use]
    pub fn as_slice(&self) -> &[NgsiLdWarning] {
        &self.0
    }

    /// The field lines of `warnings`, one per warning.
    fn lines(&self) -> impl Iterator<Item = HeaderValue> + '_ {
        // `encoded` is shared bytes; cloning a `HeaderValue` only bumps their reference count.
        self.0.iter().map(|warning| warning.encoded.clone())
    }
}

/// Whether `text` starts with a three-digit code followed by whitespace, a comma or nothing.
fn starts_with_code(text: &str) -> bool {
    let mut chars = text.chars();
    chars.by_ref().take(3).filter(char::is_ascii_digit).count() == 3 && chars.next().is_none_or(|next| matches!(next, ' ' | '\t' | ','))
}

/// Splits one field line into its warnings: a comma starts a new one only before a three-digit code.
fn split_line(line: &str) -> Vec<&str> {
    let mut warnings = Vec::new();
    let mut current = line;
    let mut searched = 0;
    while let Some(comma) = current.get(searched..).and_then(|tail| tail.find(',')).map(|offset| searched + offset) {
        let (before, after) = (current.get(..comma).unwrap_or_default(), current.get(comma + 1..).unwrap_or_default());
        if starts_with_code(after.trim_start_matches([' ', '\t'])) {
            warnings.push(before);
            current = after;
            searched = 0;
        } else {
            searched = comma + 1;
        }
    }
    warnings.push(current);
    warnings
}

impl headers::Header for NgsiLdWarnings {
    fn name() -> &'static HeaderName {
        &NGSILD_WARNING
    }

    fn decode<'i, I>(values: &mut I) -> Result<NgsiLdWarnings, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let mut warnings = Vec::new();
        for value in values {
            let line = value.to_str().map_err(|_| headers::Error::invalid())?;
            for warning in split_line(line) {
                warnings.push(NgsiLdWarning::read(warning).ok_or_else(headers::Error::invalid)?);
            }
        }
        if warnings.is_empty() {
            return Err(headers::Error::invalid());
        }
        Ok(NgsiLdWarnings(warnings))
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(self.lines());
    }
}

/// The warnings of two answers to one request, `earlier` then `later`, as when the answers for the parts of a time
/// range are joined (clause 6.3.10): the warnings of both, or, when either is malformed, the raw lines of both.
pub(crate) fn combine(earlier: OptionalHeader<NgsiLdWarnings>, later: OptionalHeader<NgsiLdWarnings>) -> OptionalHeader<NgsiLdWarnings> {
    match (earlier, later) {
        (OptionalHeader::Absent, other) | (other, OptionalHeader::Absent) => other,
        (OptionalHeader::Valid(mut first), OptionalHeader::Valid(second)) => {
            first.0.extend(second.0);
            OptionalHeader::Valid(first)
        }
        (OptionalHeader::Valid(first), OptionalHeader::Malformed(lines)) => OptionalHeader::Malformed(first.lines().chain(lines).collect()),
        (OptionalHeader::Malformed(mut lines), OptionalHeader::Valid(second)) => {
            lines.extend(second.lines());
            OptionalHeader::Malformed(lines)
        }
        (OptionalHeader::Malformed(mut lines), OptionalHeader::Malformed(more)) => {
            lines.extend(more);
            OptionalHeader::Malformed(lines)
        }
    }
}

#[cfg(test)]
mod tests;
