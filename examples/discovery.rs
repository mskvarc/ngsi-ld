//! Finds out what a broker holds, listing the entity types and describing a type and an attribute, then stores a
//! context there, lists the hosted contexts, serves it back and deletes it.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example discovery`.

use anyhow::Context as _;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::ContextKindFilter,
    model::{AttributeName, Context, ContextEntry, Entity, EntityId, EntityTypeName, InlineContext},
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
        (iri!("urn:ngsi-ld:Vehicle:V8001"), 80, "Mercedes"),
        (iri!("urn:ngsi-ld:Vehicle:V8002"), 40, "Volvo"),
    ];
    let ids = fleet.map(|(id, _, _)| EntityId::from(id));
    client.entities().batch_delete(&ids).send().await?;
    for (id, speed, brand) in fleet {
        let vehicle = Entity::builder()
            .id(id)
            .entity_type("Vehicle")
            .property("speed", speed)
            .property("brandName", brand)
            .build()?;
        client.entities().create(&vehicle).send().await?;
        println!("created {}", vehicle.id);
    }

    let types = client.entity_types().list().send().await?;
    println!("the broker holds {} entity types", types.type_list.len());
    let vehicle_type: EntityTypeName = "Vehicle".parse()?;
    let info = client.entity_types().retrieve(&vehicle_type).send().await?;
    for attribute in &info.attribute_details {
        println!("{vehicle_type}: {} {:?}", attribute.attribute_name, attribute.attribute_types);
    }
    println!("{vehicle_type}: {} entities", info.entity_count);
    let speed: AttributeName = "speed".parse()?;
    let speeds = client.attributes().retrieve(&speed).send().await?;
    println!("speed: {:?} instances, in {:?}", speeds.attribute_count, speeds.type_names);

    let inline: InlineContext = r#"{"vehicleSpeed": "https://example.org/vocabulary/speed"}"#.parse()?;
    let context = Context::Single(ContextEntry::Inline(inline));
    let created = client.contexts().add(&context).send().await?;
    let id = created.id().context("the broker named no context in the Location of its answer")?;
    println!("stored the context {id}");
    let hosted = client.contexts().list().kind(ContextKindFilter::Hosted).send().await?;
    println!("the broker hosts {} contexts", hosted.len());
    let served = client.contexts().serve(&id).send().await?;
    println!("served back: {served:?}");
    client.contexts().delete(&id).send().await?;
    println!("deleted the context {id}");

    client.entities().batch_delete(&ids).send().await?;
    println!("deleted {} vehicles", ids.len());
    Ok(())
}
