//! The `NGSILD-EntityMap` header (NGSI-LD clauses 6.4.3.2 and 6.5.3.1).

use crate::{
    http::{
        headers::{InvalidFieldValue, text_field::TextField},
        path::id_from_location,
    },
    registration::EntityMapId,
};
use http::{HeaderName, HeaderValue};
use iri_rs::IriRefBuf;

/// The header name.
static NGSILD_ENTITY_MAP: HeaderName = HeaderName::from_static("ngsild-entitymap");

/// The `NGSILD-EntityMap` header: the resource URI of an entity map.
///
/// NGSI-LD clauses 4.5.25 and 5.2.39.
///
/// In a Retrieve Entity or Query Entities request it names an existing map for the broker to use; in the response
/// it names the map the broker used, once the request asked for one (Tables 6.4.3.2-2, 6.4.3.2-3, 6.5.3.1-2 and
/// 6.5.3.1-3). The value is a URI reference, kept as written.
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::http::headers::NgsiLdEntityMap;
///
/// let map = NgsiLdEntityMap::try_new(IriRefBuf::new(
///     "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1",
/// )?)?;
/// assert_eq!(
///     map.as_iri_ref().as_str(),
///     "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NgsiLdEntityMap(TextField<IriRefBuf>);

impl NgsiLdEntityMap {
    /// The header naming the map at `target`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the reference cannot be carried in a header field.
    pub fn try_new(target: IriRefBuf) -> Result<NgsiLdEntityMap, InvalidFieldValue> {
        TextField::new(target).map(NgsiLdEntityMap)
    }

    /// The URI reference of the map.
    #[must_use]
    pub const fn as_iri_ref(&self) -> &IriRefBuf {
        self.0.value()
    }

    /// The id of the map, read from the last path segment of its location, which Table 6.2-1 makes the map's id
    /// (`/entityMaps/{entityMapId}`), percent-decoded; `None` when that segment is not a URI. With it, the map a
    /// query page or a creation names can be retrieved, updated or deleted.
    ///
    /// ```
    /// use iri_rs::IriRefBuf;
    /// use ngsi_ld::http::headers::NgsiLdEntityMap;
    ///
    /// let target = IriRefBuf::new("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1")?;
    /// let map = NgsiLdEntityMap::try_new(target)?;
    /// assert_eq!(
    ///     map.id().map(|id| id.to_string()).as_deref(),
    ///     Some("urn:ngsi-ld:EntityMap:1")
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn id(&self) -> Option<EntityMapId> {
        id_from_location(self.as_iri_ref().as_str())
    }

    /// Consumes the header and returns the URI reference of the map.
    #[must_use]
    pub fn into_iri_ref(self) -> IriRefBuf {
        self.0.into_value()
    }
}

impl headers::Header for NgsiLdEntityMap {
    fn name() -> &'static HeaderName {
        &NGSILD_ENTITY_MAP
    }

    fn decode<'i, I>(values: &mut I) -> Result<NgsiLdEntityMap, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        TextField::decode(values).map(NgsiLdEntityMap)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::NgsiLdEntityMap;
    use headers::Header;
    use http::HeaderValue;
    use iri_rs::IriRefBuf;
    use pretty_assertions::assert_eq;
    use std::iter;

    #[test]
    fn the_header_name_is_the_spec_token() {
        assert_eq!(NgsiLdEntityMap::name().as_str(), "ngsild-entitymap");
    }

    #[test]
    fn a_map_location_encodes_and_decodes_back() {
        let map = NgsiLdEntityMap::try_new(IriRefBuf::new("urn:ngsi-ld:EntityMap:7").unwrap()).unwrap();
        let mut encoded = Vec::new();
        map.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("urn:ngsi-ld:EntityMap:7")]);
        assert_eq!(NgsiLdEntityMap::decode(&mut encoded.iter()).ok(), Some(map));
    }

    #[test]
    fn a_value_that_is_not_a_uri_reference_is_rejected() {
        assert_eq!(NgsiLdEntityMap::decode(&mut iter::once(&HeaderValue::from_static("a b"))).ok(), None);
    }
}
