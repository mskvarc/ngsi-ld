//! Replaces every attribute of an entity with Replace Entity and reads the result back.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example replace_entity`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{Entity, Representation},
};
use std::{env, slice};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let original = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:R2001"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", 80)
        .build()?;
    client.entities().batch_delete(slice::from_ref(&original.id)).send().await?;

    client.entities().create(&original).send().await?;
    println!("created:\n{}", original.render(Representation::Normalized));

    let replacement = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:R2001"))
        .entity_type("Vehicle")
        .property("brandName", "Volvo")
        .property("color", "red")
        .build()?;
    println!("replacing it with:\n{}", replacement.render(Representation::Normalized));
    client.entities().replace(&replacement).send().await?;
    let stored = client.entities().retrieve(&original.id).send().await?.into_body();
    println!("after the replacement:\n{}", stored.render(Representation::Normalized));

    client.entities().delete(&original.id).send().await?;
    println!("deleted {}", original.id);
    Ok(())
}
