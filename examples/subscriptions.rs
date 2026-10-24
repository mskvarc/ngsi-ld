//! Manages a subscription to fast vehicles: creates it, reads it back, lists every subscription page by page, pauses
//! it and narrows its query with a fragment, and deletes it.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example subscriptions`.

use anyhow::Context;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    query::q,
    subscription::{Activation, Endpoint, NotificationParams, Subscription, SubscriptionFragment},
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

    // Nobody listens on the endpoint: the example only manages the subscription.
    let endpoint = Endpoint::builder("http://notifications.example/vehicles").build()?;
    let subscription = Subscription::builder()
        .subscription_name("fast vehicles")
        .entities(["Vehicle"])
        .watched_attributes(["speed"])
        .q(q::attr("speed").gt(50).build()?)
        .notification(NotificationParams::builder(endpoint).build()?)
        .build()?;
    let created = client.subscriptions().create(&subscription).send().await?;
    let id = created.id().context("the broker named no subscription in the Location of its answer")?;
    println!("created {id}");

    let stored = client.subscriptions().retrieve(&id).send().await?;
    println!("{id} is {:?}, watching {:?}", stored.output().status, stored.watched_attributes());

    let first = client.subscriptions().query().limit(5).send().await?;
    let every = client.pages(first).try_collect().await?;
    println!("the broker holds {} subscriptions", every.len());

    let pause = SubscriptionFragment::builder()
        .activation(Activation::Paused)
        .q(q::attr("speed").gt(80).build()?)
        .build()?;
    client.subscriptions().update(&id, &pause).send().await?;
    let paused = client.subscriptions().retrieve(&id).send().await?;
    println!("{id} is now {:?}, with q {:?}", paused.output().status, paused.q().map(ToString::to_string));

    client.subscriptions().delete(&id).send().await?;
    println!("deleted {id}");
    Ok(())
}
