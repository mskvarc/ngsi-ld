// Each element of Batch Entity Merge represents an entity "as mandated by clause 5.2.4" (clause 5.6.20.3),
// so a fragment whose `id` and `type` are optional is not accepted.
use ngsi_ld::{http::operation::BatchMerge, model::EntityFragment};

fn main() {
    let fragments = [EntityFragment::builder().delete("speed").build().unwrap()];
    let _ = BatchMerge::builder().fragments(&fragments).build();
}
