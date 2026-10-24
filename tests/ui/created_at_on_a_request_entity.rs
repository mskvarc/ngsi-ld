// The output-only members of a request entity are `()`, which has no `created_at`.
use iri_rs::iri;
use ngsi_ld::model::Entity;

fn main() {
    let entity = Entity::builder().id(iri!("urn:ngsi-ld:T:1")).entity_type("T").build().unwrap();
    let _ = entity.output.created_at;
}
