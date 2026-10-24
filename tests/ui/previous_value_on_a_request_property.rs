// Clause 4.5.2.2 makes `previousValue` output-only, so a request Property has nowhere to put it.
use ngsi_ld::model::Property;

fn main() {
    let _ = Property::builder(1).previous_value(5);
}
