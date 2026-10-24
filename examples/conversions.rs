//! Converts NGSI-LD temporal values to and from `chrono`, and geometries to and from `geo-types` and `geojson`,
//! including what NGSI-LD refuses, and builds a store's `location` from a `geo-types` polygon.
//!
//! Run it with `cargo run --features chrono,geo-types,geojson --example conversions`.

use anyhow::{Context, bail};
use chrono::{DateTime as ChronoDateTime, NaiveDate, NaiveTime, Utc};
use geo_types::{Geometry as GeoTypesGeometry, LineString as GeoTypesLineString, Point, Polygon, Rect};
use geojson::{Geometry as GeoJsonGeometry, GeometryValue};
use iri_rs::iri;
use ngsi_ld::model::{
    Attribute,
    AttributeName,
    Date,
    DateTime,
    Entity,
    Instances,
    Representation,
    Time,
    geometry::{Geometry, Position, Shape},
};

fn main() -> anyhow::Result<()> {
    let reading: DateTime = "2018-08-01T12:03:00.250Z".parse()?;
    let instant = ChronoDateTime::<Utc>::from(reading);
    println!("{reading} is {} in chrono, and {} back", instant.to_rfc3339(), DateTime::try_from(instant)?);
    let day: Date = "2018-08-01".parse()?;
    println!(
        "{day} is {} in chrono, and {} back",
        NaiveDate::from(day),
        Date::try_from(NaiveDate::from(day))?
    );
    let opening: Time = "08:30:00.5Z".parse()?;
    println!(
        "{opening} is {} in chrono, and {} back",
        NaiveTime::from(opening),
        Time::try_from(NaiveTime::from(opening))?
    );
    // NGSI-LD writes at most six fraction digits, so a nanosecond has no NGSI-LD form.
    let precise = ChronoDateTime::<Utc>::from_timestamp(instant.timestamp(), 250_000_001).context("the instant is outside chrono's range")?;
    match DateTime::try_from(precise) {
        Ok(converted) => println!("{} is {converted}", precise.to_rfc3339()),
        Err(error) => println!("{} is refused: {error}", precise.to_rfc3339()),
    }

    // geo-types is two-dimensional, so the altitude is dropped.
    let entrance = Shape::Point(Position::with_altitude(13.3986, 52.5547, 34.0)?);
    println!(
        "{entrance} comes back from geo-types as {}",
        Shape::try_from(GeoTypesGeometry::from(&entrance))?
    );
    let refused = [
        GeoTypesGeometry::Rect(Rect::new((13.39, 52.55), (13.40, 52.56))),
        GeoTypesGeometry::Point(Point::new(200.0, 52.55)),
    ];
    for geometry in refused {
        match Shape::try_from(geometry) {
            Ok(shape) => println!("a geo-types geometry becomes {shape}"),
            Err(error) => println!("a geo-types geometry is refused: {error}"),
        }
    }

    // The altitude, bounding box and foreign members of a GeoJSON geometry are all kept.
    let text = r#"{"type":"Point","coordinates":[13.3986,52.5547,34.0],"bbox":[13.3986,52.5547,34.0,13.3986,52.5547,34.0],"name":"entrance"}"#;
    let read: GeoJsonGeometry = text.parse()?;
    let geometry = Geometry::try_from(&read)?;
    println!("{geometry} is {} in geojson", GeoJsonGeometry::try_from(&geometry)?);
    let collection = GeoJsonGeometry::new(GeometryValue::GeometryCollection { geometries: vec![read] });
    match Geometry::try_from(&collection) {
        Ok(geometry) => println!("a geojson GeometryCollection becomes {geometry}"),
        Err(error) => println!("a geojson GeometryCollection is refused: {error}"),
    }

    // A geo-types polygon closes its exterior ring.
    let ring = GeoTypesLineString::from(vec![(13.3986, 52.5547), (13.3990, 52.5547), (13.3990, 52.5550), (13.3986, 52.5550)]);
    let outline = GeoTypesGeometry::Polygon(Polygon::new(ring, Vec::new()));
    let store = Entity::builder()
        .id(iri!("urn:ngsi-ld:Building:store001"))
        .entity_type("Building")
        .geo_property("location", Shape::try_from(outline)?)
        .build()?;
    println!("the store: {}", store.render(Representation::Normalized));
    let location: AttributeName = "location".parse()?;
    let Some(Attribute::GeoProperty(property)) = store.attribute(&location).map(Instances::first) else {
        bail!("the store has no location");
    };
    let geometry = property.value().as_geometry().context("the store's location is null")?;
    println!("its location as geo-types: {:?}", GeoTypesGeometry::from(geometry.shape()));
    Ok(())
}
