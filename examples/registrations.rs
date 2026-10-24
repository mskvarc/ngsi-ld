//! Registers a context source of truck speeds under a subscription to the registrations of trucks, finds it again by
//! entity type and attribute, describes it with a fragment, and deletes both.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example registrations`.

use anyhow::Context;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::AttributeName,
    registration::{CSourceRegistration, RegistrationFragment, RegistrationInfo},
    subscription::{Endpoint, NotificationParams, Subscription},
};
use std::env;
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    // Nobody listens on either endpoint: the example only manages the registration and the subscription.
    let endpoint = Endpoint::builder("http://notifications.example/registrations").build()?;
    let watch = Subscription::registration_builder()
        .entities(["Truck"])
        .notification(NotificationParams::builder(endpoint).build()?)
        .build()?;
    let created = client.registration_subscriptions().create(&watch).send().await?;
    let watch_id = created.id().context("the broker named no subscription in the Location of its answer")?;
    println!("watching the registrations of trucks with {watch_id}");

    let source = CSourceRegistration::builder("http://csource.example:1026")
        .registration_name("truck speeds")
        .information([RegistrationInfo::builder().entities(["Truck"]).property_names(["speed"]).build()?])
        .build()?;
    let created = client.registrations().register(&source).send().await?;
    let id = created.id().context("the broker named no registration in the Location of its answer")?;
    println!("registered {id}");

    let speed: AttributeName = "speed".parse()?;
    let found = client.registrations().query().entity_type("Truck").attrs(vec![speed]).send().await?;
    println!("{} registration(s) provide the speed of trucks", found.items().len());

    let describe = RegistrationFragment::builder().description("speeds measured on the road").build()?;
    client.registrations().update(&id, &describe).send().await?;
    let stored = client.registrations().retrieve(&id).send().await?;
    println!("{id} is now described as {}", stored.description().unwrap_or_default());

    client.registrations().delete(&id).send().await?;
    println!("deleted {id}");
    client.registration_subscriptions().delete(&watch_id).send().await?;
    println!("deleted {watch_id}");
    Ok(())
}
