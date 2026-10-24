//! Queries entities by place and by scope: a geoquery by distance and by containment, a scope query by subtree, by
//! wildcard and by alternatives, and both together with a type selection and `q`, through GET and through POST.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --example geo_scope_query`.

use anyhow::Context;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::QueryMethod,
    model::{
        Entity,
        EntityId,
        Output,
        geometry::{LinearRing, Polygon, Position, Shape},
    },
    query::{geoquery, q, scope_query, type_selection},
};
use std::env;
use url::Url;

/// A polygon around central Madrid, holding Puerta del Sol and the Retiro.
fn central_madrid() -> anyhow::Result<Shape> {
    let corners = [(-3.8, 40.3), (-3.6, 40.3), (-3.6, 40.5), (-3.8, 40.5), (-3.8, 40.3)];
    let ring = LinearRing::try_new(
        corners
            .into_iter()
            .map(|(longitude, latitude)| Position::try_new(longitude, latitude))
            .collect::<Result<_, _>>()?,
    )?;
    Ok(Shape::Polygon(Polygon::try_new(vec![ring])?))
}

/// Prints the ids of the entities a query matched.
fn print_matches(query: &str, entities: &[Entity<Output>]) {
    let ids: Vec<String> = entities.iter().map(|entity| entity.id.to_string()).collect();
    println!("{query}: {}", ids.join(", "));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    // Puerta del Sol and the Retiro are about 1.3 km apart in Madrid.
    let sol = Shape::Point(Position::try_new(-3.7038, 40.4168)?);
    let retiro = Shape::Point(Position::try_new(-3.6884, 40.4153)?);
    let barcelona = Shape::Point(Position::try_new(2.1734, 41.3851)?);

    let places = [
        (iri!("urn:ngsi-ld:Vehicle:G1301"), "Vehicle", sol.clone(), "/Madrid/Gardens", 30),
        (iri!("urn:ngsi-ld:Vehicle:G1302"), "Vehicle", retiro.clone(), "/Madrid/Sights", 60),
        (iri!("urn:ngsi-ld:Vehicle:G1303"), "Vehicle", barcelona, "/Barcelona", 90),
        (iri!("urn:ngsi-ld:Bicycle:G1304"), "Bicycle", retiro, "/Madrid/Sights", 20),
    ];
    let ids = places.each_ref().map(|(id, ..)| EntityId::from(*id));
    client.entities().batch_delete(&ids).send().await?;
    for (id, entity_type, location, scope, speed) in places {
        let entity = Entity::builder()
            .id(id)
            .entity_type(entity_type)
            .scope(scope)
            .geo_property("location", location)
            .property("speed", speed)
            .build()?;
        client.entities().create(&entity).send().await?;
        println!("created {}", entity.id);
    }

    let page = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .geo_q(geoquery::near_max(2000, sol.clone()))
        .send()
        .await?;
    print_matches("vehicles within 2 km of Puerta del Sol", page.items());

    let page = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .geo_q(geoquery::near_min(2000, sol.clone()))
        .send()
        .await?;
    print_matches("vehicles farther than 2 km from Puerta del Sol", page.items());

    let page = client
        .entities()
        .query()
        .entity_type(type_selection::entity_type("Vehicle").or("Bicycle"))
        .geo_q(geoquery::within(central_madrid()?).geoproperty("location"))
        .send()
        .await?;
    print_matches("vehicles and bicycles inside central Madrid", page.items());

    let page = client
        .entities()
        .query()
        .entity_type(type_selection::entity_type("Vehicle").or("Bicycle"))
        .scope_q(scope_query::subtree("/Madrid"))
        .send()
        .await?;
    print_matches("vehicles and bicycles in the Madrid scope tree", page.items());

    let page = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .scope_q(scope_query::scope("/Madrid/+"))
        .send()
        .await?;
    print_matches("vehicles in any district of Madrid", page.items());

    let page = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .scope_q(scope_query::scope("/Barcelona").or("/Madrid/Sights"))
        .send()
        .await?;
    print_matches("vehicles in Barcelona or among the sights of Madrid", page.items());

    for method in [QueryMethod::Get, QueryMethod::Post] {
        let page = client
            .entities()
            .query()
            .entity_type("Vehicle")
            .geo_q(geoquery::near_max(2000, sol.clone()))
            .scope_q(scope_query::subtree("/Madrid"))
            .q(q::attr("speed").gt(40))
            .method(method)
            .send()
            .await?;
        print_matches(&format!("vehicles faster than 40 near Puerta del Sol in Madrid, by {method:?}"), page.items());
    }

    client.entities().batch_delete(&ids).send().await?;
    println!("deleted {} entities", ids.len());
    Ok(())
}
