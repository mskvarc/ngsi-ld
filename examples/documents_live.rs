//! Reads a vehicle back with its system members, changes the received document and writes it back whole, then
//! changes it in place with a fragment and an attribute patch, showing the broker's `modifiedAt` move each time.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example documents_live`.

use anyhow::Context;
use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::SystemAttributes,
    model::{Attribute, AttributeKind, AttributeName, DateTime, Entity, EntityFragment, EntityId, Output, PatchMember, Property, Representation},
};
use std::{env, slice};
use url::Url;

/// Reads the vehicle back with its system members and prints when it was created and last modified.
async fn show(client: &Client<ReqwestTransport>, id: &EntityId, step: &str) -> anyhow::Result<Entity<Output>> {
    let stored = client
        .entities()
        .retrieve(id)
        .system_attributes(SystemAttributes::Include)
        .send()
        .await?
        .into_body();
    let created = stored.output.created_at.as_ref().context("the broker sent no createdAt")?;
    let modified = stored.output.modified_at.as_ref().context("the broker sent no modifiedAt")?;
    println!("{step}: created at {created}, modified at {modified}");
    println!("  {}", stored.clone().into_input()?.render(Representation::Concise));
    Ok(stored)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let observed_at: DateTime = "2024-05-01T08:30:00Z".parse()?;
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:D2001"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", Property::builder(80).unit_code(UnitCode::Kmh).observed_at(observed_at))
        .relationship("isParked", EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")))
        .language_property("street", [("fr", "Grand Place"), ("nl", "Grote Markt")])
        .build()?;
    let id = &vehicle.id;
    client.entities().batch_delete(slice::from_ref(id)).send().await?;
    client.entities().create(&vehicle).send().await?;
    let received = show(&client, id, "created").await?;

    // The received document drops its system members on the way back, and gains a colour.
    let mut repainted = received.into_input()?;
    repainted
        .attributes
        .insert("color".parse()?, Attribute::from(Property::builder("white").build()?))?;
    client.entities().replace(&repainted).send().await?;
    show(&client, id, "replaced with a colour").await?;

    let parked = EntityFragment::builder()
        .property("speed", Property::builder(0).unit_code(UnitCode::Kmh))
        .delete_kind("isParked", AttributeKind::Relationship)
        .build()?;
    client.entities().merge(id, &parked).send().await?;
    show(&client, id, "merged a stop, isParked deleted").await?;

    let speed: AttributeName = "speed".parse()?;
    let without_unit = Attribute::from(Property::builder(0).build()?)
        .into_patch()
        .delete_member(PatchMember::UnitCode)?;
    client.entities().update_attribute(id, &speed, without_unit).send().await?;
    show(&client, id, "patched speed, unitCode deleted").await?;

    client.entities().batch_delete(slice::from_ref(id)).send().await?;
    Ok(())
}
