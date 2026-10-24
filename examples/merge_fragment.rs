//! Changes an entity with Merge Entity and an entity fragment that sets `speed` and deletes `isParked` with the
//! NGSI-LD Null; every other attribute is left as it was.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example merge_fragment`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{AttributeKind, Entity, EntityFragment, Lossless, Representation},
};
use std::{env, slice};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder()
        .base_url(base_url)
        .transport(ReqwestTransport::with_default_client()?)
        .build()?;

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:M3001"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", 0)
        .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
        .build()?;
    client.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;

    client.entities().create(&vehicle).send().await?;
    println!("created:\n{}", vehicle.render(Representation::Normalized));

    let fragment = EntityFragment::builder()
        .property("speed", 50)
        .delete_kind("isParked", AttributeKind::Relationship)
        .build()?;
    println!("merging:\n{}", fragment.render(Lossless::Normalized));
    client.entities().merge(&vehicle.id, &fragment).send().await?;
    let merged = client.entities().retrieve(&vehicle.id).send().await?.into_body();
    println!("after the merge:\n{}", merged.render(Representation::Normalized));

    client.entities().delete(&vehicle.id).send().await?;
    println!("deleted {}", vehicle.id);
    Ok(())
}
