//! The three media types of the NGSI-LD API (NGSI-LD clause 6.3.4), which documents name as well as headers: the
//! `accept` of a notification endpoint (clause 5.2.15), the `accept` and `contentType` of a context source (clause
//! 4.3.6.6) and the `Content-Type` of an MQTT notification (clause 7.2).

use mediatype::{MediaType, MediaTypeBuf, MediaTypeError, ReadParams, names};
use std::str::FromStr;

/// `application/json`
pub const APPLICATION_JSON: MediaType<'static> = MediaType::new(names::APPLICATION, names::JSON);

/// `application/ld+json`
pub const APPLICATION_LD_JSON: MediaType<'static> = MediaType::from_parts(names::APPLICATION, names::LD, Some(names::JSON), &[]);

/// `application/geo+json`
pub const APPLICATION_GEO_JSON: MediaType<'static> = MediaType::from_parts(names::APPLICATION, names::GEO, Some(names::JSON), &[]);

/// Why text is not an NGSI-LD media type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum NgsiLdMediaTypeError {
    /// The text is not a media type.
    #[error("the text is not a media type")]
    Syntax(#[from] MediaTypeError),
    /// The text is a media type other than the three of the NGSI-LD API, or one of them with parameters.
    #[error("{media_type} is not an NGSI-LD media type")]
    Other {
        /// The media type.
        media_type: MediaTypeBuf,
    },
}

/// A media type an NGSI-LD document is exchanged as.
///
/// NGSI-LD clause 6.3.4.
///
/// The enum is exhaustive: clause 6.3.4 closes the set of representations to these three. Each variant is the
/// structured [`media_type`](Self::media_type) of the clause; its text is that media type's rendering, and text
/// is read by parsing it as a media type.
///
/// ```
/// use ngsi_ld::model::{APPLICATION_GEO_JSON, NgsiLdMediaType};
///
/// assert_eq!(NgsiLdMediaType::JsonLd.to_string(), "application/ld+json");
/// assert_eq!(NgsiLdMediaType::GeoJson.media_type(), APPLICATION_GEO_JSON);
/// assert_eq!(
///     "Application/JSON".parse::<NgsiLdMediaType>()?,
///     NgsiLdMediaType::Json
/// );
/// assert!("application/xml".parse::<NgsiLdMediaType>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{}", self.media_type())]
pub enum NgsiLdMediaType {
    /// `application/json`: the `@context` travels in a `Link` header.
    Json,
    /// `application/ld+json`: the `@context` travels in the body.
    JsonLd,
    /// `application/geo+json`: the `GeoJSON` representation of entities (clause 6.3.15).
    GeoJson,
}

impl NgsiLdMediaType {
    /// The structured media type.
    #[must_use]
    pub const fn media_type(self) -> MediaType<'static> {
        match self {
            NgsiLdMediaType::Json => APPLICATION_JSON,
            NgsiLdMediaType::JsonLd => APPLICATION_LD_JSON,
            NgsiLdMediaType::GeoJson => APPLICATION_GEO_JSON,
        }
    }

    /// The NGSI-LD media type `media_type` names, if any.
    ///
    /// Parameters such as `charset` are ignored, and names compare case-insensitively (RFC 9110 section 8.3.1).
    /// `application/json` matches only without a suffix, so `application/merge-patch+json` is not JSON here. A media
    /// range such as `*/*` names no single type and matches nothing: the client always states the one type it sends
    /// or asks for.
    #[must_use]
    pub fn from_media_type(media_type: &MediaType<'_>) -> Option<NgsiLdMediaType> {
        let essence = media_type.essence();
        [NgsiLdMediaType::Json, NgsiLdMediaType::JsonLd, NgsiLdMediaType::GeoJson]
            .into_iter()
            .find(|candidate| essence == candidate.media_type())
    }
}

/// Reads the media type a document names: one of the three, without parameters, whose names compare
/// case-insensitively (RFC 9110 section 8.3.1).
// No derive reads a variant through a media type parser.
impl FromStr for NgsiLdMediaType {
    type Err = NgsiLdMediaTypeError;

    fn from_str(text: &str) -> Result<NgsiLdMediaType, NgsiLdMediaTypeError> {
        let media_type = MediaType::parse(text)?;
        let known = if media_type.params().next().is_none() {
            NgsiLdMediaType::from_media_type(&media_type)
        } else {
            None
        };
        known.ok_or_else(|| NgsiLdMediaTypeError::Other {
            media_type: MediaTypeBuf::from(media_type),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{NgsiLdMediaType, NgsiLdMediaTypeError};
    use mediatype::MediaType;
    use pretty_assertions::assert_eq;

    fn recognise(raw: &str) -> Option<NgsiLdMediaType> {
        NgsiLdMediaType::from_media_type(&MediaType::parse(raw).unwrap())
    }

    #[test]
    fn each_variant_renders_its_media_type_and_reads_back() {
        for variant in [NgsiLdMediaType::Json, NgsiLdMediaType::JsonLd, NgsiLdMediaType::GeoJson] {
            assert_eq!(variant.to_string(), variant.media_type().to_string());
            assert_eq!(variant.to_string().parse::<NgsiLdMediaType>(), Ok(variant));
            assert_eq!(NgsiLdMediaType::from_media_type(&variant.media_type()), Some(variant));
        }
        assert_eq!(NgsiLdMediaType::JsonLd.to_string(), "application/ld+json");
    }

    #[test]
    fn each_media_type_is_recognised_whatever_its_parameters_or_case() {
        assert_eq!(recognise("application/json"), Some(NgsiLdMediaType::Json));
        assert_eq!(recognise("application/ld+json; charset=utf-8"), Some(NgsiLdMediaType::JsonLd));
        assert_eq!(recognise("Application/GEO+JSON"), Some(NgsiLdMediaType::GeoJson));
    }

    #[test]
    fn media_ranges_patch_media_types_and_other_types_are_not_ngsi_ld_media_types() {
        for raw in [
            "*/*",
            "application/*",
            "application/merge-patch+json",
            "application/json-patch+json",
            "text/*",
            "application/xml",
        ] {
            assert_eq!(recognise(raw), None, "{raw}");
        }
    }

    #[test]
    fn a_document_names_a_media_type_without_parameters() {
        assert_eq!("APPLICATION/LD+JSON".parse::<NgsiLdMediaType>(), Ok(NgsiLdMediaType::JsonLd));
        assert!(matches!(
            "application/json; charset=utf-8".parse::<NgsiLdMediaType>(),
            Err(NgsiLdMediaTypeError::Other { .. })
        ));
        assert!(matches!("application/xml".parse::<NgsiLdMediaType>(), Err(NgsiLdMediaTypeError::Other { .. })));
        assert!(matches!("not a media type".parse::<NgsiLdMediaType>(), Err(NgsiLdMediaTypeError::Syntax(_))));
    }
}
