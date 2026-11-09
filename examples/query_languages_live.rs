//! Puts each NGSI-LD query language to work on four vehicles in Madrid and Barcelona: `q`, `type`, `scopeQ`, the
//! geoquery, `pick`, `orderBy`, `lang` and a temporal query, each printing what it selects.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example query_languages_live`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::{
        DateTime,
        Entity,
        EntityFragment,
        EntityId,
        ErrorType,
        Lossless,
        Output,
        Property,
        Representation,
        geometry::{Position, Shape},
    },
    query::{
        geoquery,
        language_filter,
        ordering,
        projection::{self, EntityMember, builders::ProjectionBuilder},
        q,
        scope_query,
        temporal_query,
        type_selection,
    },
};
use std::env;
use url::Url;

/// One vehicle as the example writes it.
struct Vehicle {
    /// Its id.
    id: EntityId,
    /// Its entity types.
    types: &'static [&'static str],
    /// Its brand.
    brand: &'static str,
    /// Its speed at nine, and five minutes later.
    speeds: [i64; 2],
    /// Its scope.
    scope: &'static str,
    /// Where it is, as longitude and latitude.
    position: (f64, f64),
    /// Its name in English and Spanish.
    name: [&'static str; 2],
}

/// A `pick` of the id, the type and `attribute`.
fn keeping(attribute: &str) -> ProjectionBuilder {
    projection::member(EntityMember::Id).member(EntityMember::Type).attribute(attribute)
}

/// Prints the vehicles a query selected, under `label`.
fn print(label: &str, vehicles: &[Entity<Output>]) {
    println!("{label}:");
    for vehicle in vehicles {
        println!("  {}", vehicle.render(Representation::Concise));
    }
}

/// Deletes the vehicles and the history of each.
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

/// The four vehicles: two buses in central Madrid, an electric car by the Retiro and a car in Barcelona.
fn fleet() -> [Vehicle; 4] {
    [
        Vehicle {
            id: EntityId::from(iri!("urn:ngsi-ld:Vehicle:QL01")),
            types: &["Bus"],
            brand: "Irizar",
            speeds: [38, 41],
            scope: "/Madrid/Centro",
            position: (-3.7038, 40.4168),
            name: ["Red bus", "Autobus rojo"],
        },
        Vehicle {
            id: EntityId::from(iri!("urn:ngsi-ld:Vehicle:QL02")),
            types: &["Car", "Electric"],
            brand: "Renault",
            speeds: [52, 48],
            scope: "/Madrid/Retiro",
            position: (-3.6844, 40.4153),
            name: ["Blue car", "Coche azul"],
        },
        Vehicle {
            id: EntityId::from(iri!("urn:ngsi-ld:Vehicle:QL03")),
            types: &["Car"],
            brand: "Seat",
            speeds: [61, 65],
            scope: "/Barcelona/Gracia",
            position: (2.1734, 41.3851),
            name: ["White car", "Coche blanco"],
        },
        Vehicle {
            id: EntityId::from(iri!("urn:ngsi-ld:Vehicle:QL04")),
            types: &["Bus"],
            brand: "MAN",
            speeds: [0, 12],
            scope: "/Madrid/Centro/Sol",
            position: (-3.7050, 40.4170),
            name: ["Night bus", "Buho"],
        },
    ]
}

/// Creates the vehicles at nine, and changes the speed of each five minutes later.
async fn seed(client: &Client<ReqwestTransport>, fleet: &[Vehicle]) -> anyhow::Result<()> {
    let nine: DateTime = "2026-11-06T09:00:00Z".parse()?;
    let five_past: DateTime = "2026-11-06T09:05:00Z".parse()?;
    for vehicle in fleet {
        let entity = Entity::builder()
            .id(vehicle.id.clone())
            .entity_type(vehicle.types.iter().copied().map(str::parse).collect::<Result<Vec<_>, _>>()?)
            .scope(vehicle.scope)
            .property("brandName", vehicle.brand)
            .property("speed", Property::builder(vehicle.speeds[0]).observed_at(nine))
            .geo_property("location", Shape::Point(Position::try_new(vehicle.position.0, vehicle.position.1)?))
            .language_property("name", [("en", vehicle.name[0]), ("es", vehicle.name[1])])
            .build()?;
        client.entities().create(&entity).send().await?;
        let later = EntityFragment::builder()
            .property("speed", Property::builder(vehicle.speeds[1]).observed_at(five_past))
            .build()?;
        client.entities().update_attributes(&vehicle.id, &later).send().await?;
    }
    Ok(())
}

/// Selects vehicles with each query language of Query Entities.
async fn select(client: &Client<ReqwestTransport>) -> anyhow::Result<()> {
    let fast_not_seat = q::attr("speed").gt(40).and(q::attr("brandName").ne("Seat"));
    let page = client
        .entities()
        .query()
        .entity_type("Bus")
        .q(fast_not_seat.clone())
        .pick(keeping("speed"))
        .send()
        .await?;
    print("buses faster than 40 that are no Seat", page.items());
    let page = client
        .entities()
        .query()
        .entity_type("Car")
        .q(fast_not_seat)
        .pick(keeping("speed"))
        .send()
        .await?;
    print("cars faster than 40 that are no Seat", page.items());

    let buses_or_electric_cars = type_selection::entity_type("Bus").or_all_of(["Car", "Electric"]);
    let page = client
        .entities()
        .query()
        .entity_type(buses_or_electric_cars)
        .pick(keeping("brandName"))
        .send()
        .await?;
    print("type Bus|(Car;Electric)", page.items());

    let in_madrid = scope_query::subtree("/Madrid");
    let page = client
        .entities()
        .query()
        .entity_type("Bus")
        .scope_q(in_madrid)
        .pick(keeping("brandName"))
        .send()
        .await?;
    print("buses scoped under /Madrid", page.items());

    let sol = Shape::Point(Position::try_new(-3.7038, 40.4168)?);
    let near_sol = geoquery::near_max(2000, sol).geoproperty("location");
    let page = client
        .entities()
        .query()
        .entity_type("Car")
        .geo_q(near_sol)
        .pick(keeping("brandName"))
        .send()
        .await?;
    print("cars within 2 km of Puerta del Sol", page.items());

    let fastest_first = ordering::descending("speed");
    let page = client
        .entities()
        .query()
        .entity_type(type_selection::entity_type("Bus").or("Car"))
        .ordering(fastest_first)
        .pick(keeping("speed"))
        .send()
        .await?;
    print("buses and cars, fastest first", page.items());

    let page = client
        .entities()
        .query()
        .entity_type("Bus")
        .lang(language_filter::language("es"))
        .pick(keeping("name"))
        .send()
        .await?;
    print("the buses' names in Spanish", page.items());
    Ok(())
}

/// Selects the last speed of each bus with a temporal query.
async fn last_readings(client: &Client<ReqwestTransport>) -> anyhow::Result<()> {
    let last_reading = temporal_query::between("2026-11-06T09:00:00Z".parse()?, "2026-11-06T10:00:00Z".parse()?).last_n(1);
    let page = client
        .temporal()
        .query(last_reading)
        .entity_type("Bus")
        .attrs(vec!["speed".parse()?])
        .send()
        .await?;
    println!("the last speed of each bus between nine and ten:");
    for bus in page.items() {
        println!("  {}", bus.render(Lossless::Concise));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let fleet = fleet();
    let ids: Vec<EntityId> = fleet.iter().map(|vehicle| vehicle.id.clone()).collect();
    remove(&client, &ids).await?;
    seed(&client, &fleet).await?;
    select(&client).await?;
    last_readings(&client).await?;
    remove(&client, &ids).await?;
    Ok(())
}
