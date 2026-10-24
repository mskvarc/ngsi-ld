// A request entity always has `id` and `type` (clause 5.2.4); only an entity a broker returns may lack them under a
// projection (clause 4.21), so a complete request entity does not convert into a projected one.
use ngsi_ld::model::{Entity, Input, Projected};

fn main() {
    let vehicle = Entity::builder().id(iri_rs::iri!("urn:ngsi-ld:Vehicle:A4567")).entity_type("Vehicle").build().unwrap();
    let _: Entity<Input, Projected> = vehicle.into();
}
