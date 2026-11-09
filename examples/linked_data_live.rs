//! Stores two `@context`s on a broker that name the same things differently, writes a vehicle under the fleet's and
//! reads and queries it under the garage's, then translates the fleet's query with the contexts the broker serves.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features linked-data --example linked_data_live`.

use anyhow::Context as _;
use iri_rs::{IriRefBuf, iri};
use ngsi_ld::{
    client::{Client, ContextSource, ReqwestTransport},
    linked_data::ContextProcessor,
    model::{Context, ContextEntry, ContextId, Entity, Representation},
    query::q::{self, Query},
};
use std::{env, slice};
use url::Url;

/// The fleet's names: `Vehicle`, `speed` and `plate`.
const FLEET: &str = r#"{
    "Vehicle": "https://example.org/fleet/Vehicle",
    "speed": "https://example.org/fleet/speed",
    "plate": "https://example.org/fleet/licensePlate"
}"#;

/// The garage's names for the same things: `Car`, `velocity`, and the prefix `fleet:` for the license plate.
const GARAGE: &str = r#"{
    "Car": "https://example.org/fleet/Vehicle",
    "velocity": "https://example.org/fleet/speed",
    "fleet": "https://example.org/fleet/"
}"#;

/// Stores `definition` on the broker, and returns its id and the URL the broker serves it at.
async fn store(client: &Client<ReqwestTransport>, definition: &str) -> anyhow::Result<(ContextId, IriRefBuf)> {
    let context = Context::Single(ContextEntry::Inline(definition.parse()?));
    let added = client.contexts().add(&context).send().await?;
    let id = added.id().context("the broker names no stored @context")?;
    let location = added.location.valid().context("the broker sends no Location")?;
    let url = client.endpoint().base_url().join(location.as_str())?;
    Ok((id, IriRefBuf::new(url.to_string())?))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let (fleet_id, fleet_url) = store(&client, FLEET).await?;
    let (garage_id, garage_url) = store(&client, GARAGE).await?;
    println!("the fleet's @context is served at {fleet_url}");
    println!("the garage's @context is served at {garage_url}");
    let fleet = client.with_context(fleet_url.clone())?;
    let garage = client.with_context(garage_url.clone())?;

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:L7001"))
        .entity_type("Vehicle")
        .property("speed", 55)
        .property("plate", "N-1234")
        .property("brandName", "Mercedes")
        .build()?;
    fleet.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;
    fleet.entities().create(&vehicle).send().await?;
    println!("the fleet writes {}", vehicle.render(Representation::Concise));

    let read = garage.entities().retrieve(&vehicle.id).send().await?.into_body();
    println!("the garage reads {}", read.render(Representation::Concise));
    let page = garage.entities().query().entity_type("Car").q(q::attr("velocity").gt(50)).send().await?;
    println!("the garage's query for cars with velocity>50 finds {} vehicle(s)", page.items().len());

    // The loader reads each context where the broker reads it, so both sides agree on what the names mean.
    let loader = client.context_loader().source(ContextSource::Broker).build();
    let processor = ContextProcessor::try_new()?;
    let fleet_names = processor.process(&Context::from(fleet_url), &loader).await?;
    let garage_names = processor.process(&Context::from(garage_url), &loader).await?;
    let fleet_query: Query = "speed>50;plate==\"N-1234\"".parse()?;
    for term in [q::attr("speed").gt(50).build()?, fleet_query] {
        match garage_names.recontextualize(term.clone(), &fleet_names) {
            Ok(translated) => println!("the fleet's {term} is {translated} for the garage"),
            Err(error) => println!("the fleet's {term} cannot be written for the garage: {error}"),
        }
    }

    fleet.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;
    client.contexts().delete(&fleet_id).send().await?;
    client.contexts().delete(&garage_id).send().await?;
    Ok(())
}
