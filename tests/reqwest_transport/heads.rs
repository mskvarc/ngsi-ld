//! The HEAD request of a retrieval, over the client (ETSI TS 104 176 clause 5).

use crate::fixtures::client;
use headers::Header;
use http::StatusCode;
use ngsi_ld::{
    client::ClientError,
    http::{
        ResponseError,
        headers::NgsiLdResultsCount,
        operation::{Head, QueryEntities, ResultsCount, RetrieveEntity},
    },
    model::{EntityId, Lossless},
    query::SelectorType,
};
use pretty_assertions::assert_eq;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{method, path, query_param},
};

#[tokio::test]
async fn the_head_of_a_counted_query_reads_the_total_from_the_head() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Vehicle"))
        .and(query_param("limit", "0"))
        .and(query_param("count", "true"))
        .respond_with(ResponseTemplate::new(204).insert_header(NgsiLdResultsCount::name(), "42"))
        .expect(1)
        .mount(&server)
        .await;
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let query = QueryEntities::builder()
        .entity_type(&vehicles)
        .limit(0)
        .count(ResultsCount::Include)
        .format(Lossless::Normalized)
        .build();
    let head = client(&server.uri()).send(&Head::new(&query)).await.unwrap();
    assert_eq!(head.typed::<NgsiLdResultsCount>().into_valid().map(NgsiLdResultsCount::count), Some(42));
}

#[tokio::test]
async fn the_head_of_a_missing_entity_is_a_404_without_a_body() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    let id: EntityId = "urn:ngsi-ld:Vehicle:A4567".parse().unwrap();
    let retrieve = RetrieveEntity::builder().id(&id).format(Lossless::Normalized).build();
    let error = client(&server.uri()).send(&Head::new(&retrieve)).await.unwrap_err();
    assert!(
        matches!(
            error,
            ClientError::Response(ResponseError::Status {
                status: StatusCode::NOT_FOUND,
                ..
            })
        ),
        "{error:?}"
    );
}
