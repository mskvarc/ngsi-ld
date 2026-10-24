//! Query Entities and Purge Entities with a geoquery (clause 4.10) and a scope query (clause 4.19), through
//! `GET /entities`, `POST /entityOperations/query` and `DELETE /entities`.

use crate::broker::{client, create, remove_all, unique};
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{BatchOutcome, operation::QueryMethod},
    model::{
        Entity,
        Input,
        geometry::{LinearRing, Polygon, Position, Shape},
    },
    query::{
        geoquery::{self, builders::GeoQueryBuilder},
        scope_query::{self, builders::ScopeQueryBuilder},
    },
};
use pretty_assertions::assert_eq;

/// Puerta del Sol, Madrid.
fn sol() -> Shape {
    Shape::Point(Position::new(-3.7038, 40.4168).unwrap())
}

/// A polygon around central Madrid.
fn central_madrid() -> Shape {
    let corners = [(-3.8, 40.3), (-3.6, 40.3), (-3.6, 40.5), (-3.8, 40.5), (-3.8, 40.3)];
    let ring = LinearRing::new(
        corners
            .into_iter()
            .map(|(longitude, latitude)| Position::new(longitude, latitude).unwrap())
            .collect(),
    )
    .unwrap();
    Shape::Polygon(Polygon::new(vec![ring]).unwrap())
}

/// Creates three vehicles of a type unique to the calling test, under the scope root `root`: `n1` at Puerta del Sol
/// scoped `/{root}/Madrid/Gardens`, `n2` at the Retiro, about 1.3 km away, scoped `/{root}/Madrid/Sights`, and `n3` in
/// Barcelona scoped `/{root}/Barcelona`. A root of its own keeps a test's scope queries away from the vehicles of the
/// tests running beside it.
async fn create_placed_fleet(client: &Client<ReqwestTransport>, label: &str, root: &str) -> (String, Vec<Entity<Input>>) {
    let vehicle_type = unique(label);
    let places = [
        (sol(), "Madrid/Gardens"),
        (Shape::Point(Position::new(-3.6884, 40.4153).unwrap()), "Madrid/Sights"),
        (Shape::Point(Position::new(2.1734, 41.3851).unwrap()), "Barcelona"),
    ];
    let mut vehicles = Vec::new();
    for (number, (location, scope)) in (1..).zip(places) {
        let id = format!("urn:ngsi-ld:Vehicle:{vehicle_type}n{number}");
        let vehicle = Entity::builder()
            .id(id.as_str())
            .entity_type(vehicle_type.as_str())
            .scope(format!("/{root}/{scope}").as_str())
            .geo_property("location", location)
            .build()
            .unwrap();
        create(client, &vehicle).await;
        vehicles.push(vehicle);
    }
    (vehicle_type, vehicles)
}

/// The ids of the vehicles of `vehicle_type` that `geo_q` and `scope_q` match, sorted.
async fn matching(
    client: &Client<ReqwestTransport>,
    vehicle_type: &str,
    geo_q: Option<GeoQueryBuilder>,
    scope_q: Option<ScopeQueryBuilder>,
    method: QueryMethod,
) -> Vec<String> {
    let page = client
        .entities()
        .query()
        .entity_type(vehicle_type)
        .maybe_geo_q(geo_q)
        .maybe_scope_q(scope_q)
        .method(method)
        .send()
        .await
        .unwrap();
    let mut ids: Vec<String> = page.items().iter().map(|vehicle| vehicle.id.as_str().to_owned()).collect();
    ids.sort_unstable();
    ids
}

/// The ids of the vehicles of `fleet` at the positions `numbers` (1 for `n1`), sorted.
fn ids_of(fleet: &[Entity<Input>], numbers: &[usize]) -> Vec<String> {
    let mut ids: Vec<String> = numbers.iter().map(|number| fleet[number - 1].id.as_str().to_owned()).collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn geoqueries_match_by_distance_and_by_containment() {
    let client = client();
    let (vehicle_type, fleet) = create_placed_fleet(&client, "geoquery", &unique("geoRoot")).await;
    let mut found = Vec::new();
    for method in [QueryMethod::Get, QueryMethod::Post] {
        found.push(matching(&client, &vehicle_type, Some(geoquery::near_max(2000, sol())), None, method).await);
        found.push(
            matching(
                &client,
                &vehicle_type,
                Some(geoquery::within(central_madrid()).geoproperty("location")),
                None,
                method,
            )
            .await,
        );
        found.push(matching(&client, &vehicle_type, Some(geoquery::near_min(2000, sol())), None, method).await);
    }
    remove_all(&client, &fleet).await;
    let madrid = ids_of(&fleet, &[1, 2]);
    let barcelona = ids_of(&fleet, &[3]);
    assert_eq!(found, [madrid.clone(), madrid.clone(), barcelona.clone(), madrid.clone(), madrid, barcelona]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn scope_queries_match_subtrees_wildcards_and_alternatives() {
    let client = client();
    let root = unique("scopeRoot");
    let (vehicle_type, fleet) = create_placed_fleet(&client, "scopeQuery", &root).await;
    let mut found = Vec::new();
    for method in [QueryMethod::Get, QueryMethod::Post] {
        let madrid = scope_query::subtree(format!("/{root}/Madrid"));
        found.push(matching(&client, &vehicle_type, None, Some(madrid), method).await);
        let any_district = scope_query::scope(format!("/{root}/Madrid/+"));
        found.push(matching(&client, &vehicle_type, None, Some(any_district), method).await);
        let alternatives = scope_query::scope(format!("/{root}/Barcelona")).or(format!("/{root}/Madrid/Sights"));
        found.push(matching(&client, &vehicle_type, None, Some(alternatives), method).await);
    }
    remove_all(&client, &fleet).await;
    let madrid = ids_of(&fleet, &[1, 2]);
    let alternatives = ids_of(&fleet, &[2, 3]);
    assert_eq!(
        found,
        [madrid.clone(), madrid.clone(), alternatives.clone(), madrid.clone(), madrid, alternatives]
    );
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_scope_query_with_alternatives_matches_only_the_queried_type() {
    // Known deviation, clause 5.7.2.4: an entity matches every filter of the query, its type and its scope query.
    // The broker also returns the vehicle of the other type whose scope the second alternative names, as if the
    // type applied to the first alternative only.
    let client = client();
    let root = unique("sharedRoot");
    let (vehicle_type, fleet) = create_placed_fleet(&client, "scopeTypeA", &root).await;
    let (_, other_fleet) = create_placed_fleet(&client, "scopeTypeB", &root).await;
    let mut found = Vec::new();
    for method in [QueryMethod::Get, QueryMethod::Post] {
        let alternatives = scope_query::scope(format!("/{root}/Barcelona")).or(format!("/{root}/Madrid/Sights"));
        found.push(matching(&client, &vehicle_type, None, Some(alternatives), method).await);
    }
    remove_all(&client, &fleet).await;
    remove_all(&client, &other_fleet).await;
    let alternatives = ids_of(&fleet, &[2, 3]);
    assert_eq!(found, [alternatives.clone(), alternatives]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn purge_by_scope_query_deletes_only_the_entities_in_scope() {
    // Known deviation, clause 6.4.3.3: as for every purge, the broker answers 405.
    let client = client();
    let root = unique("purgeRoot");
    let (vehicle_type, fleet) = create_placed_fleet(&client, "purgeScope", &root).await;
    let outcome = client
        .entities()
        .purge()
        .entity_type(vehicle_type.as_str())
        .scope_q(format!("/{root}/Barcelona").as_str())
        .send()
        .await;
    let remaining = matching(&client, &vehicle_type, None, None, QueryMethod::Get).await;
    remove_all(&client, &fleet).await;
    assert_eq!(outcome.unwrap(), BatchOutcome::Done);
    assert_eq!(remaining, ids_of(&fleet, &[1, 2]));
}
