//! A response header whose absence or malformation never turns a success into an error.

use headers::Header;
use http::{HeaderMap, HeaderValue};

/// What a successful response carries in a header it may omit.
///
/// The outcome does not depend on such a header, like the `Location` of a created entity (NGSI-LD clause 6.4.3.1).
///
/// The operation succeeded whichever variant this is: a Create Entity answered 201 created the entity even when its
/// `Location` cannot be read, and failing the call would make a retry hit `AlreadyExists`. A malformed field keeps
/// its raw lines, so nothing the broker sent is lost.
///
/// The enum is exhaustive: a header is absent, readable, or not.
///
/// ```
/// use ngsi_ld::http::headers::OptionalHeader;
///
/// let location: OptionalHeader<u64> = OptionalHeader::Valid(42);
/// assert_eq!(location.valid(), Some(&42));
/// assert!(OptionalHeader::<u64>::Absent.is_absent());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::IsVariant)]
pub enum OptionalHeader<T> {
    /// The response does not carry the header.
    Absent,
    /// The header, read.
    Valid(T),
    /// The header is present but cannot be read; its field lines, as received.
    Malformed(Vec<HeaderValue>),
}

impl<T> OptionalHeader<T> {
    /// The value, when the header was read.
    #[must_use]
    pub const fn valid(&self) -> Option<&T> {
        match self {
            Self::Valid(value) => Some(value),
            Self::Absent | Self::Malformed(_) => None,
        }
    }

    /// Takes the value, when the header was read.
    #[must_use]
    pub fn into_valid(self) -> Option<T> {
        match self {
            Self::Valid(value) => Some(value),
            Self::Absent | Self::Malformed(_) => None,
        }
    }

    /// Converts the value, keeping absence and malformation as they are.
    pub(crate) fn map<U>(self, convert: impl FnOnce(T) -> U) -> OptionalHeader<U> {
        match self {
            Self::Absent => OptionalHeader::Absent,
            Self::Valid(value) => OptionalHeader::Valid(convert(value)),
            Self::Malformed(lines) => OptionalHeader::Malformed(lines),
        }
    }
}

impl<H: Header> OptionalHeader<H> {
    /// Reads the header `H` from `headers`.
    pub(crate) fn read(headers: &HeaderMap) -> OptionalHeader<H> {
        let lines = headers.get_all(H::name());
        if lines.iter().next().is_none() {
            return OptionalHeader::Absent;
        }
        // `headers::Error` carries no detail, so the raw lines are what is kept of a field that does not decode.
        H::decode(&mut lines.iter()).map_or_else(|_| OptionalHeader::Malformed(lines.iter().cloned().collect()), OptionalHeader::Valid)
    }
}

#[cfg(test)]
mod tests {
    use crate::http::headers::{Location, OptionalHeader};
    use http::{HeaderMap, HeaderValue, header::LOCATION};
    use iri_rs::IriRefBuf;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_missing_header_is_absent() {
        assert_eq!(OptionalHeader::<Location>::read(&HeaderMap::new()), OptionalHeader::Absent);
    }

    #[test]
    fn a_readable_header_is_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(LOCATION, HeaderValue::from_static("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"));
        let read = OptionalHeader::<Location>::read(&headers).map(Location::into_iri_ref);
        assert_eq!(read.valid().map(IriRefBuf::as_str), Some("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"));
    }

    #[test]
    fn an_unreadable_header_keeps_every_raw_line() {
        let mut headers = HeaderMap::new();
        headers.append(LOCATION, HeaderValue::from_static("/entities/a b"));
        headers.append(LOCATION, HeaderValue::from_static("/entities/c"));
        assert_eq!(
            OptionalHeader::<Location>::read(&headers),
            OptionalHeader::Malformed(vec![HeaderValue::from_static("/entities/a b"), HeaderValue::from_static("/entities/c")])
        );
    }

    #[test]
    fn mapping_keeps_absence_and_malformation() {
        assert_eq!(OptionalHeader::<u8>::Absent.map(u16::from), OptionalHeader::Absent);
        assert_eq!(
            OptionalHeader::<u8>::Malformed(Vec::new()).map(u16::from),
            OptionalHeader::Malformed(Vec::new())
        );
        assert_eq!(OptionalHeader::Valid(7_u8).into_valid(), Some(7));
    }
}
