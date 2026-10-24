// Clause 4.5.3.2 forbids `unitCode` on a Relationship.
use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::model::{EntityId, Relationship};

fn main() {
    let _ = Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Person:Bob"))).unit_code(UnitCode::Cel);
}
