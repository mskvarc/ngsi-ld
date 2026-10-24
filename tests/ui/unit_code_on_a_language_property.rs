// Clause 4.5.18.2 forbids `unitCode` on a LanguageProperty, so the builder has no such setter.
use cefact_units::UnitCode;
use ngsi_ld::model::{LanguageMap, LanguageProperty};

fn main() {
    let map = LanguageMap::try_from_pairs([("en", "x")]).unwrap();
    let _ = LanguageProperty::builder(map).unit_code(UnitCode::Cel);
}
