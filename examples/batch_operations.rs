//! Works on a fleet with the batch operations: creates three vehicles in one request, merges a change into two of
//! them, upserts a fourth alongside a replacement of the first, and deletes the whole fleet.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example batch_operations`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{BatchCreateOutcome, BatchOperationResult, BatchOutcome, UpsertOutcome},
    model::{AttributeKind, Entity, EntityFragment, EntityId},
};
use std::env;
use url::Url;

/// Prints what a 207 says about each entity.
fn report(operation: &str, result: &BatchOperationResult) {
    for id in &result.success {
        println!("{operation}: {id} done");
    }
    for error in &result.errors {
        println!("{operation}: {} failed: {:?}", error.entity_id, error.error.title);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder()
        .base_url(base_url)
        .transport(ReqwestTransport::with_default_client()?)
        .build()?;

    let iris = [
        iri!("urn:ngsi-ld:Vehicle:F1001"),
        iri!("urn:ngsi-ld:Vehicle:F1002"),
        iri!("urn:ngsi-ld:Vehicle:F1003"),
        iri!("urn:ngsi-ld:Vehicle:F1004"),
    ];
    let ids = iris.map(EntityId::from);
    let [first, second, third, fourth] = iris;
    client.entities().batch_delete(&ids).send().await?;

    let fleet = [first, second, third]
        .into_iter()
        .map(|id| {
            Entity::builder()
                .id(id)
                .entity_type("Vehicle")
                .property("speed", 0)
                .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
                .build()
        })
        .collect::<Result<Vec<_>, _>>()?;
    match client.entities().batch_create(&fleet).send().await? {
        BatchCreateOutcome::Created(created) => println!("created {created:?}"),
        BatchCreateOutcome::Partial(result) => report("create", &result),
    }

    // A departure sets a speed and deletes isParked.
    let departures = [(first, 40), (second, 60)]
        .into_iter()
        .map(|(id, speed)| {
            EntityFragment::for_entity(id, "Vehicle")
                .property("speed", speed)
                .delete_kind("isParked", AttributeKind::Relationship)
                .build()
        })
        .collect::<Result<Vec<_>, _>>()?;
    match client.entities().batch_merge(&departures).send().await? {
        BatchOutcome::Done => println!("merged {} departures", departures.len()),
        BatchOutcome::Partial(result) => report("merge", &result),
    }

    // The broker replaces the first vehicle, which exists, and creates the fourth, which does not.
    let upserted = [
        Entity::builder().id(first).entity_type("Vehicle").property("speed", 0).build()?,
        Entity::builder()
            .id(fourth)
            .entity_type("Vehicle")
            .property("speed", 0)
            .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
            .build()?,
    ];
    match client.entities().batch_upsert(&upserted).send().await? {
        UpsertOutcome::Created(created) => println!("upsert created {created:?}, updated the others"),
        UpsertOutcome::Updated => println!("upsert updated every entity"),
        UpsertOutcome::Partial(result) => report("upsert", &result),
    }

    match client.entities().batch_delete(&ids).send().await? {
        BatchOutcome::Done => println!("deleted {} vehicles", ids.len()),
        BatchOutcome::Partial(result) => report("delete", &result),
    }
    Ok(())
}
