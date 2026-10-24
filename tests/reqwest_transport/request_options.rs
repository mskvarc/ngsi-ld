//! What a request may state besides its operation, over the client: the `Prefer` header (clause 6.3.6), the `local`
//! parameter (clause 6.3.18), and the implicit locality of a query on a snapshot (clause 5.5.15).

use crate::fixtures::{client, vehicle};
use headers::Header;
use http::header;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::headers::{NgsiLdSnapshot, Prefer, SpecVersion},
    query::Locality,
    snapshot::SnapshotId,
};
use pretty_assertions::assert_eq;
use url::Url;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{header, method, path, query_param, query_param_is_missing},
};

#[tokio::test]
async fn the_client_states_its_preference_and_the_locality_of_a_call() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("local", "true"))
        .and(header(Prefer::name(), "ngsi-ld=1.9"))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567"))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::builder()
        .base_url(Url::parse(&format!("{}/ngsi-ld/v1/", server.uri())).unwrap())
        .prefer(Prefer::builder().version(SpecVersion::new(1, 9)).build().unwrap())
        .transport(ReqwestTransport::new(reqwest::Client::builder().build().unwrap()))
        .build()
        .unwrap();
    let entity = vehicle();
    assert!(client.entities().create(&entity).locality(Locality::Local).send().await.is_ok());
}

#[tokio::test]
async fn a_query_on_a_snapshot_needs_no_type_and_sends_no_local() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param_is_missing("local"))
        .and(header(NgsiLdSnapshot::name(), "urn:ngsi-ld:Snapshot:1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let snapshot: SnapshotId = "urn:ngsi-ld:Snapshot:1".parse().unwrap();
    let page = client.on_snapshot(&snapshot).entities().query().send().await.unwrap();
    assert_eq!(page.items().len(), 0);
}
