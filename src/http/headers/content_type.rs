//! The `Content-Type` header restricted to the media types NGSI-LD bodies are sent as (NGSI-LD clause 6.3.4).

use crate::{
    http::headers::field_value::{decode_singleton, encode_text},
    model::NgsiLdMediaType,
};
use http::{HeaderName, HeaderValue, header::CONTENT_TYPE};
use mediatype::{MediaType, names};

/// `application/merge-patch+json` (IETF RFC 7396).
const APPLICATION_MERGE_PATCH_JSON: MediaType<'static> = MediaType::from_parts(names::APPLICATION, names::MERGE_PATCH, Some(names::JSON), &[]);

/// A `Content-Type` naming a media type an NGSI-LD body is sent as.
///
/// It is one of the NGSI-LD media types, or `application/merge-patch+json`, which NGSI-LD clause 6.3.4 admits for a
/// PATCH body.
///
/// Separate from the `headers` crate's own `ContentType`, which is untyped over `mime`. Decoding tolerates
/// parameters (`application/json; charset=utf-8`) and refuses media ranges and every other media type. Encoding
/// writes the [`media_type`](Self::media_type), without parameters.
///
/// The enum is exhaustive: clause 6.3.4 closes the set of media types a body is sent as.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::HeaderMap;
/// use ngsi_ld::{http::headers::ContentType, model::NgsiLdMediaType};
///
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(ContentType::NgsiLd(NgsiLdMediaType::JsonLd));
/// assert_eq!(
///     headers.typed_get::<ContentType>(),
///     Some(ContentType::NgsiLd(NgsiLdMediaType::JsonLd))
/// );
/// headers.typed_insert(ContentType::MergePatch);
/// assert_eq!(
///     headers.typed_get::<ContentType>(),
///     Some(ContentType::MergePatch)
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    /// One of the NGSI-LD media types.
    NgsiLd(NgsiLdMediaType),
    /// `application/merge-patch+json` (IETF RFC 7396), which clause 6.3.4 admits for a PATCH body and makes a broker
    /// read as `application/json`.
    MergePatch,
}

impl ContentType {
    /// The media type, without parameters.
    #[must_use]
    pub const fn media_type(self) -> MediaType<'static> {
        match self {
            ContentType::NgsiLd(media_type) => media_type.media_type(),
            ContentType::MergePatch => APPLICATION_MERGE_PATCH_JSON,
        }
    }
}

impl headers::Header for ContentType {
    fn name() -> &'static HeaderName {
        &CONTENT_TYPE
    }

    fn decode<'i, I>(values: &mut I) -> Result<ContentType, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let media_type = MediaType::parse(decode_singleton(values)?).map_err(|_| headers::Error::invalid())?;
        if media_type.essence() == APPLICATION_MERGE_PATCH_JSON {
            return Ok(ContentType::MergePatch);
        }
        NgsiLdMediaType::from_media_type(&media_type)
            .map(ContentType::NgsiLd)
            .ok_or_else(headers::Error::invalid)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // The names of a media type are restricted names (RFC 6838 section 4.2), all visible ASCII, so the rendering
        // of one without parameters is always a field value and the conversion never refuses it.
        values.extend(encode_text(&self.media_type().to_string()).ok());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{ContentType, field_value::tests::decode_line},
        model::NgsiLdMediaType,
    };
    use headers::Header;

    use mediatype::MediaType;
    use pretty_assertions::assert_eq;

    #[test]
    fn the_merge_patch_content_type_is_the_media_type_of_rfc_7396() {
        assert_eq!(ContentType::MergePatch.media_type(), MediaType::parse("application/merge-patch+json").unwrap());
    }

    #[test]
    fn each_ngsi_ld_media_type_decodes() {
        assert_eq!(decode_line::<ContentType>("application/json"), Some(ContentType::NgsiLd(NgsiLdMediaType::Json)));
        assert_eq!(
            decode_line::<ContentType>("application/ld+json"),
            Some(ContentType::NgsiLd(NgsiLdMediaType::JsonLd))
        );
        assert_eq!(
            decode_line::<ContentType>("application/geo+json"),
            Some(ContentType::NgsiLd(NgsiLdMediaType::GeoJson))
        );
    }

    #[test]
    fn the_merge_patch_media_type_decodes_whatever_its_parameters_or_case() {
        assert_eq!(decode_line::<ContentType>("application/merge-patch+json"), Some(ContentType::MergePatch));
        assert_eq!(
            decode_line::<ContentType>("Application/Merge-Patch+JSON; charset=utf-8"),
            Some(ContentType::MergePatch)
        );
    }

    #[test]
    fn a_charset_parameter_is_tolerated() {
        assert_eq!(
            decode_line::<ContentType>("application/json; charset=utf-8"),
            Some(ContentType::NgsiLd(NgsiLdMediaType::Json))
        );
    }

    #[test]
    fn media_ranges_and_other_types_are_rejected() {
        assert_eq!(decode_line::<ContentType>("application/json-patch+json"), None);
        assert_eq!(decode_line::<ContentType>("*/*"), None);
        assert_eq!(decode_line::<ContentType>("application/*"), None);
        assert_eq!(decode_line::<ContentType>("text/plain"), None);
        assert_eq!(decode_line::<ContentType>("not a media type"), None);
    }

    #[test]
    fn a_content_type_encodes_its_media_type_and_decodes_back() {
        let every = [
            ContentType::NgsiLd(NgsiLdMediaType::Json),
            ContentType::NgsiLd(NgsiLdMediaType::JsonLd),
            ContentType::NgsiLd(NgsiLdMediaType::GeoJson),
            ContentType::MergePatch,
        ];
        for header in every {
            let mut encoded = Vec::new();
            header.encode(&mut encoded);
            let text: Vec<&str> = encoded.iter().map(|value| value.to_str().unwrap()).collect();
            assert_eq!(text, [header.media_type().to_string()]);
            assert_eq!(ContentType::decode(&mut encoded.iter()).ok(), Some(header));
        }
    }
}
