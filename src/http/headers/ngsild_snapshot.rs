//! The `NGSILD-Snapshot` header (NGSI-LD clause 6.3.22).

use crate::{
    http::headers::{InvalidFieldValue, text_field::TextField},
    snapshot::SnapshotId,
};
use http::{HeaderName, HeaderValue};

/// The name of the header.
static NGSILD_SNAPSHOT: HeaderName = HeaderName::from_static("ngsild-snapshot");

/// The snapshot an operation of the Core or Temporal API is aimed at (NGSI-LD clauses 5.5.15 and 6.3.22): its
/// value is the snapshot's identifier, and the broker repeats it in the response.
///
/// A snapshot id is a URI, which a header carries as written; an IRI with characters outside ASCII cannot be
/// carried, so [`NgsiLdSnapshot::new`] refuses it.
///
/// ```
/// use headers::{Header, HeaderMapExt};
/// use http::HeaderMap;
/// use iri_rs::iri;
/// use ngsi_ld::{http::headers::NgsiLdSnapshot, snapshot::SnapshotId};
///
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(NgsiLdSnapshot::new(SnapshotId::from(iri!(
///     "urn:ngsi-ld:Snapshot:1"
/// )))?);
/// assert_eq!(headers[NgsiLdSnapshot::name()], "urn:ngsi-ld:Snapshot:1");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NgsiLdSnapshot(TextField<SnapshotId>);

impl NgsiLdSnapshot {
    /// The header naming `snapshot`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the identifier holds characters a header field cannot carry.
    pub fn new(snapshot: SnapshotId) -> Result<NgsiLdSnapshot, InvalidFieldValue> {
        TextField::new(snapshot).map(NgsiLdSnapshot)
    }

    /// Returns the snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &SnapshotId {
        self.0.value()
    }

    /// Returns the snapshot, consuming the header.
    #[must_use]
    pub fn into_snapshot(self) -> SnapshotId {
        self.0.into_value()
    }
}

impl headers::Header for NgsiLdSnapshot {
    fn name() -> &'static HeaderName {
        &NGSILD_SNAPSHOT
    }

    fn decode<'i, I>(values: &mut I) -> Result<NgsiLdSnapshot, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        TextField::decode(values).map(NgsiLdSnapshot)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::{http::headers::NgsiLdSnapshot, snapshot::SnapshotId};
    use headers::Header;
    use http::HeaderValue;
    use iri_rs::iri;
    use pretty_assertions::assert_eq;
    use std::iter;

    #[test]
    fn a_snapshot_encodes_as_written_and_decodes_back() {
        let header = NgsiLdSnapshot::new(SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"))).unwrap();
        let mut encoded = Vec::new();
        header.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("urn:ngsi-ld:Snapshot:1")]);
        assert_eq!(NgsiLdSnapshot::decode(&mut encoded.iter()).ok(), Some(header.clone()));
        assert_eq!(header.into_snapshot().as_str(), "urn:ngsi-ld:Snapshot:1");
    }

    #[test]
    fn an_iri_outside_ascii_cannot_be_carried() {
        assert!(NgsiLdSnapshot::new("urn:ngsi-ld:Snapshot:é".parse().unwrap()).is_err());
    }

    #[test]
    fn a_value_that_is_no_uri_or_written_twice_is_refused() {
        assert!(NgsiLdSnapshot::decode(&mut iter::once(&HeaderValue::from_static("not a uri"))).is_err());
        let lines = [HeaderValue::from_static("urn:a:1"), HeaderValue::from_static("urn:a:2")];
        assert!(NgsiLdSnapshot::decode(&mut lines.iter()).is_err());
    }
}
