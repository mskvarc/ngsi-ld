//! The `Location` header (RFC 9110 section 10.2.2, NGSI-LD clause 6.4.3.1).

use crate::http::headers::{InvalidFieldValue, text_field::TextField};
use http::{HeaderName, HeaderValue, header::LOCATION};
use iri_rs::IriRefBuf;

/// The `Location` header: where a created resource lives, such as the relative path of an entity a Create Entity
/// operation made (NGSI-LD clause 6.4.3.1).
///
/// RFC 9110 makes the value a URI reference, so a relative path is kept as written. The field value is rendered
/// once, by [`Location::new`], and kept: an IRI holding non-ASCII characters cannot be carried verbatim, and
/// [`headers::Header::encode`] cannot fail.
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::http::headers::Location;
///
/// let location = Location::new(IriRefBuf::new(
///     "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567",
/// )?)?;
/// assert_eq!(
///     location.as_iri_ref().as_str(),
///     "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location(TextField<IriRefBuf>);

impl Location {
    /// A `Location` naming `target`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the reference cannot be carried in a header field.
    pub fn new(target: IriRefBuf) -> Result<Location, InvalidFieldValue> {
        TextField::new(target).map(Location)
    }

    /// The URI reference.
    #[must_use]
    pub const fn as_iri_ref(&self) -> &IriRefBuf {
        self.0.value()
    }

    /// Consumes the header and returns the URI reference.
    #[must_use]
    pub fn into_iri_ref(self) -> IriRefBuf {
        self.0.into_value()
    }
}

impl headers::Header for Location {
    fn name() -> &'static HeaderName {
        &LOCATION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Location, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        TextField::decode(values).map(Location)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{Location, field_value::tests::decode_line};
    use headers::Header;
    use http::HeaderValue;
    use iri_rs::IriRefBuf;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_relative_entity_path_decodes_as_written() {
        let location = decode_line::<Location>("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567").unwrap();
        assert_eq!(location.as_iri_ref().as_str(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567");
    }

    #[test]
    fn a_value_that_is_not_a_uri_reference_is_rejected() {
        assert_eq!(decode_line::<Location>("/entities/a b"), None);
        assert_eq!(decode_line::<Location>("http://[broken/"), None);
    }

    #[test]
    fn a_reference_with_non_ascii_characters_cannot_be_carried() {
        assert!(Location::new(IriRefBuf::new("/entities/caf\u{e9}").unwrap()).is_err());
    }

    #[test]
    fn a_location_encodes_its_reference_and_decodes_back() {
        let location = Location::new(IriRefBuf::new("https://broker.example/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:1").unwrap()).unwrap();
        let mut encoded = Vec::new();
        location.encode(&mut encoded);
        assert_eq!(
            encoded,
            vec![HeaderValue::from_static(
                "https://broker.example/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:1"
            )]
        );
        assert_eq!(Location::decode(&mut encoded.iter()).ok(), Some(location.clone()));
        assert_eq!(
            location.into_iri_ref().as_str(),
            "https://broker.example/ngsi-ld/v1/subscriptions/urn:ngsi-ld:Subscription:1"
        );
    }
}
