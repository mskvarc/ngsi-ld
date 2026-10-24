//! Builds, prints and parses each NGSI-LD query language: `q`, `type`, `scopeQ`, the geoquery, `pick` and `omit`,
//! `orderBy`, `lang` and the `TemporalQuery` object. Needs no broker and no feature.
//!
//! Run it with `cargo run --example query_languages`.

use ngsi_ld::{
    model::{
        evolution::AggregationMethod,
        geometry::{LinearRing, Polygon, Position, Shape},
    },
    query::{
        Quality,
        geoquery::{self, GeoQuery},
        language_filter::{self, LanguageFilter},
        ordering::{self, OrderBy},
        projection::{self, EntityMember, Projection},
        q::{self, Query},
        scope_query::{self, ScopeQuery},
        temporal_query::{self, TemporalProperty, TemporalQuery},
        type_selection::{self, TypeSelection},
    },
};

fn main() -> anyhow::Result<()> {
    let q = q::attr("speed")
        .within(40, 60)
        .and(q::attr("brandName").eq("Mercedes").or(q::attr("address").member("city").matches("^Ber")))
        .build()?;
    println!("q: {q}");
    let parsed: Query = "brandName==\"Mercedes\";(speed>50|rpm>3000)".parse()?;
    println!("  parsed: {parsed}");
    // A context source filter is written in the same language, over the properties of a registration.
    let csf = q::attr("provider").eq("City Council").build()?;
    println!("csf: {csf}");

    let selection = type_selection::entity_type("Vehicle").or_all_of(["Car", "Electric"]).build()?;
    println!("type: {selection}");
    let parsed: TypeSelection = "Vehicle|(Car;Electric)".parse()?;
    println!("  parsed: {parsed}");

    let scope = scope_query::all_of(["/Madrid/Districts", "/CompanyA"])
        .or_subtree("/Madrid/Gardens")
        .or("/Madrid/+/ParqueNorte")
        .build()?;
    println!("scopeQ: {scope}");
    // `,` is another spelling of `|`; the printer writes `|`.
    let parsed: ScopeQuery = "/Madrid,/Barcelona".parse()?;
    println!("  parsed: {parsed}");

    let sol = Shape::Point(Position::new(-3.7038, 40.4168)?);
    let near = geoquery::near_max(2000, sol.clone()).geoproperty("location").build()?;
    println!("georel: {}", near.relation());
    println!("geoQ: {}", near.render());
    let corners = [(-3.8, 40.3), (-3.6, 40.3), (-3.6, 40.5), (-3.8, 40.5), (-3.8, 40.3)];
    let ring = LinearRing::new(
        corners
            .into_iter()
            .map(|(longitude, latitude)| Position::new(longitude, latitude))
            .collect::<Result<_, _>>()?,
    )?;
    let within = geoquery::within(Shape::Polygon(Polygon::new(vec![ring])?)).build()?;
    println!("geoQ: {}", within.render());
    let parsed = GeoQuery::parse(r#"{"georel": "near;maxDistance==500", "geometry": "Point", "coordinates": [2.1734, 41.3851]}"#)?;
    println!("  parsed: {}", parsed.render());

    let pick = projection::member(EntityMember::Id)
        .member(EntityMember::Type)
        .attribute("speed")
        .linked("isParked", projection::attributes(["name", "totalSpotNumber"]))
        .build()?;
    println!("pick: {pick}");
    let omit = projection::member(EntityMember::Scope).attribute("brandName").build()?;
    println!("omit: {omit}");
    let parsed: Projection = "id,speed,isParked{name}".parse()?;
    println!("  parsed: {parsed}");

    let ordering = ordering::by("name")
        .then_descending("speed")
        .then_nearest("location")
        .origin(sol)
        .collation("de-u-co-phonebk")
        .build()?;
    println!("orderBy: {}", ordering.order_by());
    let parsed: OrderBy = "name,speed;desc".parse()?;
    println!("  parsed: {parsed}");

    let lang = language_filter::language("nl")
        .weighted_language("en", Quality::from_thousandths(500)?)
        .build()?;
    println!("lang: {lang}");
    let parsed: LanguageFilter = "fr,en;q=0.8".parse()?;
    println!("  parsed: {parsed}");

    let temporal = temporal_query::between("2018-08-01T12:00:00Z".parse()?, "2018-08-01T13:00:00Z".parse()?)
        .time_property(TemporalProperty::ObservedAt)
        .last_n(3)
        .aggregation_methods([AggregationMethod::Max, AggregationMethod::Avg])
        .aggregation_period("PT5M".parse()?)
        .build()?;
    println!("temporalQ: {}", temporal.render());
    let parsed = TemporalQuery::parse(r#"{"timerel": "before", "timeAt": "2018-08-01T12:00:00Z"}"#)?;
    println!("  parsed: {}", parsed.render());
    Ok(())
}
