//! Puts two stores on a broker with outlines drawn in `geo-types`, finds the one inside a district read with
//! `geojson`, reads its outline back as `geo-types`, and works out with `chrono` when its visitor counts were taken.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features chrono,geo-types,geojson --example conversions_live`.

use anyhow::{Context, bail};
use chrono::{TimeDelta, TimeZone, Utc};
use geo_types::{Geometry as GeoTypesGeometry, Rect};
use geojson::Geometry as GeoJsonGeometry;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{
        Attribute,
        AttributeName,
        DateTime,
        Entity,
        EntityFragment,
        EntityId,
        ErrorType,
        Instances,
        Property,
        geometry::{Geometry, Shape},
    },
    query::geoquery,
};
use std::env;
use url::Url;

/// The district of Prenzlauer Berg as a city's open data publishes it, simplified to a rectangle.
const DISTRICT: &str = r#"{"type":"Polygon","coordinates":[[[13.40,52.53],[13.45,52.53],[13.45,52.56],[13.40,52.56],[13.40,52.53]]]}"#;

/// Deletes the stores and the history of each.
async fn remove(client: &Client<ReqwestTransport>, ids: &[EntityId]) -> anyhow::Result<()> {
    client.entities().batch_delete(ids).send().await?;
    for id in ids {
        match client.temporal().delete(id).send().await {
            Ok(()) => {}
            Err(error) if error.error_type() == Some(&ErrorType::ResourceNotFound) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let opening = Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).single().context("the opening time is ambiguous")?;
    let stores = [
        (
            EntityId::from(iri!("urn:ngsi-ld:Building:store001")),
            Rect::new((13.4120, 52.5390), (13.4128, 52.5395)),
        ),
        (
            EntityId::from(iri!("urn:ngsi-ld:Building:store002")),
            Rect::new((13.3770, 52.5160), (13.3778, 52.5165)),
        ),
    ];
    let ids = stores.each_ref().map(|(id, _)| id.clone());
    remove(&client, &ids).await?;

    for (id, outline) in &stores {
        let store = Entity::builder()
            .id(id.clone())
            .entity_type("Building")
            .geo_property("location", Shape::try_from(GeoTypesGeometry::Polygon(outline.to_polygon()))?)
            .property("visitors", Property::builder(0).observed_at(DateTime::try_from(opening)?))
            .build()?;
        client.entities().create(&store).send().await?;
    }
    for (minutes, visitors) in [(45, 120), (150, 310)] {
        let counted = opening + TimeDelta::minutes(minutes);
        let count = EntityFragment::builder()
            .property("visitors", Property::builder(visitors).observed_at(DateTime::try_from(counted)?))
            .build()?;
        client.entities().update_attributes(&ids[0], &count).send().await?;
    }

    let district = Geometry::try_from(&DISTRICT.parse::<GeoJsonGeometry>()?)?;
    let inside = client
        .entities()
        .query()
        .entity_type("Building")
        .geo_q(geoquery::within(district.shape().clone()).geoproperty("location"))
        .send()
        .await?;
    let location: AttributeName = "location".parse()?;
    for store in inside.items() {
        let Some(Attribute::GeoProperty(property)) = store.attribute(&location).map(Instances::first) else {
            bail!("{} has no location", store.id);
        };
        let outline = property.value().as_geometry().context("the store's location is null")?;
        println!(
            "{} is in the district, with the outline {:?}",
            store.id,
            GeoTypesGeometry::from(outline.shape())
        );
    }

    let visitors: AttributeName = "visitors".parse()?;
    let history = client.temporal().retrieve(&ids[0]).attrs(vec!["visitors".parse()?]).send().await?.into_body();
    let readings = history.attribute(&visitors).context("the store has no visitor history")?;
    for reading in readings {
        let Attribute::Property(count) = reading else {
            bail!("a visitor count is not a Property");
        };
        let taken = count.meta().observed_at.context("a visitor count has no observedAt")?;
        let after = chrono::DateTime::<Utc>::from(taken) - opening;
        println!("{} visitors, counted {} minutes after opening", count.value(), after.num_minutes());
    }

    remove(&client, &ids).await?;
    Ok(())
}
