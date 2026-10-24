//! Query Entities through `GET /entities` (clause 6.4.3.2) and `POST /entityOperations/query` (clause 6.23.3.1):
//! type selection, `q`, `pick`, `limit`, `count` and the `next` links of clause 6.3.10.

use crate::broker::{attribute_names, client, create_fleet, remove_all};
use ngsi_ld::{
    client::{Client, ClientError, ReqwestError, ReqwestTransport},
    http::{
        Page,
        RequestError,
        operation::{ArgumentError, QueryMethod, ResultsCount},
    },
    model::{Entity, EntityId, Input, Output, SimplifiedEntity},
    query::{
        projection::{self, EntityMember},
        q::{self, builders::QueryBuilder},
    },
};
use pretty_assertions::assert_eq;

/// The most pages a test follows; more means the broker keeps handing out `next` links.
const MAX_PAGES: usize = 10;

/// Queries the vehicles of `vehicle_type` that match `q`, keeping `id`, `type` and `speed`, two per page with the
/// count, and follows every `next` link.
async fn query_pages(
    client: &Client<ReqwestTransport>,
    vehicle_type: &str,
    q: QueryBuilder,
    method: QueryMethod,
) -> Result<Vec<Page<Entity<Output>>>, ClientError<ReqwestError>> {
    let first = client
        .entities()
        .query()
        .entity_type(vehicle_type)
        .q(q)
        .pick(projection::member(EntityMember::Id).member(EntityMember::Type).attribute("speed"))
        .limit(2)
        .count(ResultsCount::Include)
        .method(method)
        .send()
        .await?;
    let mut walk = client.pages(first);
    let mut pages = Vec::new();
    while let Some(page) = walk.next().await {
        assert!(pages.len() < MAX_PAGES, "the broker hands out more pages than there are results");
        pages.push(page?);
    }
    Ok(pages)
}

/// The ids of the vehicles of `fleet` going faster than `speed`, sorted.
fn ids_faster_than(fleet: &[Entity<Input>], speed: usize) -> Vec<&str> {
    let mut ids: Vec<&str> = fleet.iter().skip(speed / 10).map(|vehicle| vehicle.id.as_str()).collect();
    ids.sort_unstable();
    ids
}

/// Checks the pages of a query that matches four of five vehicles with two per page: the count, the page sizes,
/// the matching ids and the projection.
fn assert_four_matches_in_two_pages(pages: &[Page<Entity<Output>>], expected: &[&str]) {
    assert_eq!(pages.len(), 2, "{pages:?}");
    assert_eq!(pages[0].total(), Some(4));
    assert!(pages.iter().all(|page| page.items().len() == 2), "{pages:?}");
    let mut found: Vec<&str> = pages.iter().flat_map(Page::items).map(|vehicle| vehicle.id.as_str()).collect();
    found.sort_unstable();
    assert_eq!(found, expected);
    assert!(
        pages.iter().flat_map(Page::items).all(|vehicle| attribute_names(vehicle) == ["speed"]),
        "{pages:?}"
    );
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_through_get_with_a_comparison_follows_the_next_links_to_every_match() {
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "queryGetComparison", 5).await;
    let pages = query_pages(&client, &vehicle_type, q::attr("speed").gt(15), QueryMethod::Get).await;
    remove_all(&client, &fleet).await;
    assert_four_matches_in_two_pages(&pages.unwrap(), &ids_faster_than(&fleet, 15));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_through_get_with_a_value_list_follows_the_next_links_to_every_match() {
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "queryGetList", 5).await;
    let pages = query_pages(&client, &vehicle_type, q::attr("speed").any_of([20, 30, 40, 50]), QueryMethod::Get).await;
    remove_all(&client, &fleet).await;
    assert_four_matches_in_two_pages(&pages.unwrap(), &ids_faster_than(&fleet, 15));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_through_post_follows_the_next_links_to_every_match() {
    // Known deviation, clause 6.3.10 with clause 6.23.3.1: the `next` link leads to the next page of the same
    // query. Observed: the broker answers the POST with the link `/ngsi-ld/v1/entities?count=true&limit=2&offset=2`,
    // which drops the type, `q` and `pick` of the query body, and that link is answered with 400 InvalidRequest.
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "queryPost", 5).await;
    let pages = query_pages(&client, &vehicle_type, q::attr("speed").any_of([20, 30, 40, 50]), QueryMethod::Post).await;
    remove_all(&client, &fleet).await;
    assert_four_matches_in_two_pages(&pages.unwrap(), &ids_faster_than(&fleet, 15));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_simplified_returns_every_entity_of_the_type_in_simplified_form() {
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "querySimplified", 3).await;
    let page = client.entities().query_simplified().entity_type(vehicle_type.as_str()).send().await;
    remove_all(&client, &fleet).await;
    let mut found = page.unwrap().into_items();
    found.sort_unstable_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    let expected: Vec<SimplifiedEntity> = fleet.iter().map(SimplifiedEntity::from).collect();
    assert_eq!(found, expected);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_by_ids_returns_exactly_those_entities() {
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "queryIds", 3).await;
    // The query owns the ids it selects, and the fleet keeps its own for the cleanup.
    let wanted: Vec<EntityId> = vec![fleet[0].id.clone(), fleet[2].id.clone()];
    let page = client.entities().query().entity_type(vehicle_type.as_str()).ids(wanted).send().await;
    remove_all(&client, &fleet).await;
    let mut found: Vec<String> = page.unwrap().items().iter().map(|vehicle| vehicle.id.to_string()).collect();
    found.sort_unstable();
    assert_eq!(found, [fleet[0].id.to_string(), fleet[2].id.to_string()]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_with_limit_zero_and_count_returns_the_total_and_no_entity() {
    // Known deviation, clauses 6.3.13 and 6.4.3.2: the answer is 200 with the count and an empty JSON array as its
    // body. Observed: the broker answers 200 with `NGSILD-Results-Count` and an empty body, which is not JSON.
    let client = client();
    let (vehicle_type, fleet) = create_fleet(&client, "queryCountOnly", 3).await;
    let page = client
        .entities()
        .query()
        .entity_type(vehicle_type.as_str())
        .limit(0)
        .count(ResultsCount::Include)
        .send()
        .await;
    remove_all(&client, &fleet).await;
    let page = page.unwrap();
    assert_eq!(page.total(), Some(3));
    assert_eq!(page.items(), []);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_with_limit_zero_without_count_is_refused_before_sending() {
    let error = client().entities().query().entity_type("Vehicle").limit(0).send().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::ZeroLimitWithoutCount))),
        "{error:?}"
    );
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn query_without_type_or_q_is_refused_before_sending() {
    let error = client().entities().query().send().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Request(RequestError::Argument(ArgumentError::UnboundedQuery))),
        "{error:?}"
    );
}
