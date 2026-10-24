//! Delete Snapshot (NGSI-LD clauses 5.16.5 and 6.37.3.3).

use crate::{http::path::ResourcePath, snapshot::SnapshotId};

/// Deletes a snapshot (NGSI-LD clause 5.16.5): `DELETE /snapshots/{snapshotId}` (clause 6.37.3.3), answered 204.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteSnapshot, Operation},
///     },
///     snapshot::SnapshotId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = SnapshotId::from(iri!("urn:ngsi-ld:Snapshot:1"));
/// let request = DeleteSnapshot { id: &id }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteSnapshot<'a> {
    /// The snapshot.
    pub id: &'a SnapshotId,
}

delete_resource!(DeleteSnapshot => ResourcePath::snapshot);
