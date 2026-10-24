//! Fixes the entities a distributed query pages through with an entity map: creates the map, reads it back, queries
//! the vehicles on it, extends its expiry and deletes it, then asks the broker who it is.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example entity_maps`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::EntityMapRequest,
    model::{DateTime, Entity, EntityId},
    registration::EntitySource,
};
use std::env;
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let fleet = [(iri!("urn:ngsi-ld:Vehicle:E9001"), 80), (iri!("urn:ngsi-ld:Vehicle:E9002"), 40)];
    let ids = fleet.map(|(id, _)| EntityId::from(id));
    client.entities().batch_delete(&ids).send().await?;
    for (id, speed) in fleet {
        let vehicle = Entity::builder().id(id).entity_type("Vehicle").property("speed", speed).build()?;
        client.entities().create(&vehicle).send().await?;
        println!("created {}", vehicle.id);
    }

    let created = client.entity_maps().create().entity_type("Vehicle").lifetime("PT1H".parse()?).send().await?;
    let id = &created.body().id;
    println!("created the map {id}");
    let map = client.entity_maps().retrieve(id).send().await?;
    for (entity, sources) in &map.entity_map {
        let local = sources.contains(&EntitySource::Local);
        println!("{entity}: held locally {local}, by {} sources", sources.len());
    }

    let location = created.entity_map().valid().cloned().context("the broker named no entity map in its answer")?;
    let reuse = EntityMapRequest::reuse(location);
    let page = client.entities().query().entity_type("Vehicle").entity_map(&reuse).send().await?;
    println!("{} vehicles on the map", page.items().len());

    let later: DateTime = "2099-01-01T00:00:00Z".parse()?;
    client.entity_maps().update(id, &later).send().await?;
    println!("the map now expires at {}", client.entity_maps().retrieve(id).send().await?.expires_at);
    client.entity_maps().delete(id).send().await?;
    println!("deleted the map {id}");

    client.entities().batch_delete(&ids).send().await?;
    println!("deleted {} vehicles", ids.len());

    let identity = client.source_identity().send().await?;
    println!("the broker is {}, up for {} at {}", identity.alias, identity.uptime, identity.time_at);
    Ok(())
}
