//! The media range an `Accept` header asks for (RFC 9110 section 12.5.1).

use crate::{http::headers::field_value::is_field_text, model::NgsiLdMediaType};
use mediatype::{MediaType, MediaTypeBuf, MediaTypeError, Name, ReadParams, names};
use std::str::FromStr;

/// Why a media type is not a media range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum MediaRangeError {
    /// The text is not a media type.
    #[error("the media range is not a media type")]
    Syntax(#[from] MediaTypeError),
    /// A `*` stands for something other than the whole subtype, or the whole type of `*/*`.
    #[error("a media range uses * only as a whole subtype, or as the whole type of */*")]
    Wildcard,
    /// The media range carries a `q` parameter, which is the weight of an `Accept` element and not a parameter.
    #[error("a media range cannot carry the q parameter")]
    Weight,
    /// A parameter value holds a character a header field cannot carry.
    #[error("a media range parameter value holds a character a header field cannot carry")]
    ParameterValue,
}

/// A media range: `*/*`, `type/*` or a media type, with its parameters.
///
/// RFC 9110 section 12.5.1.
///
/// Any media type can be asked for, not only the three of NGSI-LD clause 6.3.4, so a request a broker must refuse
/// with 406 Not Acceptable, such as one for `application/xml`, can be written. Names are kept in lowercase, the
/// form RFC 9110 section 8.3.1 makes them equal to; parameter values are kept as written.
///
/// ```
/// use mediatype::names;
/// use ngsi_ld::{
///     http::headers::{MediaRange, MediaRangeError},
///     model::NgsiLdMediaType,
/// };
///
/// assert_eq!(MediaRange::any().to_string(), "*/*");
/// assert_eq!(
///     MediaRange::try_any_subtype_of(names::TEXT)?.to_string(),
///     "text/*"
/// );
/// assert_eq!(
///     MediaRange::from(NgsiLdMediaType::GeoJson).to_string(),
///     "application/geo+json"
/// );
/// let xml: MediaRange = "Application/XML".parse()?;
/// assert_eq!(xml.to_string(), "application/xml");
/// assert_eq!(
///     "application/*+json".parse::<MediaRange>(),
///     Err(MediaRangeError::Wildcard)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{_0}")]
pub struct MediaRange(MediaTypeBuf);

impl MediaRange {
    /// `*/*`: any media type.
    #[must_use]
    pub fn any() -> MediaRange {
        MediaRange(MediaTypeBuf::new(names::_STAR, names::_STAR))
    }

    /// `type/*`: any subtype of `ty`, such as `application/*`.
    ///
    /// # Errors
    ///
    /// Returns [`MediaRangeError::Wildcard`] when `ty` holds a `*` and is not `*` itself; `*` gives `*/*`.
    pub fn try_any_subtype_of(ty: Name<'_>) -> Result<MediaRange, MediaRangeError> {
        MediaRange::try_from(MediaType::new(ty, names::_STAR))
    }

    /// The media range as a structured media type, with its parameters.
    #[must_use]
    pub fn media_type(&self) -> MediaType<'_> {
        self.0.to_ref()
    }

    /// The NGSI-LD media type this range names exactly, whatever its parameters (NGSI-LD clause 6.3.4).
    ///
    /// A range with a `*`, such as `application/*`, names no single type and gives `None`.
    #[must_use]
    pub fn ngsi_ld_media_type(&self) -> Option<NgsiLdMediaType> {
        NgsiLdMediaType::from_media_type(&self.0.essence())
    }
}

/// Whether `name` holds a `*`.
fn has_star(name: Name<'_>) -> bool {
    name.as_str().contains('*')
}

impl TryFrom<MediaType<'_>> for MediaRange {
    type Error = MediaRangeError;

    fn try_from(media_type: MediaType<'_>) -> Result<MediaRange, MediaRangeError> {
        let wildcard_type = media_type.ty == names::_STAR;
        let wildcard_subtype = media_type.subty == names::_STAR;
        let well_placed = match (wildcard_type, wildcard_subtype) {
            (true, true) => media_type.suffix.is_none(),
            (false, true) => media_type.suffix.is_none() && !has_star(media_type.ty),
            (true, false) => false,
            (false, false) => !has_star(media_type.ty) && !has_star(media_type.subty) && !media_type.suffix.is_some_and(has_star),
        };
        if !well_placed {
            return Err(MediaRangeError::Wildcard);
        }
        if media_type.get_param(names::Q).is_some() {
            return Err(MediaRangeError::Weight);
        }
        if !media_type.params().all(|(_, value)| is_field_text(value.as_str())) {
            return Err(MediaRangeError::ParameterValue);
        }
        Ok(MediaRange(MediaTypeBuf::from(media_type).canonicalize()))
    }
}

// A derived parser would skip the checks of `TryFrom<MediaType>`.
impl FromStr for MediaRange {
    type Err = MediaRangeError;

    fn from_str(text: &str) -> Result<MediaRange, MediaRangeError> {
        MediaRange::try_from(MediaType::parse(text)?)
    }
}

impl From<NgsiLdMediaType> for MediaRange {
    fn from(media_type: NgsiLdMediaType) -> MediaRange {
        MediaRange(MediaTypeBuf::from(media_type.media_type()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{MediaRange, MediaRangeError},
        model::NgsiLdMediaType,
    };
    use mediatype::{MediaTypeError, Name, names};
    use pretty_assertions::assert_eq;

    fn parse(text: &str) -> Result<String, MediaRangeError> {
        text.parse::<MediaRange>().map(|range| range.to_string())
    }

    #[test]
    fn every_kind_of_range_parses_and_renders_in_lowercase_with_its_parameters() {
        assert_eq!(parse("*/*"), Ok("*/*".to_owned()));
        assert_eq!(parse("TEXT/*"), Ok("text/*".to_owned()));
        assert_eq!(parse("application/xml"), Ok("application/xml".to_owned()));
        assert_eq!(parse("text/plain;charset=UTF-8"), Ok("text/plain; charset=UTF-8".to_owned()));
        assert_eq!(parse("application/LD+JSON"), Ok("application/ld+json".to_owned()));
    }

    #[test]
    fn a_misplaced_wildcard_is_rejected() {
        for text in [
            "*/json",
            "application/*+json",
            "te*t/plain",
            "application/js*n",
            "*/*+json",
            "application/ld+js*",
        ] {
            assert_eq!(parse(text), Err(MediaRangeError::Wildcard), "{text}");
        }
    }

    #[test]
    fn a_q_parameter_or_an_uncarriable_parameter_value_is_rejected() {
        assert_eq!(parse("application/json;q=0.5"), Err(MediaRangeError::Weight));
        assert_eq!(parse("application/json;Q=1"), Err(MediaRangeError::Weight));
        assert_eq!(parse("text/plain;a=\"caf\u{e9}\""), Err(MediaRangeError::ParameterValue));
        assert_eq!(parse("text/plain;a=\"\u{1}\""), Err(MediaRangeError::ParameterValue));
    }

    #[test]
    fn text_that_is_no_media_type_is_rejected() {
        assert_eq!(parse("json"), Err(MediaRangeError::Syntax(MediaTypeError::InvalidTypeName)));
        assert_eq!(
            parse("application/json, text/plain"),
            Err(MediaRangeError::Syntax(MediaTypeError::InvalidParams))
        );
    }

    #[test]
    fn the_constructors_build_the_wildcard_ranges() {
        assert_eq!(MediaRange::any().to_string(), "*/*");
        assert_eq!(
            MediaRange::try_any_subtype_of(names::APPLICATION).map(|range| range.to_string()),
            Ok("application/*".to_owned())
        );
        assert_eq!(MediaRange::try_any_subtype_of(names::_STAR), Ok(MediaRange::any()));
        let starred = Name::new("te*t").unwrap();
        assert_eq!(MediaRange::try_any_subtype_of(starred), Err(MediaRangeError::Wildcard));
    }

    #[test]
    fn an_ngsi_ld_media_type_converts_and_is_recognised_back() {
        for media_type in [NgsiLdMediaType::Json, NgsiLdMediaType::JsonLd, NgsiLdMediaType::GeoJson] {
            let range = MediaRange::from(media_type);
            assert_eq!(range.to_string(), media_type.to_string());
            assert_eq!(range.ngsi_ld_media_type(), Some(media_type));
            assert_eq!(range.media_type(), media_type.media_type());
        }
        assert_eq!(MediaRange::any().ngsi_ld_media_type(), None);
        assert_eq!(
            "application/json; charset=utf-8".parse::<MediaRange>().unwrap().ngsi_ld_media_type(),
            Some(NgsiLdMediaType::Json)
        );
    }
}
