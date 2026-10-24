//! Creates the vehicle of NGSI-LD Annex C.2.2, with every attribute kind it carries, reads it back and deletes it.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features reqwest --example create_entity`.

use anyhow::Context;
use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::CreateOutcome,
    model::{DateTime, Entity, EntityId, EntityTypeName, ListProperty, ListRelationship, Relationship, Representation, Value, VocabTerm},
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
        .transport(ReqwestTransport::new(reqwest::Client::new()))
        .build()?;

    let is_parked = Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")))
        .object_type("OffStreetParking".parse::<EntityTypeName>()?)
        .observed_at("2017-07-29T12:00:04Z".parse::<DateTime>()?)
        .relationship("providedBy", iri!("urn:ngsi-ld:Person:Bob"));
    let tyre_tread_depths = ListProperty::builder(vec![Value::from(300), Value::from(300), Value::from(120), Value::from(6)])
        .value_type("Integer".parse::<VocabTerm>()?)
        .unit_code(UnitCode::Mmt);
    let passengers = Relationship::builder(vec![
        EntityId::from(iri!("urn:ngsi-ld:Person:Alice")),
        EntityId::from(iri!("urn:ngsi-ld:Person:Bob")),
    ])
    .object_type("Person".parse::<EntityTypeName>()?);
    let route = ListRelationship::builder(vec![
        EntityId::from(iri!("urn:ngsi-ld:City:Antwerp")),
        EntityId::from(iri!("urn:ngsi-ld:City:Rotterdam")),
        EntityId::from(iri!("urn:ngsi-ld:City:Amsterdam")),
    ])
    .object_type("City".parse::<EntityTypeName>()?);
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .language_property("street", [("fr", "Grand Place"), ("nl", "Grote Markt")])
        .relationship("isParked", is_parked)
        .vocab_property("category", "non-commercial")
        .list_property("tyreTreadDepths", tyre_tread_depths)
        .relationship("passengers", passengers)
        .list_relationship("route", route)
        .build()?;

    // Removes what an earlier run left behind; an id the broker does not hold is reported, not an error.
    client.entities().batch_delete(slice::from_ref(&vehicle.id)).send().await?;

    println!("sending:\n{}", vehicle.render(Representation::Normalized));
    match client.entities().create(&vehicle).send().await? {
        CreateOutcome::Created { location } => println!("created {} at {location:?}", vehicle.id),
        CreateOutcome::PartiallyCreated { location, result } => {
            println!("created {} at {location:?}, but a distributed part failed: {result:?}", vehicle.id);
        }
    }

    let stored = client.entities().retrieve(&vehicle.id).send().await?.into_body();
    println!("read back:\n{}", stored.render(Representation::Normalized));

    client.entities().delete(&vehicle.id).send().await?;
    println!("deleted {}", vehicle.id);
    Ok(())
}
