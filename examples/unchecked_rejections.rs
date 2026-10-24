//! Sends two invalid Create Entity requests through the deliberately unchecked path, an entity without `type` and
//! a body that is not JSON, and prints the problem the broker answers each with.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features reqwest --example unchecked_rejections`.

use anyhow::Context;
use bytes::Bytes;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{UncheckedDocument, UncheckedRequest, operation::CreateEntity},
    model::{Entity, Lossless},
};
use std::{env, slice};
use url::Url;

/// Sends `request` and prints whether the broker accepted it, or the problem it rejected it with.
async fn send(client: &Client<ReqwestTransport>, request: UncheckedRequest) -> anyhow::Result<()> {
    match client.send_unchecked(request).await {
        Ok(response) => println!("  accepted with {}", response.status()),
        Err(error) => match error.problem() {
            Some(problem) => println!(
                "  rejected: {:?}\n  title: {:?}\n  detail: {:?}",
                problem.error_type, problem.title, problem.detail
            ),
            None => return Err(error.into()),
        },
    }
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

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:U7001"))
        .entity_type("Vehicle")
        .property("speed", 80)
        .build()?;
    client.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;
    // Each request starts from a valid Create Entity and is broken on purpose.
    let create = CreateEntity::builder().entity(&vehicle).build();

    let untyped = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized).remove_member("type");
    println!("creating an entity without a type: {untyped}");
    send(&client, client.unchecked(&create)?.document(&untyped)).await?;

    let not_json = Bytes::from_static(br#"{"id": "urn:ngsi-ld:Vehicle:"#);
    println!("creating an entity from a body that is not JSON: {}", String::from_utf8_lossy(&not_json));
    send(&client, client.unchecked(&create)?.body(not_json)).await?;

    // A broker that accepted the untyped entity has stored it.
    client.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;
    Ok(())
}
