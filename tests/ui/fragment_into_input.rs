// A fragment may hold the NGSI-LD Null, which a request entity may not (clause 5.5.4); there is no
// conversion from one to the other.
use ngsi_ld::model::{Entity, EntityFragment, Input};

fn main() {
    let fragment = EntityFragment::builder().delete("speed").build().unwrap();
    let _: Entity<Input> = fragment.into();
}
