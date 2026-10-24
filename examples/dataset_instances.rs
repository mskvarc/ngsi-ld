//! Keeps three instances of `speed`, told apart by `datasetId`, and removes one of them with Delete Attribute and
//! its `datasetId` parameter.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example dataset_instances`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::InstanceSelection,
    model::{AttributeName, Entity, Instances, Property, Representation, Uri},
};
use std::{env, slice};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:D5001"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(55).property("source", "Speedometer"))
        .property(
            "speed",
            Property::builder(54).dataset_id(iri!("urn:ngsi-ld:dataset:gps")).property("source", "GPS"),
        )
        .property(
            "speed",
            Property::builder(56).dataset_id(iri!("urn:ngsi-ld:dataset:radar")).property("source", "Radar"),
        )
        .build()?;
    client.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;

    client.entities().create(&vehicle).send().await?;
    println!("created:\n{}", vehicle.render(Representation::Normalized));

    let speed: AttributeName = "speed".parse()?;
    let stored = client.entities().retrieve(&vehicle.id).send().await?.into_body();
    let count = stored.attribute(&speed).map_or(0, Instances::len);
    println!("read back with {count} instances of speed:\n{}", stored.render(Representation::Normalized));

    let gps = Uri::from(iri!("urn:ngsi-ld:dataset:gps"));
    client
        .entities()
        .delete_attribute(&vehicle.id, &speed)
        .instances(InstanceSelection::Dataset(&gps))
        .send()
        .await?;
    println!("deleted the instance {gps}");
    let remaining = client.entities().retrieve(&vehicle.id).send().await?.into_body();
    let count = remaining.attribute(&speed).map_or(0, Instances::len);
    println!("{count} instances of speed remain:\n{}", remaining.render(Representation::Normalized));

    client.entities().delete(&vehicle.id).send().await?;
    println!("deleted {}", vehicle.id);
    Ok(())
}
