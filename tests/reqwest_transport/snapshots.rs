//! The snapshot operations and a client's snapshot view, over the client (clauses 6.3.22 and
//! 6.36 to 6.38).

use crate::{
    fixtures::client,
    links::{context_link, link_field, link_value},
};
use headers::Header;
use http::header;
use iri_rs::iri;
use ngsi_ld::{
    client::ClientError,
    http::{
        BatchOutcome,
        ResponseError,
        UncheckedDocument,
        headers::{NgsiLdSnapshot, RelationType},
        operation::CreateEntity,
    },
    model::{Entity, EntityId, ErrorType, Input, Lossless},
    query::q,
    snapshot::{SettledStatus, Snapshot, SnapshotClone, SnapshotFragment, SnapshotId, SnapshotMember, SnapshotStatus},
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

/// `S`.
const S: &str = "urn:ngsi-ld:Snapshot:1";

/// The path of `S`.
const S_PATH: &str = "/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:1";

/// A snapshot of the vehicles, as JSON.
fn vehicles_json() -> Value {
    json!({"type": "Snapshot", "snapshotQueries": [{"type": "Query", "entities": [{"type": "Vehicle"}]}], "snapshotLifetime": "PT1H"})
}

/// The snapshot `S` as a broker returns it.
fn returned() -> Value {
    json!({
        "id": S,
        "type": "Snapshot",
        "snapshotQueries": [{"type": "Query", "entities": [{"type": "Vehicle"}]}],
        "snapshotLifetime": "PT1H",
        "snapshotStatus": "success",
        "snapshotQueriesDetails": [{"resultStatus": "success"}],
        "expiresAt": "2030-01-01T00:00:00Z"
    })
}

/// Parses the snapshot id `S`.
fn s() -> SnapshotId {
    S.parse().unwrap()
}

#[tokio::test]
async fn every_snapshot_operation_reaches_the_wire_and_reads_its_answer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/snapshots"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(vehicles_json()))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, S_PATH))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(S_PATH))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(returned()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(S_PATH))
        .and(header(&header::ACCEPT, "application/json"))
        .and(body_json(
            json!({"type": "Snapshot", "snapshotLifetime": "PT2H", "endpoint": "urn:ngsi-ld:null"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(returned()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{S_PATH}/clone")))
        .and(body_json(json!({"id": "urn:ngsi-ld:Snapshot:2", "type": "Snapshot", "snapshotPriority": 3})))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, "/ngsi-ld/v1/snapshots/urn:ngsi-ld:Snapshot:2"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(S_PATH))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/ngsi-ld/v1/snapshots"))
        .and(query_param("q", "snapshotPriority<3"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let snapshot: Snapshot<Input> = Snapshot::parse(&vehicles_json().to_string()).unwrap();
    let snapshots = client.snapshots();
    let created = snapshots.create(&snapshot).send().await.unwrap();
    assert_eq!(created.id(), Some(s()));
    let retrieved = snapshots.retrieve(&s()).send().await.unwrap();
    assert_eq!(retrieved.status(), SnapshotStatus::Settled(SettledStatus::Success));
    let longer = SnapshotFragment::builder()
        .lifetime("PT2H".parse().unwrap())
        .delete(SnapshotMember::Endpoint)
        .build()
        .unwrap();
    assert_eq!(snapshots.update(&s(), &longer).send().await.unwrap(), retrieved);
    let clone = SnapshotClone::builder().id(iri!("urn:ngsi-ld:Snapshot:2")).priority(3).build().unwrap();
    let cloned = snapshots.clone_snapshot(&s(), &clone).send().await.unwrap();
    assert_eq!(cloned.id(), Some("urn:ngsi-ld:Snapshot:2".parse().unwrap()));
    snapshots.delete(&s()).send().await.unwrap();
    assert_eq!(snapshots.purge(q::attr("snapshotPriority").lt(3)).send().await.unwrap(), BatchOutcome::Done);
}

#[tokio::test]
async fn the_snapshot_view_aims_every_call_and_every_page_at_the_snapshot() {
    let server = MockServer::start().await;
    let vehicle = |id: &str| json!({"id": id, "type": "Vehicle", "speed": 80});
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("offset", "1"))
        .and(header(NgsiLdSnapshot::name(), S))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([vehicle("urn:ngsi-ld:Vehicle:B9211")])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("limit", "1"))
        .and(header(NgsiLdSnapshot::name(), S))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(&header::LINK, &link_field([link_value("?type=Vehicle&limit=1&offset=1", RelationType::Next)]))
                .set_body_json(json!([vehicle("urn:ngsi-ld:Vehicle:A4567")])),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:A4567"))
        .and(header(NgsiLdSnapshot::name(), S))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle"})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let snapshot = s();
    let view = client.on_snapshot(&snapshot);
    let first = view.entities().query_simplified().entity_type("Vehicle").limit(1).send().await.unwrap();
    assert_eq!(first.snapshot(), Some(&snapshot));
    let every = client.pages(first).try_collect().await.unwrap();
    let ids: Vec<&str> = every.iter().map(|vehicle| vehicle.id.as_str()).collect();
    assert_eq!(ids, ["urn:ngsi-ld:Vehicle:A4567", "urn:ngsi-ld:Vehicle:B9211"]);
    let evolution = view
        .temporal()
        .retrieve(&EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")))
        .send()
        .await
        .unwrap();
    assert_eq!(evolution.body().id.as_str(), "urn:ngsi-ld:Vehicle:A4567");
}

#[tokio::test]
async fn an_unchecked_request_of_the_snapshot_view_carries_the_snapshot_and_its_rejection_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(header(NgsiLdSnapshot::name(), S))
        .and(body_json(json!({"id": "urn:ngsi-ld:Vehicle:A4567"})))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/BadRequestData", "title": "missing type"})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let snapshot = s();
    let vehicle = Entity::builder().id(iri!("urn:ngsi-ld:Vehicle:A4567")).entity_type("Vehicle").build().unwrap();
    let create = CreateEntity::builder().entity(&vehicle).build();
    let broken = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized).remove_member("type");
    let request = client.on_snapshot(&snapshot).unchecked(&create).unwrap().document(&broken);
    assert_eq!(request.as_request().headers()[NgsiLdSnapshot::name()], S);
    let error = client.send_unchecked(request).await.unwrap_err();
    assert!(
        matches!(&error, ClientError::Response(ResponseError::Problem { details, .. }) if details.error_type == Some(ErrorType::BadRequestData)),
        "{error:?}"
    );
}
