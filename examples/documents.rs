//! Builds, writes and reads the documents of the data model without any feature: an entity in each representation,
//! a received entity turned into one a client sends, invalid documents refused, and two fragments.
//!
//! Run it with `cargo run --example documents`.

use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::model::{
    Attribute,
    AttributeFragment,
    Change,
    DateTime,
    Entity,
    EntityFragment,
    EntityId,
    InstanceDeletion,
    Lossless,
    Output,
    PatchMember,
    Projected,
    Property,
    Representation,
};

/// A received vehicle in concise representation, with the system members a broker adds.
const RECEIVED: &str = r#"{
    "id": "urn:ngsi-ld:Vehicle:B9211",
    "type": "Vehicle",
    "speed": {"value": 80, "unitCode": "KMH", "createdAt": "2024-05-01T08:00:00Z", "modifiedAt": "2024-05-01T09:00:00Z"},
    "createdAt": "2024-05-01T08:00:00Z",
    "modifiedAt": "2024-05-01T09:00:00Z"
}"#;

/// A vehicle in normalized representation whose `speed` has no `type`, which that representation requires.
const UNTYPED: &str = r#"{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": {"value": 80}}"#;

fn main() -> anyhow::Result<()> {
    let observed_at: DateTime = "2024-05-01T08:30:00Z".parse()?;
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", Property::builder(80).unit_code(UnitCode::Kmh).observed_at(observed_at))
        .relationship("isParked", EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")))
        .language_property("street", [("fr", "Grand Place"), ("nl", "Grote Markt")])
        .build()?;
    println!("normalized:\n{}", vehicle.render(Representation::Normalized));
    println!("concise:\n{}", vehicle.render(Representation::Concise));
    println!("simplified:\n{}", vehicle.render(Representation::Simplified));

    let received = Entity::<Output>::parse(RECEIVED, Lossless::Concise)?;
    if let Some(modified_at) = &received.output.modified_at {
        println!("received {}, last modified at {modified_at}", received.id);
    }
    let sendable = received.into_input()?;
    println!("without its system members:\n{}", sendable.render(Representation::Normalized));

    // The builder reports every problem at once: an invalid id and a location that is not a GeoProperty.
    let invalid = Entity::builder()
        .id("urn:ngsi-ld:Vehicle:A 4567")
        .entity_type("Vehicle")
        .property("location", "Grand Place")
        .build();
    match invalid {
        Ok(vehicle) => println!("the builder accepted {}", vehicle.id),
        Err(error) => println!("the builder refused the vehicle: {error}"),
    }
    match Entity::<Output>::parse(UNTYPED, Lossless::Normalized) {
        Ok(vehicle) => println!("the reader accepted {}", vehicle.id),
        Err(error) => println!("the reader refused the vehicle: {error}"),
    }

    let fragment = EntityFragment::builder().property("speed", 0).delete("isParked").build()?;
    let text = fragment.render(Lossless::Normalized).to_string();
    println!("a fragment setting speed and deleting isParked:\n{text}");
    let parsed = EntityFragment::<Projected>::parse(&text, Lossless::Normalized)?;
    println!("read back, concise:\n{}", parsed.render(Lossless::Concise));

    let speed = Attribute::from(Property::builder(90).observed_at(observed_at).build()?)
        .into_patch()
        .delete_member(PatchMember::UnitCode)?;
    let text = speed.render(Lossless::Normalized).to_string();
    println!("a fragment of speed deleting its unitCode:\n{text}");
    match Change::<AttributeFragment, InstanceDeletion>::parse(&text, Lossless::Normalized)? {
        Change::Set(fragment) => println!("read back, concise:\n{}", fragment.render(Lossless::Concise)),
        Change::Delete(deletion) => println!("read back as a deletion: {deletion:?}"),
    }
    Ok(())
}
