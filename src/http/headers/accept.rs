// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The `Accept` header (RFC 9110 section 12.5.1, NGSI-LD clause 6.3.4).
//!
//! Portions of this file are derived from the `Accept` implementation in <https://github.com/hyperium/headers>.
//! This file, including local modifications, is distributed under the MIT License.

use crate::{
    http::headers::{
        ListFieldError,
        MediaRange,
        field_list::{FieldList, decode_elements},
    },
    model::NgsiLdMediaType,
    query::{Quality, Weighted},
};
use http::{HeaderName, HeaderValue, header::ACCEPT};
use mediatype::{MediaType, ReadParams, WriteParams, names};
use std::iter;

/// An `Accept` header (RFC 9110 section 12.5.1): the media ranges a client takes a response in, in order, each
/// with an optional weight.
///
/// A client asks for one NGSI-LD representation with [`Accept::new`], the form the request pipeline sends (NGSI-LD
/// clause 6.3.4); a weighted list, a wildcard range or a media type outside NGSI-LD, which a broker must refuse
/// with 406 Not Acceptable, is built with [`Accept::try_from_entries`]. Decoding reads any valid list, across all its
/// lines, keeping its order and weights, so a mock server can check what a broker forwarded. An `Accept` with no
/// element does not decode: it states no preference, which is said by sending no `Accept` at all.
///
/// A list is built only when its rendering is a field value, and a media type without parameters always renders as
/// one, so encoding cannot fail.
///
/// ```
/// use headers::HeaderMapExt;
/// use http::{HeaderMap, header};
/// use mediatype::names;
/// use ngsi_ld::{
///     http::headers::{Accept, MediaRange},
///     model::NgsiLdMediaType,
///     query::{Quality, Weighted},
/// };
///
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(Accept::new(NgsiLdMediaType::GeoJson));
/// assert_eq!(headers[&header::ACCEPT], "application/geo+json");
///
/// let weighted = Accept::try_from_entries([
///     Weighted::new(NgsiLdMediaType::Json).with_quality(Quality::try_from_thousandths(500)?),
///     Weighted::new(NgsiLdMediaType::JsonLd),
/// ])?;
/// headers.typed_insert(weighted);
/// assert_eq!(
///     headers[&header::ACCEPT],
///     "application/json;q=0.5, application/ld+json"
/// );
///
/// let listed = Accept::try_from_entries(
///     [
///         NgsiLdMediaType::GeoJson,
///         NgsiLdMediaType::JsonLd,
///         NgsiLdMediaType::Json,
///     ]
///     .map(Weighted::new),
/// )?;
/// headers.typed_insert(listed);
/// assert_eq!(
///     headers[&header::ACCEPT],
///     "application/geo+json, application/ld+json, application/json"
/// );
///
/// let ranges = Accept::try_from_entries([
///     Weighted::new(MediaRange::try_any_subtype_of(names::TEXT)?),
///     Weighted::new("application/xml".parse::<MediaRange>()?),
///     Weighted::new(MediaRange::any()).with_quality(Quality::try_from_thousandths(100)?),
/// ])?;
/// headers.typed_insert(ranges);
/// assert_eq!(
///     headers[&header::ACCEPT],
///     "text/*, application/xml, */*;q=0.1"
/// );
/// let decoded = headers.typed_get::<Accept>();
/// let weights: Option<Vec<Option<Quality>>> =
///     decoded.map(|accept| accept.entries().iter().map(Weighted::quality).collect());
/// assert_eq!(
///     weights,
///     Some(vec![None, None, Some(Quality::try_from_thousandths(100)?)])
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Accept(FieldList<Weighted<MediaRange>>);

impl Accept {
    /// An `Accept` asking for `media_type` alone.
    #[must_use]
    pub fn new(media_type: NgsiLdMediaType) -> Accept {
        Accept(FieldList::known(vec![Weighted::new(media_type)]))
    }

    /// An `Accept` asking for `first`, then for each of `then`, in order and unweighted.
    ///
    /// ```
    /// use headers::HeaderMapExt;
    /// use http::{HeaderMap, header};
    /// use ngsi_ld::{http::headers::Accept, model::NgsiLdMediaType};
    ///
    /// let mut headers = HeaderMap::new();
    /// headers.typed_insert(Accept::in_order(
    ///     NgsiLdMediaType::JsonLd,
    ///     [NgsiLdMediaType::Json],
    /// ));
    /// assert_eq!(
    ///     headers[&header::ACCEPT],
    ///     "application/ld+json, application/json"
    /// );
    /// ```
    #[must_use]
    pub fn in_order(first: NgsiLdMediaType, then: impl IntoIterator<Item = NgsiLdMediaType>) -> Accept {
        let entries = iter::once(first).chain(then).map(Weighted::new).collect();
        Accept(FieldList::known(entries))
    }

    /// An `Accept` taking any media type: `*/*`.
    #[must_use]
    pub fn any() -> Accept {
        Accept(FieldList::known(vec![Weighted::new(MediaRange::any())]))
    }

    /// An `Accept` listing `entries`, in order.
    ///
    /// # Errors
    ///
    /// Returns [`ListFieldError::Empty`] when `entries` is empty, and [`ListFieldError::FieldValue`] when the
    /// rendering cannot be carried in a header field, which validated media ranges never cause.
    pub fn try_from_entries(entries: impl IntoIterator<Item = Weighted<MediaRange>>) -> Result<Accept, ListFieldError> {
        FieldList::non_empty(entries.into_iter().collect()).map(Accept)
    }

    /// The media ranges, in order, with their weights.
    #[must_use]
    pub fn entries(&self) -> &[Weighted<MediaRange>] {
        self.0.elements()
    }
}

impl From<NgsiLdMediaType> for Accept {
    fn from(media_type: NgsiLdMediaType) -> Accept {
        Accept::new(media_type)
    }
}

/// Reads one element: a media range, then its weight as a last `q` parameter (RFC 9110 sections 12.4.2 and 12.5.1).
///
/// RFC 9110 has no parameter after the weight, so a `q` anywhere but last, or twice, is refused.
fn parse_entry(element: &str) -> Result<Weighted<MediaRange>, headers::Error> {
    let mut media_type = MediaType::parse(element).map_err(|_| headers::Error::invalid())?;
    let weights = media_type.params().filter(|(name, _)| *name == names::Q).count();
    let quality = if weights == 0 {
        None
    } else {
        let (_, weight) = media_type
            .params
            .last()
            .filter(|(name, _)| weights == 1 && *name == names::Q)
            .ok_or_else(headers::Error::invalid)?;
        Some(weight.as_str().parse::<Quality>().map_err(|_| headers::Error::invalid())?)
    };
    media_type.remove_params(names::Q);
    let range = MediaRange::try_from(media_type).map_err(|_| headers::Error::invalid())?;
    Ok(Weighted::from_parts(range, quality))
}

impl headers::Header for Accept {
    fn name() -> &'static HeaderName {
        &ACCEPT
    }

    fn decode<'i, I>(values: &mut I) -> Result<Accept, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        Accept::try_from_entries(decode_elements(values, parse_entry)?).map_err(|_| headers::Error::invalid())
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{Accept, ListFieldError, MediaRange, field_value::tests::decode_line},
        model::NgsiLdMediaType,
        query::{Quality, Weighted},
    };
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    fn encoded(accept: &Accept) -> Vec<HeaderValue> {
        let mut values = Vec::new();
        accept.encode(&mut values);
        values
    }

    fn rendered(accept: &Accept) -> Vec<(String, Option<u16>)> {
        accept
            .entries()
            .iter()
            .map(|entry| (entry.value().to_string(), entry.quality().map(Quality::thousandths)))
            .collect()
    }

    #[test]
    fn each_ngsi_ld_media_type_encodes_alone_and_decodes_back() {
        for media_type in [NgsiLdMediaType::Json, NgsiLdMediaType::JsonLd, NgsiLdMediaType::GeoJson] {
            let accept = Accept::new(media_type);
            assert_eq!(encoded(&accept), vec![HeaderValue::try_from(media_type.to_string()).unwrap()]);
            assert_eq!(Accept::decode(&mut encoded(&accept).iter()).ok(), Some(accept.clone()));
            assert_eq!(Accept::from(media_type), accept);
            assert_eq!(Accept::try_from_entries([Weighted::new(media_type)]).ok(), Some(accept));
        }
    }

    #[test]
    fn the_any_range_encodes_as_a_star_pair() {
        assert_eq!(encoded(&Accept::any()), vec![HeaderValue::from_static("*/*")]);
        assert_eq!(decode_line::<Accept>("*/*"), Some(Accept::any()));
        assert_eq!(Accept::try_from_entries([Weighted::new(MediaRange::any())]).ok(), Some(Accept::any()));
    }

    #[test]
    fn a_weighted_list_keeps_its_order_and_weights_through_a_round_trip() {
        let accept = Accept::try_from_entries([
            Weighted::new(NgsiLdMediaType::Json).with_quality(Quality::try_from_thousandths(500).unwrap()),
            Weighted::new(NgsiLdMediaType::JsonLd),
            Weighted::new("text/plain; charset=utf-8".parse::<MediaRange>().unwrap()).with_quality(Quality::try_from_thousandths(0).unwrap()),
        ])
        .unwrap();
        assert_eq!(
            encoded(&accept),
            vec![HeaderValue::from_static(
                "application/json;q=0.5, application/ld+json, text/plain; charset=utf-8;q=0"
            )]
        );
        assert_eq!(Accept::decode(&mut encoded(&accept).iter()).ok(), Some(accept));
    }

    #[test]
    fn any_valid_list_decodes_in_order_across_lines() {
        let lines = [
            HeaderValue::from_static("text/html, application/xml;q=0.9"),
            HeaderValue::from_static("*/*; q=0.8, Text/*"),
        ];
        let accept = Accept::decode(&mut lines.iter()).unwrap();
        assert_eq!(
            rendered(&accept),
            vec![
                ("text/html".to_owned(), None),
                ("application/xml".to_owned(), Some(900)),
                ("*/*".to_owned(), Some(800)),
                ("text/*".to_owned(), None),
            ]
        );
    }

    #[test]
    fn a_media_type_parameter_is_kept_apart_from_the_weight() {
        let accept = decode_line::<Accept>("application/ld+json; profile=\"https://example.org/p\"; q=0.7").unwrap();
        assert_eq!(
            rendered(&accept),
            vec![("application/ld+json; profile=\"https://example.org/p\"".to_owned(), Some(700))]
        );
        assert_eq!(accept.entries()[0].value().ngsi_ld_media_type(), Some(NgsiLdMediaType::JsonLd));
    }

    #[test]
    fn a_malformed_list_or_weight_does_not_decode() {
        for raw in [
            "",
            " , ",
            "json",
            "application/json;q=2",
            "application/json;q=0.1234",
            "application/json;q=\"0.5\"",
            "application/json;q=0.5;charset=utf-8",
            "application/json;q=0.5;q=0.4",
            "*/json",
            "application/json, text",
            "text/plain; a=\"open",
        ] {
            assert_eq!(decode_line::<Accept>(raw), None, "{raw}");
        }
    }

    #[test]
    fn an_empty_list_is_refused_on_construction() {
        assert!(matches!(Accept::try_from_entries(Vec::new()), Err(ListFieldError::Empty)));
    }
}
