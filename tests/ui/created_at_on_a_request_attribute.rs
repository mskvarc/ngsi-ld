// Clause 5.2.2 makes `createdAt` read-only, so a request attribute has nowhere to put it.
use ngsi_ld::model::{DateTime, Property};

fn main() {
    let created: DateTime = "2020-01-01T00:00:00Z".parse().unwrap();
    let _ = Property::builder(1).created_at(created);
}
