//! Records the speed of a vehicle over time and reads it back normalized, as temporal values and as the maximum per
//! period; queries vehicles within a time range, corrects one reading by its instance id and deletes the evolution.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features reqwest --example temporal`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{
        Attribute,
        AttributeName,
        DateTime,
        EntityId,
        ErrorType,
        Input,
        Property,
        evolution::{AggregationMethod, Evolution, TemporalEntity},
    },
    query::temporal_query::{self, builders::TemporalQueryBuilder},
};
use std::env;
use url::Url;

/// The time of a reading at `minute` past noon.
fn at(minute: u32) -> anyhow::Result<DateTime> {
    Ok(format!("2018-08-01T12:{minute:02}:00Z").parse()?)
}

/// The evolution of the vehicle, with a speed reading at each of `readings`.
fn readings(readings: &[(u32, i64)]) -> anyhow::Result<TemporalEntity<Input>> {
    let mut builder = TemporalEntity::builder().id(iri!("urn:ngsi-ld:Vehicle:H1201")).entity_type("Vehicle");
    for &(minute, speed) in readings {
        builder = builder.property("speed", Property::builder(speed).observed_at(at(minute)?));
    }
    Ok(builder.build()?)
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

    let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:H1201"));
    // Removes the evolution an earlier run left behind, if there is one.
    match client.temporal().delete(&id).send().await {
        Ok(()) => {}
        Err(error) if error.error_type() == Some(&ErrorType::ResourceNotFound) => {}
        Err(error) => return Err(error.into()),
    }

    let created = client.temporal().create_or_update(&readings(&[(3, 30), (5, 50), (7, 70)])?).send().await?;
    println!("recorded {id}: {created:?}");
    let added = client.temporal().create_or_update(&readings(&[(9, 90)])?).send().await?;
    println!("added a reading: {added:?}");

    let speed: AttributeName = "speed".parse()?;
    let day = temporal_query::between("2018-08-01T00:00:00Z".parse()?, "2018-08-02T00:00:00Z".parse()?).build()?;
    let normalized = client.temporal().retrieve(&id).temporal_q(day.clone()).send().await?.into_body();
    println!("{id} has {} speed readings", normalized.attribute(&speed).map_or(0, Evolution::len));
    let values = client.temporal().retrieve_values(&id).temporal_q(day.clone()).send().await?.into_body();
    println!("as temporal values: {}", values.render());
    let per_five_minutes = TemporalQueryBuilder::from(day)
        .aggregation_methods([AggregationMethod::Max])
        .aggregation_period("PT5M".parse()?);
    let maximum = client
        .temporal()
        .retrieve_aggregated(&id)
        .temporal_q(per_five_minutes)
        .send()
        .await?
        .into_body();
    println!("the maximum per five minutes: {}", maximum.render());

    let page = client
        .temporal()
        .query(temporal_query::between(at(0)?, at(6)?))
        .entity_type("Vehicle")
        .send()
        .await?;
    println!("{} vehicles moved in the first six minutes", page.items().len());

    // The instances of an evolution come in no fixed order, so the first reading is found by its observedAt,
    // compared as an instant.
    let first_minute = at(3)?.timestamp();
    let first = normalized
        .attribute(&speed)
        .and_then(|evolution| {
            evolution
                .iter()
                .find(|reading| reading.meta().observed_at.is_some_and(|observed| observed.timestamp() == first_minute))
        })
        .and_then(|reading| reading.meta().output.instance_id.clone())
        .context("the broker returned no identified reading observed at 12:03")?;
    let corrected: Attribute<Input> = Property::builder(42).observed_at(at(3)?).build()?.into();
    client.temporal().modify_instance(&id, &speed, &first, &corrected).send().await?;
    println!("corrected the reading {first}");

    client.temporal().delete(&id).send().await?;
    println!("deleted the evolution of {id}");
    Ok(())
}
