//! Changes the attributes of an entity with Append Attributes (with and without `noOverwrite`), Update Attributes,
//! Partial Attribute Update and Replace Attribute, reading the entity back after each.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features reqwest --example append_update_attributes`.

use anyhow::Context;
use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{UpdateOutcome, operation::Overwrite},
    model::{Attribute, AttributeName, DateTime, Entity, EntityFragment, EntityId, Input, Property, Representation},
};
use std::{env, slice};
use url::Url;

/// Prints what a step did and the entity it left.
async fn print_step(client: &Client<ReqwestTransport>, step: &str, outcome: UpdateOutcome, id: &EntityId) -> anyhow::Result<()> {
    match outcome {
        UpdateOutcome::Done => println!("{step}: done"),
        UpdateOutcome::Partial(result) => println!("{step}: not updated {:?}", result.not_updated),
    }
    let stored = client.entities().retrieve(id).send().await?.into_body();
    println!("{}", stored.render(Representation::Concise));
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder()
        .base_url(base_url)
        .transport(ReqwestTransport::new(reqwest::Client::new()))
        .build()?;
    let entities = client.entities();

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A8001"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", Property::builder(80).unit_code(UnitCode::Kmh))
        .build()?;
    let id = &vehicle.id;
    entities.batch_delete(slice::from_ref(id)).send().await?;
    entities.create(&vehicle).send().await?;
    println!("created:\n{}", vehicle.render(Representation::Concise));

    let appended = EntityFragment::builder()
        .property("color", "white")
        .property("speed", Property::builder(70).unit_code(UnitCode::Kmh))
        .build()?;
    let outcome = entities.append_attributes(id, &appended).send().await?;
    print_step(&client, "append color, overwrite speed", outcome, id).await?;

    // With noOverwrite, the existing speed is kept and listed as not updated.
    let kept = EntityFragment::builder().property("mileage", 12000).property("speed", 0).build()?;
    let outcome = entities.append_attributes(id, &kept).overwrite(Overwrite::Keep).send().await?;
    print_step(&client, "append mileage, keep speed", outcome, id).await?;

    let updated = EntityFragment::builder().property("brandName", "Mercedes-Benz").build()?;
    let outcome = entities.update_attributes(id, &updated).send().await?;
    print_step(&client, "update brandName", outcome, id).await?;

    // The members the patch does not name, here the unitCode, are left as they are.
    let speed: AttributeName = "speed".parse()?;
    let observed_at: DateTime = "2024-05-01T08:30:00Z".parse()?;
    let patch = Attribute::from(Property::builder(90).observed_at(observed_at).build()?).into_patch();
    let outcome = entities.update_attribute(id, &speed, patch).send().await?;
    print_step(&client, "partially update speed", outcome, id).await?;

    let color: AttributeName = "color".parse()?;
    let replacement: Attribute<Input> = Property::builder("silver").property("finish", "metallic").build()?.into();
    let outcome = entities.replace_attribute(id, &color, &replacement).send().await?;
    print_step(&client, "replace color", outcome, id).await?;

    entities.delete(id).send().await?;
    println!("deleted {id}");
    Ok(())
}
