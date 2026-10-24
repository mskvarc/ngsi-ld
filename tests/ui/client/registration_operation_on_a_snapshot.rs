// The Registry API cannot be aimed at a snapshot (Table 4.3.5-1, clause 5.5.15).

use ngsi_ld::{
    http::operation::{DeleteRegistration, OnSnapshot},
    registration::RegistrationId,
    snapshot::SnapshotId,
};

fn main() {
    let snapshot: SnapshotId = "urn:ngsi-ld:Snapshot:1".parse().unwrap();
    let registration: RegistrationId = "urn:ngsi-ld:ContextSourceRegistration:r1".parse().unwrap();
    let delete = DeleteRegistration { id: &registration };
    let _ = OnSnapshot::new(&snapshot, &delete);
}
