// Clauses 4.5.3.2 and 4.5.23.2 make `previousObject` and `entity` output-only, so a request Relationship
// has no output members to read.
use iri_rs::iri;
use ngsi_ld::model::{EntityId, Relationship};

fn main() {
    let owner = Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Person:Bob"))).build().unwrap();
    let _ = owner.output();
}
