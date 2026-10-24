//! Keeps an entity in a tenant of its own: created with the `NGSILD-Tenant` header, missing from the default tenant,
//! found in its own.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example tenants`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{Entity, ErrorType, Representation, Tenant},
};
use std::{env, slice};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let tenant: Tenant = "fleet".parse()?;
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:T4001"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .build()?;
    // On a first run the tenant does not exist yet, so there is nothing to clean up.
    match client.entities().batch_delete(slice::from_ref(&vehicle.id)).tenant(&tenant).send().await {
        Ok(_) => {}
        Err(error) if error.error_type() == Some(&ErrorType::NonexistentTenant) => {}
        Err(error) => return Err(error.into()),
    }

    client.entities().create(&vehicle).tenant(&tenant).send().await?;
    println!("created {} in tenant {tenant}", vehicle.id);

    match client.entities().retrieve(&vehicle.id).send().await {
        Ok(_) => println!("the default tenant holds {}", vehicle.id),
        Err(error) if error.error_type() == Some(&ErrorType::ResourceNotFound) => {
            println!("the default tenant does not hold {}", vehicle.id);
        }
        Err(error) => return Err(error.into()),
    }
    let stored = client.entities().retrieve(&vehicle.id).tenant(&tenant).send().await?.into_body();
    println!("tenant {tenant} holds:\n{}", stored.render(Representation::Normalized));

    client.entities().delete(&vehicle.id).tenant(&tenant).send().await?;
    println!("deleted {} from tenant {tenant}", vehicle.id);
    Ok(())
}
