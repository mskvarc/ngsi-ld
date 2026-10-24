//! Reads one entity back as normalized, concise and simplified JSON, using the `format` parameter of Retrieve
//! Entity.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example representations`.

use anyhow::Context;
use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{DateTime, Entity, EntityId, Lossless, Property, Relationship, Representation},
};
use std::{env, slice};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let observed_at: DateTime = "2018-08-07T12:00:00Z".parse()?;
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
        .entity_type("Vehicle")
        .property("brandName", "Volvo")
        .property("speed", Property::builder(80).unit_code(UnitCode::Kmh).observed_at(observed_at))
        .relationship(
            "isParked",
            Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))).observed_at(observed_at),
        )
        .language_property("street", [("fr", "Grand Place"), ("nl", "Grote Markt")])
        .vocab_property("category", "commercial")
        .build()?;
    client.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;

    client.entities().create(&vehicle).send().await?;
    println!("created {}", vehicle.id);

    let normalized = client.entities().retrieve(&vehicle.id).send().await?.into_body();
    println!("normalized:\n{}", normalized.render(Representation::Normalized));
    let concise = client.entities().retrieve(&vehicle.id).form(Lossless::Concise).send().await?.into_body();
    println!("concise:\n{}", concise.render(Representation::Concise));
    let simplified = client.entities().retrieve_simplified(&vehicle.id).send().await?.into_body();
    println!("simplified:\n{}", simplified.render());

    client.entities().delete(&vehicle.id).send().await?;
    println!("deleted {}", vehicle.id);
    Ok(())
}
