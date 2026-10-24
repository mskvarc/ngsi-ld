//! Queries entities by type with a `q` filter and a `pick` projection, two per page with the total count, and walks
//! the pages through the `next` link the broker returns.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example query_pagination`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::ResultsCount,
    model::{Entity, EntityId, Representation},
    query::{
        projection::{self, EntityMember},
        q,
    },
};
use std::env;
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

    let fleet = [
        (iri!("urn:ngsi-ld:Vehicle:Q2001"), "Mercedes", 20),
        (iri!("urn:ngsi-ld:Vehicle:Q2002"), "Volvo", 40),
        (iri!("urn:ngsi-ld:Vehicle:Q2003"), "Fiat", 60),
        (iri!("urn:ngsi-ld:Vehicle:Q2004"), "Renault", 80),
        (iri!("urn:ngsi-ld:Vehicle:Q2005"), "Skoda", 100),
        (iri!("urn:ngsi-ld:Vehicle:Q2006"), "Seat", 120),
    ];
    let ids = fleet.map(|(id, _, _)| EntityId::from(id));
    client.entities().batch_delete(&ids).send().await?;

    for (id, brand, speed) in fleet {
        let vehicle = Entity::builder()
            .id(id)
            .entity_type("Vehicle")
            .property("brandName", brand)
            .property("speed", speed)
            .property("mileage", 1000 * speed)
            .build()?;
        client.entities().create(&vehicle).send().await?;
        println!("created {}", vehicle.id);
    }

    let first = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .q(q::attr("speed").any_of([40, 80, 120]))
        .pick(
            projection::member(EntityMember::Id)
                .member(EntityMember::Type)
                .attribute("brandName")
                .attribute("speed"),
        )
        .limit(2)
        .count(ResultsCount::Include)
        .send()
        .await?;
    if let Some(total) = first.total() {
        println!("{total} vehicles match");
    }
    let mut pages = client.pages(first);
    let mut number = 1;
    while let Some(page) = pages.next().await {
        println!("page {number}:");
        for vehicle in page?.items() {
            println!("  {}", vehicle.render(Representation::Concise));
        }
        number += 1;
    }
    println!("no further page");

    client.entities().batch_delete(&ids).send().await?;
    println!("deleted {} vehicles", ids.len());
    Ok(())
}
