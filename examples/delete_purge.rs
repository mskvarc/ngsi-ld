//! Removes entities with Delete Entity, with Purge Entities and `drop`, and with Purge Entities alone, querying what
//! is left after each step. The purges name the example's own ids, so they touch nothing else on the broker.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example delete_purge`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{AttributeName, Entity, EntityId, Representation},
};
use std::env;
use url::Url;

/// Prints the entities among `ids` that are left.
async fn print_remaining(client: &Client<ReqwestTransport>, ids: &[EntityId]) -> anyhow::Result<()> {
    let page = client.entities().query().entity_type("Vehicle").ids(ids.to_vec()).send().await?;
    println!("{} entities left:", page.items().len());
    for entity in page.items() {
        println!("  {}", entity.render(Representation::Concise));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let iris = [
        iri!("urn:ngsi-ld:Vehicle:P6001"),
        iri!("urn:ngsi-ld:Vehicle:P6002"),
        iri!("urn:ngsi-ld:Vehicle:P6003"),
    ];
    let ids = iris.map(EntityId::from);
    let [first, rest @ ..] = &ids;
    client.entities().batch_delete(&ids).send().await?;

    for (iri, color) in iris.into_iter().zip(["red", "green", "blue"]) {
        let vehicle = Entity::builder()
            .id(iri)
            .entity_type("Vehicle")
            .property("color", color)
            .property("speed", 30)
            .build()?;
        client.entities().create(&vehicle).send().await?;
        println!("created {}", vehicle.id);
    }

    client.entities().delete(first).send().await?;
    println!("deleted {first}");
    print_remaining(&client, &ids).await?;

    let color: AttributeName = "color".parse()?;
    client
        .entities()
        .purge()
        .entity_type("Vehicle")
        .ids(rest.to_vec())
        .drop(vec![color])
        .send()
        .await?;
    println!("purged color from the remaining vehicles");
    print_remaining(&client, &ids).await?;

    client.entities().purge().entity_type("Vehicle").ids(rest.to_vec()).send().await?;
    println!("purged the remaining vehicles");
    print_remaining(&client, &ids).await?;

    client.entities().batch_delete(&ids).send().await?;
    Ok(())
}
