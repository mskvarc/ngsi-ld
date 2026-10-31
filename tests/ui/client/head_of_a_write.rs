// A write operation has no HEAD request: HEAD functions as GET (ETSI TS 104 176 clause 5).

use ngsi_ld::{
    http::operation::{DeleteEntity, Head},
    model::EntityId,
};

fn main() {
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let delete = DeleteEntity::builder().id(&id).build();
    let _ = Head::new(&delete);
}
