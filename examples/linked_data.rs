//! Moves a vehicle and a query between two `@context`s that name the same things differently: expands names and the
//! document, compacts it under the other context, and re-contextualises the vehicle and a query. Nothing is fetched.
//!
//! Run it with `cargo run --features linked-data --example linked_data`.

use anyhow::Context as _;
use ngsi_ld::{
    linked_data::{ActiveContext, ContextProcessor},
    model::{AttributeName, Entity, Input, Lossless, Representation},
    query::q::Query,
};

/// The vehicle as the fleet writes it, under the fleet's context.
const FLEET_VEHICLE: &str = r#"{
    "id": "urn:ngsi-ld:Vehicle:A4567",
    "type": "Vehicle",
    "speed": {"type": "Property", "value": 55},
    "plate": {"type": "Property", "value": "N-1234"},
    "brandName": {"type": "Property", "value": "Mercedes"},
    "@context": {
        "Vehicle": "https://example.org/fleet/Vehicle",
        "speed": "https://example.org/fleet/speed",
        "plate": "https://example.org/fleet/licensePlate"
    }
}"#;

/// The same vehicle as the garage writes it, under the garage's context: `fleet:` is a prefix, not a term, of the
/// license plate.
const GARAGE_VEHICLE: &str = r#"{
    "id": "urn:ngsi-ld:Vehicle:A4567",
    "type": "Car",
    "velocity": {"type": "Property", "value": 55},
    "fleet:licensePlate": {"type": "Property", "value": "N-1234"},
    "brandName": {"type": "Property", "value": "Mercedes"},
    "@context": {
        "Car": "https://example.org/fleet/Vehicle",
        "velocity": "https://example.org/fleet/speed",
        "fleet": "https://example.org/fleet/"
    }
}"#;

/// Reads a vehicle and processes the `@context` it carries.
async fn read(processor: &ContextProcessor, text: &str) -> anyhow::Result<(Entity<Input>, ActiveContext)> {
    let vehicle = Entity::<Input>::parse(text, Lossless::Normalized)?;
    let context = vehicle.context.as_ref().context("the vehicle carries no @context")?;
    let active = processor.process_offline(context).await?;
    Ok((vehicle, active))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let processor = ContextProcessor::try_new()?;
    let (fleet_vehicle, fleet) = read(&processor, FLEET_VEHICLE).await?;
    let (garage_vehicle, garage) = read(&processor, GARAGE_VEHICLE).await?;

    let vehicle_type = fleet_vehicle.entity_type.as_one().context("the vehicle has several types")?;
    println!("{vehicle_type} expands to {}", fleet.expand(vehicle_type)?);
    // brandName is in neither context, so it expands under the core context's default vocabulary.
    for name in ["speed", "plate", "brandName"] {
        let name: AttributeName = name.parse()?;
        println!("{name} expands to {}", fleet.expand(&name)?);
    }

    let expanded = fleet.expand_document(&fleet_vehicle).await?;
    println!("the fleet's vehicle as JSON-LD: {}", expanded.render());
    println!("the garage's vehicle as JSON-LD: {}", garage.expand_document(&garage_vehicle).await?.render());
    let compacted: Entity<Input> = garage.compact_document(&expanded).await?;
    println!("compacted under the garage's context: {}", compacted.render(Representation::Normalized));
    let recontextualized = garage.recontextualize(fleet_vehicle, &fleet)?;
    println!("re-contextualised for the garage: {}", recontextualized.render(Representation::Normalized));

    let fast: Query = "speed>50".parse()?;
    println!("the fleet's query speed>50 is {} for the garage", garage.recontextualize(fast, &fleet)?);
    // A query term must be a bare term, and the garage can write the plate only as a compact IRI.
    let plate: Query = r#"plate=="N-1234""#.parse()?;
    match garage.recontextualize(plate, &fleet) {
        Ok(translated) => println!("the fleet's query naming plate is {translated} for the garage"),
        Err(error) => println!("the fleet's query naming plate is refused for the garage: {error}"),
    }
    Ok(())
}
