// An entity needs a `type` (clause 5.2.4); the builder cannot finish without one.
use iri_rs::iri;
use ngsi_ld::model::Entity;

fn main() {
    let _ = Entity::builder().id(iri!("urn:ngsi-ld:Vehicle:A4567")).build();
}
