// Clause 4.5.20.2 forbids `unitCode` on a VocabProperty.
use cefact_units::UnitCode;
use ngsi_ld::model::{VocabProperty, VocabTerm};

fn main() {
    let term: VocabTerm = "x".parse().unwrap();
    let _ = VocabProperty::builder(term).unit_code(UnitCode::Cel);
}
