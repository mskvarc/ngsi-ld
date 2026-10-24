// The Snapshot API cannot be aimed at a snapshot (Table 4.3.5-1, clause 5.5.15).

use ngsi_ld::{
    http::operation::{DeleteSnapshot, OnSnapshot},
    snapshot::SnapshotId,
};

fn main() {
    let snapshot: SnapshotId = "urn:ngsi-ld:Snapshot:1".parse().unwrap();
    let delete = DeleteSnapshot { id: &snapshot };
    let _ = OnSnapshot::new(&snapshot, &delete);
}
