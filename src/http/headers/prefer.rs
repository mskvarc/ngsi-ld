//! The `Prefer` request header (RFC 7240, NGSI-LD clause 6.3.6).

use crate::http::headers::{InvalidFieldValue, SpecVersion, field_value::encode_text, preference::first_value, preference_applied::NGSI_LD_PREFERENCE};
use bon::bon;
use http::{HeaderName, HeaderValue};
use std::iter;

/// The header name.
static PREFER: HeaderName = HeaderName::from_static("prefer");

/// The name of the preference for where the `@context` of a `GeoJSON` body goes (clause 6.3.6).
const BODY_PREFERENCE: &str = "body";

/// Where a response in the `GeoJSON` representation carries its `@context`.
///
/// `body=json` asks for it in a `Link` header and a body without one, `body=ld+json`, the default, for it in the body
/// (NGSI-LD clause 6.3.6).
///
/// ```
/// use ngsi_ld::http::headers::GeoJsonBody;
///
/// assert_eq!(GeoJsonBody::Json.to_string(), "json");
/// assert_eq!("ld+json".parse::<GeoJsonBody>(), Ok(GeoJsonBody::LdJson));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum GeoJsonBody {
    /// `json`: the `@context` in a `Link` header.
    #[strum(serialize = "json")]
    Json,
    /// `ld+json`: the `@context` in the body.
    #[strum(serialize = "ld+json")]
    LdJson,
}

/// Why a `Prefer` header could not be built.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PreferError {
    /// The header states no preference.
    #[error("a Prefer header needs at least one preference")]
    Empty,
    /// The rendering cannot be carried in a header field.
    #[error("the preferences cannot be sent in a header")]
    FieldValue(#[from] InvalidFieldValue),
}

/// The `Prefer` header as NGSI-LD uses it.
///
/// As NGSI-LD clause 6.3.6 uses it, the header states the version of the specification a client wants the payloads in
/// (`ngsi-ld=<version>`, clause 4.3.6.8), which a broker confirms with `Preference-Applied`, and where a `GeoJSON`
/// response carries its `@context` (`body=json` or `body=ld+json`).
///
/// Decoding reads the first `ngsi-ld` and the first `body` preference of the field and ignores every other one; a
/// field with neither, or with a value this crate cannot read, does not decode. The field value is rendered on
/// construction, so encoding cannot fail.
///
/// ```
/// use headers::{Header, HeaderMapExt};
/// use http::HeaderMap;
/// use ngsi_ld::http::headers::{GeoJsonBody, Prefer, PreferError, SpecVersion};
///
/// let prefer = Prefer::builder()
///     .version(SpecVersion::new(1, 9))
///     .geo_json_body(GeoJsonBody::Json)
///     .build()?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(prefer);
/// assert_eq!(headers[Prefer::name()], "ngsi-ld=1.9, body=json");
/// assert!(matches!(Prefer::builder().build(), Err(PreferError::Empty)));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Prefer {
    /// The version the payloads are wanted in.
    version: Option<SpecVersion>,
    /// Where a `GeoJSON` response carries its `@context`.
    geo_json_body: Option<GeoJsonBody>,
    /// The preferences, rendered as a field value.
    encoded: HeaderValue,
}

#[bon]
impl Prefer {
    /// A `Prefer` header stating the preferences given, in the order `ngsi-ld`, `body`.
    ///
    /// # Errors
    ///
    /// Returns [`PreferError::Empty`] when no preference is given, and [`PreferError::FieldValue`] when the
    /// rendering cannot be carried in a header field.
    #[builder]
    pub fn new(version: Option<SpecVersion>, geo_json_body: Option<GeoJsonBody>) -> Result<Prefer, PreferError> {
        let preferences: Vec<String> = [
            version.map(|version| format!("{NGSI_LD_PREFERENCE}={version}")),
            geo_json_body.map(|body| format!("{BODY_PREFERENCE}={body}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        if preferences.is_empty() {
            return Err(PreferError::Empty);
        }
        let encoded = encode_text(&preferences.join(", "))?;
        Ok(Prefer {
            version,
            geo_json_body,
            encoded,
        })
    }

    /// The version the payloads are wanted in.
    #[must_use]
    pub const fn version(&self) -> Option<SpecVersion> {
        self.version
    }

    /// Where a `GeoJSON` response is asked to carry its `@context`.
    #[must_use]
    pub const fn geo_json_body(&self) -> Option<GeoJsonBody> {
        self.geo_json_body
    }
}

impl headers::Header for Prefer {
    fn name() -> &'static HeaderName {
        &PREFER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Prefer, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let lines: Vec<&HeaderValue> = values.collect();
        let version = first_value(&lines, NGSI_LD_PREFERENCE)?
            .map(str::parse)
            .transpose()
            .map_err(|_| headers::Error::invalid())?;
        let geo_json_body = first_value(&lines, BODY_PREFERENCE)?
            .map(str::parse)
            .transpose()
            .map_err(|_| headers::Error::invalid())?;
        Prefer::builder()
            .maybe_version(version)
            .maybe_geo_json_body(geo_json_body)
            .build()
            .map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // `encode` borrows the header; cloning a `HeaderValue` only bumps the reference count of its shared bytes.
        values.extend(iter::once(self.encoded.clone()));
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{GeoJsonBody, Prefer, SpecVersion, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn encoded(prefer: &Prefer) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        prefer.encode(&mut values);
        values
    }

    #[test]
    fn a_version_encodes_as_its_preference() {
        let prefer = Prefer::builder().version(SpecVersion::new(1, 9)).build().unwrap();
        assert_eq!(encoded(&prefer), vec![HeaderValue::from_static("ngsi-ld=1.9")]);
    }

    #[test]
    fn both_preferences_encode_in_order_and_decode_back() {
        let prefer = Prefer::builder()
            .version(SpecVersion::with_patch(1, 8, 1))
            .geo_json_body(GeoJsonBody::LdJson)
            .build()
            .unwrap();
        assert_eq!(encoded(&prefer), vec![HeaderValue::from_static("ngsi-ld=1.8.1, body=ld+json")]);
        assert_eq!(Prefer::decode(&mut encoded(&prefer).iter()).ok(), Some(prefer));
    }

    #[test]
    fn decoding_reads_the_first_of_each_and_ignores_the_rest() {
        let prefer = decode_line::<Prefer>("respond-async, BODY=\"json\"; strict, ngsi-ld=1.9, ngsi-ld=1.7").unwrap();
        assert_eq!(
            (prefer.version(), prefer.geo_json_body()),
            (Some(SpecVersion::new(1, 9)), Some(GeoJsonBody::Json))
        );
    }

    #[test]
    fn a_field_without_a_readable_preference_does_not_decode() {
        for raw in ["respond-async", "ngsi-ld=latest", "body=xml", "return=minimal", ""] {
            assert_eq!(decode_line::<Prefer>(raw), None, "{raw}");
        }
    }
}
