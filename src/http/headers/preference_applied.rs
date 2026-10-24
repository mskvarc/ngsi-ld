// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The `Preference-Applied` response header (RFC 7240 section 3, NGSI-LD clauses 4.3.6.8 and 6.3.6).
//!
//! Portions of this file are derived from the `PreferenceApplied` implementation in
//! <https://github.com/hyperium/headers>. This file, including local modifications, is distributed under the MIT
//! License.

use crate::http::headers::{InvalidFieldValue, SpecVersion, field_value::encode_text, preference::first_value};
use http::{HeaderName, HeaderValue};
use std::iter;

/// The header name.
static PREFERENCE_APPLIED: HeaderName = HeaderName::from_static("preference-applied");

/// The name of the NGSI-LD version preference (clause 6.3.6).
pub(crate) const NGSI_LD_PREFERENCE: &str = "ngsi-ld";

/// The `Preference-Applied` header: the version a broker amended a payload to.
///
/// NGSI-LD uses the header this way in clauses 4.3.6.8 and 6.3.6, as in `Preference-Applied: ngsi-ld=1.8.1` on a 203
/// response.
///
/// Decoding reads the first `ngsi-ld` preference of the field (RFC 7240 section 2 makes the first occurrence of a
/// preference the one that counts) and ignores every other preference, which this crate does not ask for. A field
/// without a readable `ngsi-ld` version does not decode. The field value is rendered on construction, so encoding
/// cannot fail.
///
/// ```
/// use headers::{Header, HeaderMapExt};
/// use http::{HeaderMap, HeaderValue};
/// use ngsi_ld::http::headers::{PreferenceApplied, SpecVersion};
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     PreferenceApplied::name(),
///     HeaderValue::from_static("respond-async, ngsi-ld=1.8.1"),
/// );
/// let applied = headers.typed_get::<PreferenceApplied>();
/// assert_eq!(
///     applied.as_ref().map(PreferenceApplied::version),
///     Some(SpecVersion::with_patch(1, 8, 1))
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreferenceApplied {
    /// The version the payload conforms to.
    version: SpecVersion,
    /// `ngsi-ld=<version>`, rendered as a field value.
    encoded: HeaderValue,
}

impl PreferenceApplied {
    /// A `Preference-Applied: ngsi-ld=<version>` naming the version a payload conforms to.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the rendering cannot be carried in a header field.
    pub fn try_new(version: SpecVersion) -> Result<PreferenceApplied, InvalidFieldValue> {
        let encoded = encode_text(&format!("{NGSI_LD_PREFERENCE}={version}"))?;
        Ok(PreferenceApplied { version, encoded })
    }

    /// The version the payload conforms to.
    #[must_use]
    pub const fn version(&self) -> SpecVersion {
        self.version
    }
}

impl headers::Header for PreferenceApplied {
    fn name() -> &'static HeaderName {
        &PREFERENCE_APPLIED
    }

    fn decode<'i, I>(values: &mut I) -> Result<PreferenceApplied, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let lines: Vec<&HeaderValue> = values.collect();
        let version = first_value(&lines, NGSI_LD_PREFERENCE)?
            .ok_or_else(headers::Error::invalid)?
            .parse()
            .map_err(|_| headers::Error::invalid())?;
        PreferenceApplied::try_new(version).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // `encode` borrows the header; cloning a `HeaderValue` only bumps the reference count of its shared bytes.
        values.extend(iter::once(self.encoded.clone()));
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{PreferenceApplied, SpecVersion, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn decode(raw: &'static str) -> Option<SpecVersion> {
        decode_line::<PreferenceApplied>(raw).map(|applied| applied.version())
    }

    #[test]
    fn a_patch_level_version_decodes() {
        assert_eq!(decode("ngsi-ld=1.8.1"), Some(SpecVersion::with_patch(1, 8, 1)));
    }

    #[test]
    fn the_version_is_read_whatever_its_case_spacing_quoting_parameters_or_neighbours() {
        assert_eq!(decode(" NGSI-LD = \"1.8\" ; strict"), Some(SpecVersion::new(1, 8)));
        assert_eq!(decode("respond-async, wait=5, ngsi-ld=1.9"), Some(SpecVersion::new(1, 9)));
    }

    #[test]
    fn the_first_version_wins_across_lines() {
        let lines = [HeaderValue::from_static("respond-async"), HeaderValue::from_static("ngsi-ld=1.7, ngsi-ld=1.8")];
        assert_eq!(
            PreferenceApplied::decode(&mut lines.iter()).ok().map(|applied| applied.version()),
            Some(SpecVersion::new(1, 7))
        );
    }

    #[test]
    fn a_field_without_a_readable_version_does_not_decode() {
        for raw in ["respond-async", "return=minimal, body=json", "ngsi-ld=latest", "ngsi-ld", ""] {
            assert_eq!(decode(raw), None, "{raw}");
        }
    }

    #[test]
    fn a_version_encodes_as_its_token_and_decodes_back() {
        let applied = PreferenceApplied::try_new(SpecVersion::new(1, 8)).unwrap();
        let mut encoded = Vec::new();
        applied.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("ngsi-ld=1.8")]);
        assert_eq!(PreferenceApplied::decode(&mut encoded.iter()).ok(), Some(applied));
    }
}
