// `InstanceKey` is sealed, since an attribute and a change to one are the only instances clause 4.5.5 has.
use ngsi_ld::model::{DatasetId, InstanceKey};

struct Downstream;

impl InstanceKey for Downstream {
    fn dataset_id(&self) -> Option<&DatasetId> {
        None
    }
}

fn main() {}
