//! Query Entities with a geoquery and a scope query, over the client (clauses 4.10, 4.19 and 6.4.3.2).

use crate::fixtures::client;
use ngsi_ld::{
    client::ClientError,
    http::{RequestError, operation::ArgumentError},
    model::geometry::{Position, Shape},
    query::{Problem, geoquery},
};
use pretty_assertions::assert_eq;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{method, path},
};

/// The Point `[8,40]` of clause 4.10 example 1.
fn point() -> Shape {
    Shape::Point(Position::try_new(8.0, 40.0).unwrap())
}

#[tokio::test]
async fn a_query_sends_the_geoquery_and_the_scope_query_with_their_syntax() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle"}]"#, "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server.uri())
        .entities()
        .query()
        .geo_q(geoquery::near_max(2000, point()))
        .scope_q("/Madrid/+/ParqueNorte")
        .send()
        .await
        .unwrap();
    assert_eq!(page.items().len(), 1);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].url.query(),
        Some("georel=near;maxDistance==2000&geometry=Point&coordinates=%5B8%2C40%5D&scopeQ=/Madrid/%2B/ParqueNorte")
    );
}

#[tokio::test]
async fn an_invalid_distance_or_scope_is_refused_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("[]", "application/json"))
        .expect(0)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let distance = client.entities().query().geo_q(geoquery::near_max(0, point())).send().await.unwrap_err();
    assert!(
        matches!(&distance, ClientError::Request(RequestError::Argument(ArgumentError::InvalidArgument { source }))
            if matches!(source.problems(), [Problem::InvalidDistance { .. }])),
        "{distance:?}"
    );
    let scope = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .scope_q("/Madrid//Gardens")
        .send()
        .await
        .unwrap_err();
    assert!(
        matches!(&scope, ClientError::Request(RequestError::Argument(ArgumentError::InvalidArgument { source }))
            if matches!(source.problems(), [Problem::InvalidScopePattern { .. }])),
        "{scope:?}"
    );
}
