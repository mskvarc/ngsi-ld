//! Freezes vehicles in a snapshot, waits for it to settle, changes a vehicle and pages through the vehicles of the
//! snapshot, which still hold the speed they had.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features reqwest --example snapshots`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{Entity, EntityFragment, EntityId},
    snapshot::{QueryDocument, Snapshot, SnapshotStatus},
};
use std::{env, time::Duration};
use tokio::time::sleep;
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder()
        .base_url(base_url)
        .transport(ReqwestTransport::new(reqwest::Client::new()))
        .build()?;

    let fleet = [(iri!("urn:ngsi-ld:Vehicle:S1101"), 80), (iri!("urn:ngsi-ld:Vehicle:S1102"), 40)];
    let ids = fleet.map(|(id, _)| EntityId::from(id));
    let [changed, _] = &ids;
    client.entities().batch_delete(&ids).send().await?;
    for (id, speed) in fleet {
        let vehicle = Entity::builder().id(id).entity_type("Vehicle").property("speed", speed).build()?;
        client.entities().create(&vehicle).send().await?;
        println!("created {}", vehicle.id);
    }

    let vehicles = QueryDocument::builder().entities(["Vehicle"]).build()?;
    let snapshot = Snapshot::builder().query(vehicles).lifetime("PT1H".parse()?).build()?;
    let created = client.snapshots().create(&snapshot).send().await?;
    let id = created.id().context("the broker named no snapshot in its answer")?;
    println!("created the snapshot {}", id.as_str());

    // The snapshot runs its queries in the background; its status says when they are done.
    let mut status = client.snapshots().retrieve(&id).send().await?.status();
    for _ in 0..20 {
        if status != SnapshotStatus::Preparing {
            break;
        }
        sleep(Duration::from_millis(500)).await;
        status = client.snapshots().retrieve(&id).send().await?.status();
    }
    println!("the snapshot is {status}");

    let stopped = EntityFragment::builder().property("speed", 0).build()?;
    client.entities().merge(changed, &stopped).send().await?;
    println!("stopped {changed}");

    let first = client
        .on_snapshot(&id)
        .entities()
        .query_simplified()
        .entity_type("Vehicle")
        .limit(1)
        .send()
        .await?;
    for vehicle in client.pages(first).try_collect().await? {
        println!("on the snapshot: {}", vehicle.render());
    }

    client.snapshots().delete(&id).send().await?;
    println!("deleted the snapshot {}", id.as_str());
    client.entities().batch_delete(&ids).send().await?;
    println!("deleted {} vehicles", ids.len());
    Ok(())
}
