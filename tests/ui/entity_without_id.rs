// An entity needs an `id` (clause 5.2.4); the builder cannot finish without one.
use ngsi_ld::model::Entity;

fn main() {
    let _ = Entity::builder().entity_type("Vehicle").build();
}
