// Clause 4.5.24.2 forbids `unitCode` on a JsonProperty.
use cefact_units::UnitCode;
use ngsi_ld::model::{JsonProperty, RawJson};

fn main() {
    let json: RawJson = "{}".parse().unwrap();
    let _ = JsonProperty::builder(json).unit_code(UnitCode::Cel);
}
